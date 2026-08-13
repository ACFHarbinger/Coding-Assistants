use super::super::*;
impl HubStore {
    pub fn request_wake(
        &self,
        target_agent: &str,
        reason: Option<&str>,
        message_id: Option<&str>,
        requires_human_gate: bool,
    ) -> Result<WakeRecord, HubError> {
        self.upsert_agent(target_agent, target_agent)?;

        if let Some(budget) = self.get_budget(target_agent)? {
            if budget.paused {
                return Err(HubError::Invalid(format!(
                    "{target_agent} is budget-paused ({}/{} units spent); \
                     resume_agent() required before new wakes are allowed",
                    budget.spent_units, budget.limit_units
                )));
            }
        }

        let policy = self.get_wake_policy()?;
        let mut requires_human_gate = requires_human_gate;
        if policy.default_requires_human_gate {
            requires_human_gate = true;
        }
        if !requires_human_gate && !policy.allow_auto_wake {
            return Err(HubError::Invalid(
                "wake policy forbids auto-wake without human gate".into(),
            ));
        }

        // A pending wake is an edge-triggered signal. Repeating the same
        // request must not create duplicate durable rows or side-channel files.
        let existing = self
            .conn
            .query_row(
                r#"
                SELECT id, target_agent, message_id, reason, status,
                       requires_human_gate, created_at
                FROM wake_requests
                WHERE target_agent = ?1
                  AND status = 'pending'
                  AND message_id IS ?2
                  AND reason IS ?3
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                params![target_agent, message_id, reason],
                |r| {
                    Ok(WakeRecord {
                        id: r.get(0)?,
                        target_agent: r.get(1)?,
                        message_id: r.get(2)?,
                        reason: r.get(3)?,
                        status: r.get(4)?,
                        requires_human_gate: r.get::<_, i64>(5)? != 0,
                        created_at: r.get(6)?,
                    })
                },
            )
            .optional()?;
        if let Some(wake) = existing {
            return Ok(wake);
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO wake_requests(
                id, target_agent, message_id, reason, status,
                requires_human_gate, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                id,
                target_agent,
                message_id,
                reason,
                WakeStatus::Pending.as_str(),
                if requires_human_gate { 1 } else { 0 },
                now,
            ],
        )?;

        // Ephemeral wake side-channel: drop a file agents/file-watchers can observe.
        let wake_path = self.data_dir.join("wake").join(format!("{id}.json"));
        let payload = serde_json::json!({
            "id": id,
            "target_agent": target_agent,
            "message_id": message_id,
            "reason": reason,
            "requires_human_gate": requires_human_gate,
            "created_at": now,
            "status": "pending"
        });
        fs::write(wake_path, serde_json::to_string_pretty(&payload).unwrap())?;

        Ok(WakeRecord {
            id,
            target_agent: target_agent.into(),
            message_id: message_id.map(|s| s.into()),
            reason: reason.map(|s| s.into()),
            status: WakeStatus::Pending.as_str().into(),
            requires_human_gate,
            created_at: now,
        })
    }

    pub fn set_wake_status(&self, id: &str, status: WakeStatus) -> Result<(), HubError> {
        let n = self.conn.execute(
            "UPDATE wake_requests SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        // Keep side-channel file in sync when present.
        let path = self.data_dir.join("wake").join(format!("{id}.json"));
        if path.exists() {
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) {
                    v["status"] = serde_json::json!(status.as_str());
                    let _ = fs::write(&path, serde_json::to_string_pretty(&v).unwrap_or_default());
                }
            }
            if status != WakeStatus::Pending {
                let _ = fs::remove_file(&path);
            }
        }
        Ok(())
    }

    pub fn list_wakes(
        &self,
        target_agent: Option<&str>,
        pending_only: bool,
    ) -> Result<Vec<WakeRecord>, HubError> {
        let mut sql = String::from(
            r#"
            SELECT id, target_agent, message_id, reason, status, requires_human_gate, created_at
            FROM wake_requests WHERE 1=1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(t) = target_agent {
            sql.push_str(" AND target_agent = ?");
            params_vec.push(Box::new(t.to_string()));
        }
        if pending_only {
            sql.push_str(" AND status = 'pending'");
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 100");

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(WakeRecord {
                id: r.get(0)?,
                target_agent: r.get(1)?,
                message_id: r.get(2)?,
                reason: r.get(3)?,
                status: r.get(4)?,
                requires_human_gate: r.get::<_, i64>(5)? != 0,
                created_at: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Append to a private journal file (never written into shared SQLite tables).
    pub fn append_private_journal(&self, agent_id: &str, entry: &str) -> Result<PathBuf, HubError> {
        if entry.trim().is_empty() {
            return Err(HubError::Invalid("journal entry must not be empty".into()));
        }
        let dir = self.data_dir.join("journals").join(agent_id);
        fs::create_dir_all(&dir)?;
        let path = dir.join("journal.md");
        let stamp = Utc::now().to_rfc3339();
        let block = format!("\n## {stamp}\n\n{entry}\n");
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        f.write_all(block.as_bytes())?;
        Ok(path)
    }

    /// Permanently delete memories already marked stale (M5 retention).
    pub fn purge_stale_memories(&self) -> Result<usize, HubError> {
        let n = self
            .conn
            .execute("DELETE FROM memories WHERE stale = 1", [])?;
        Ok(n)
    }

    /// Mark short-term memories older than `max_age_hours` as stale (soft retention).
    pub fn mark_short_term_stale_older_than(&self, max_age_hours: i64) -> Result<usize, HubError> {
        if max_age_hours < 0 {
            return Err(HubError::Invalid("max_age_hours must be >= 0".into()));
        }
        let cutoff = (Utc::now() - chrono::Duration::hours(max_age_hours)).to_rfc3339();
        let n = self.conn.execute(
            r#"
            UPDATE memories
            SET stale = 1, updated_at = ?1
            WHERE tier = 'short_term' AND stale = 0 AND created_at < ?2
            "#,
            params![Utc::now().to_rfc3339(), cutoff],
        )?;
        Ok(n)
    }

    pub fn set_message_status(
        &self,
        id: &str,
        status: MessageStatus,
    ) -> Result<MessageRecord, HubError> {
        let acked = if matches!(status, MessageStatus::Acked | MessageStatus::Done) {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };
        let n = self.conn.execute(
            "UPDATE messages SET status = ?1, acked_at = COALESCE(?2, acked_at) WHERE id = ?3",
            params![status.as_str(), acked, id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        self.get_message(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    pub fn update_message_body(&self, id: &str, body: &str) -> Result<MessageRecord, HubError> {
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        let n = self.conn.execute(
            "UPDATE messages SET body = ?1 WHERE id = ?2",
            params![body, id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        self.get_message(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    pub fn delete_message(&self, id: &str) -> Result<(), HubError> {
        let n = self.conn.execute(
            "UPDATE messages SET status = ?1 WHERE id = ?2",
            params![MessageStatus::Cancelled.as_str(), id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        Ok(())
    }

    /// Finds every row sharing `message_id`'s broadcast group: an exact
    /// `subject` match when it carries a `:<uuid>` suffix (CA-107 team/channel
    /// fan-out, one row per recipient), otherwise the legacy grouping by
    /// `(from_agent, body, subject, created-at-to-the-second)` that the
    /// desktop chat also uses to collapse duplicate renders.
    fn broadcast_group_ids(&self, message_id: &str) -> Result<Vec<String>, HubError> {
        let anchor = self
            .get_message(message_id)?
            .ok_or_else(|| HubError::NotFound(message_id.into()))?;

        let has_uuid_suffix = anchor
            .subject
            .as_deref()
            .is_some_and(|subject| subject.matches(':').count() >= 2);

        if has_uuid_suffix {
            let subject = anchor.subject.as_deref().expect("checked above");
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM messages WHERE subject = ?1")?;
            let ids = stmt
                .query_map(params![subject], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(ids);
        }

        let created_second = anchor.created_at.get(..19).unwrap_or(&anchor.created_at);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id FROM messages
            WHERE from_agent = ?1 AND body = ?2
              AND subject IS ?3
              AND substr(created_at, 1, 19) = ?4
            "#,
        )?;
        let ids = stmt
            .query_map(
                params![
                    anchor.from_agent,
                    anchor.body,
                    anchor.subject,
                    created_second
                ],
                |r| r.get::<_, String>(0),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    /// Edit every copy of a team/channel broadcast (CA-106). `message_id` may
    /// be any one row from the group; all sibling copies are updated too.
    pub fn update_broadcast(
        &self,
        message_id: &str,
        body: &str,
    ) -> Result<Vec<MessageRecord>, HubError> {
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        let ids = self.broadcast_group_ids(message_id)?;
        ids.iter()
            .map(|id| self.update_message_body(id, body))
            .collect()
    }

    /// Delete (cancel) every copy of a team/channel broadcast (CA-106).
    /// Returns the number of rows affected.
    pub fn delete_broadcast(&self, message_id: &str) -> Result<usize, HubError> {
        let ids = self.broadcast_group_ids(message_id)?;
        for id in &ids {
            self.delete_message(id)?;
        }
        Ok(ids.len())
    }
}
