use super::super::*;
impl HubStore {
    #[allow(clippy::too_many_arguments)]
    pub fn write_memory(
        &self,
        tier: MemoryTier,
        scope: MemoryScope,
        agent_id: Option<&str>,
        workspace_path: Option<&str>,
        title: Option<&str>,
        body: &str,
        tags: &[String],
    ) -> Result<MemoryRecord, HubError> {
        self.write_memory_with_source(
            tier,
            scope,
            agent_id,
            workspace_path,
            title,
            body,
            tags,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_memory_with_tool(
        &self,
        tier: MemoryTier,
        scope: MemoryScope,
        agent_id: Option<&str>,
        workspace_path: Option<&str>,
        title: Option<&str>,
        body: &str,
        tags: &[String],
        tool: Option<&str>,
    ) -> Result<MemoryRecord, HubError> {
        let record = self.write_memory(tier, scope, agent_id, workspace_path, title, body, tags)?;
        if let Some(tool) = tool.filter(|tool| !tool.trim().is_empty()) {
            self.conn.execute(
                "UPDATE memories SET tool = ?1 WHERE id = ?2",
                params![tool, record.id],
            )?;
        }
        self.get_memory(&record.id)?
            .ok_or_else(|| HubError::NotFound(record.id))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_memory_with_source(
        &self,
        tier: MemoryTier,
        scope: MemoryScope,
        agent_id: Option<&str>,
        workspace_path: Option<&str>,
        title: Option<&str>,
        body: &str,
        tags: &[String],
        source_event_id: Option<&str>,
    ) -> Result<MemoryRecord, HubError> {
        if scope == MemoryScope::Workspace && workspace_path.is_none() {
            return Err(HubError::Invalid(
                "workspace scope requires --workspace".into(),
            ));
        }
        if body.trim().is_empty() {
            return Err(HubError::Invalid("memory body must not be empty".into()));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".into());

        self.conn.execute(
            r#"
            INSERT INTO memories(
                id, scope, workspace_path, tier, agent_id, title, body,
                tags_json, created_at, updated_at, stale, source_event_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11)
            "#,
            params![
                id,
                scope.as_str(),
                workspace_path,
                tier.as_str(),
                agent_id,
                title,
                body,
                tags_json,
                now,
                now,
                source_event_id,
            ],
        )?;

        let record = self
            .get_memory(&id)?
            .ok_or_else(|| HubError::NotFound(id))?;
        let _ = self.upsert_memory_vector(&record.id, title, body, tags);
        Ok(record)
    }

    pub fn get_memory(&self, id: &str) -> Result<Option<MemoryRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, scope, workspace_path, tier, agent_id, title, body,
                   tags_json, created_at, updated_at, stale, source_event_id, tool
            FROM memories WHERE id = ?1
            "#,
        )?;
        let row = stmt
            .query_row(params![id], |r| {
                Ok(MemoryRecord {
                    id: r.get(0)?,
                    scope: r.get(1)?,
                    workspace_path: r.get(2)?,
                    tier: r.get(3)?,
                    agent_id: r.get(4)?,
                    title: r.get(5)?,
                    body: r.get(6)?,
                    tags_json: r.get(7)?,
                    created_at: r.get(8)?,
                    updated_at: r.get(9)?,
                    stale: r.get::<_, i64>(10)? != 0,
                    source_event_id: r.get(11)?,
                    tool: r.get(12)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    pub fn update_memory(
        &self,
        id: &str,
        title: Option<&str>,
        body: &str,
        tags: Option<&[String]>,
    ) -> Result<MemoryRecord, HubError> {
        let now = Utc::now().to_rfc3339();

        if body.trim().is_empty() {
            return Err(HubError::Invalid("memory body must not be empty".into()));
        }

        if let Some(t) = tags {
            let tags_json = serde_json::to_string(t).unwrap_or_else(|_| "[]".into());
            let updated = self.conn.execute(
                "UPDATE memories SET title = ?1, body = ?2, tags_json = ?3, updated_at = ?4 WHERE id = ?5",
                params![title, body, tags_json, now, id],
            )?;
            if updated == 0 {
                return Err(HubError::NotFound(id.to_string()));
            }
        } else {
            let updated = self.conn.execute(
                "UPDATE memories SET title = ?1, body = ?2, updated_at = ?3 WHERE id = ?4",
                params![title, body, now, id],
            )?;
            if updated == 0 {
                return Err(HubError::NotFound(id.to_string()));
            }
        }

        let record = self
            .get_memory(id)?
            .ok_or_else(|| HubError::NotFound(id.to_string()))?;
        let tags_vec: Vec<String> = serde_json::from_str(&record.tags_json).unwrap_or_default();
        let _ = self.upsert_memory_vector(&record.id, title, body, &tags_vec);
        Ok(record)
    }

    pub fn list_memories(
        &self,
        scope: Option<MemoryScope>,
        tier: Option<MemoryTier>,
        workspace_path: Option<&str>,
        include_stale: bool,
    ) -> Result<Vec<MemoryRecord>, HubError> {
        let mut sql = String::from(
            r#"
            SELECT id, scope, workspace_path, tier, agent_id, title, body,
                   tags_json, created_at, updated_at, stale, source_event_id, tool
            FROM memories WHERE 1=1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if !include_stale {
            sql.push_str(" AND stale = 0");
        }
        if let Some(s) = scope {
            sql.push_str(" AND scope = ?");
            params_vec.push(Box::new(s.as_str().to_string()));
        }
        if let Some(t) = tier {
            sql.push_str(" AND tier = ?");
            params_vec.push(Box::new(t.as_str().to_string()));
        }
        if let Some(ws) = workspace_path {
            sql.push_str(" AND workspace_path = ?");
            params_vec.push(Box::new(ws.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 200");

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(MemoryRecord {
                id: r.get(0)?,
                scope: r.get(1)?,
                workspace_path: r.get(2)?,
                tier: r.get(3)?,
                agent_id: r.get(4)?,
                title: r.get(5)?,
                body: r.get(6)?,
                tags_json: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
                stale: r.get::<_, i64>(10)? != 0,
                source_event_id: r.get(11)?,
                tool: r.get(12)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn search_memories(&self, query: &str) -> Result<Vec<MemoryRecord>, HubError> {
        self.search_memories_impl(query, None)
    }

    /// Exact search with an optional tool scope. `None` preserves the legacy
    /// all-tool result set; `Some` matches only that tool identifier.
    pub fn search_memories_with_tool(
        &self,
        query: &str,
        tool: Option<&str>,
    ) -> Result<Vec<MemoryRecord>, HubError> {
        self.search_memories_impl(query, tool)
    }

    pub(super) fn search_memories_impl(
        &self,
        query: &str,
        tool: Option<&str>,
    ) -> Result<Vec<MemoryRecord>, HubError> {
        let q = format!("%{}%", query.trim());
        if query.trim().is_empty() {
            return Err(HubError::Invalid("search query must not be empty".into()));
        }
        let mut sql = String::from(
            r#"
            SELECT id, scope, workspace_path, tier, agent_id, title, body,
                   tags_json, created_at, updated_at, stale, source_event_id, tool
            FROM memories
            WHERE stale = 0 AND (body LIKE ?1 OR IFNULL(title, '') LIKE ?1 OR tags_json LIKE ?1)
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(q)];
        if let Some(tool) = tool {
            sql.push_str(" AND tool = ?");
            params_vec.push(Box::new(tool.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 100");
        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|param| param.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(MemoryRecord {
                id: r.get(0)?,
                scope: r.get(1)?,
                workspace_path: r.get(2)?,
                tier: r.get(3)?,
                agent_id: r.get(4)?,
                title: r.get(5)?,
                body: r.get(6)?,
                tags_json: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
                stale: r.get::<_, i64>(10)? != 0,
                source_event_id: r.get(11)?,
                tool: r.get(12)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn mark_memory_stale(&self, id: &str, stale: bool) -> Result<(), HubError> {
        let n = self.conn.execute(
            "UPDATE memories SET stale = ?1, updated_at = ?2 WHERE id = ?3",
            params![if stale { 1 } else { 0 }, Utc::now().to_rfc3339(), id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        Ok(())
    }

    pub fn delete_memory(&self, id: &str) -> Result<(), HubError> {
        let n = self
            .conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        Ok(())
    }

    /// Promote a memory to another tier, preserving provenance via `source_event_id`.
    pub fn promote_memory(&self, id: &str, to_tier: MemoryTier) -> Result<MemoryRecord, HubError> {
        let src = self
            .get_memory(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        if src.stale {
            return Err(HubError::Invalid("cannot promote a stale memory".into()));
        }
        let from = MemoryTier::parse(&src.tier)?;
        if from == to_tier {
            return Ok(src);
        }
        // Only allow short_term → episodic → semantic (no demotion).
        let allowed = matches!(
            (from, to_tier),
            (MemoryTier::ShortTerm, MemoryTier::Episodic)
                | (MemoryTier::ShortTerm, MemoryTier::Semantic)
                | (MemoryTier::Episodic, MemoryTier::Semantic)
        );
        if !allowed {
            return Err(HubError::Invalid(format!(
                "cannot promote {} → {}",
                from.as_str(),
                to_tier.as_str()
            )));
        }

        let new_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let title = src
            .title
            .clone()
            .unwrap_or_else(|| format!("promoted from {}", from.as_str()));
        let body = format!(
            "{}\n\n---\n_Promoted from `{}` (`{}`) at {}_\n",
            src.body,
            from.as_str(),
            id,
            now
        );

        self.conn.execute(
            r#"
            INSERT INTO memories(
                id, scope, workspace_path, tier, agent_id, title, body,
                tags_json, created_at, updated_at, stale, source_event_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11)
            "#,
            params![
                new_id,
                src.scope,
                src.workspace_path,
                to_tier.as_str(),
                src.agent_id,
                title,
                body,
                src.tags_json,
                now,
                now,
                id,
            ],
        )?;
        // Mark source stale so short-term lists stay lean; provenance remains queryable.
        self.mark_memory_stale(id, true)?;
        self.get_memory(&new_id)?
            .ok_or_else(|| HubError::NotFound(new_id))
    }

    /// Compact short-term memories: keep the newest `keep_newest`, promote the rest to episodic.
    pub fn compact_short_term(&self, keep_newest: usize) -> Result<CompactReport, HubError> {
        let mut short = self.list_memories(None, Some(MemoryTier::ShortTerm), None, false)?;
        // list is DESC by created_at; keep head, promote tail
        let mut promoted = 0usize;
        let mut skipped = 0usize;
        if short.len() <= keep_newest {
            return Ok(CompactReport {
                examined: short.len(),
                promoted: 0,
                kept: short.len(),
                skipped: 0,
            });
        }
        let to_promote: Vec<MemoryRecord> = short.split_off(keep_newest);
        let kept = short.len();
        for m in &to_promote {
            match self.promote_memory(&m.id, MemoryTier::Episodic) {
                Ok(_) => promoted += 1,
                Err(_) => skipped += 1,
            }
        }
        Ok(CompactReport {
            examined: kept + to_promote.len(),
            promoted,
            kept,
            skipped,
        })
    }
}
