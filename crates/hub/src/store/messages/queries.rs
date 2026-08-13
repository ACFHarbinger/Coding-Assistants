use super::super::*;
impl HubStore {
    /// A recipient set represents one fan-out, while a channel/session prefix
    /// represents a conversation. Callers may intentionally reuse that prefix
    /// for subsequent posts, so retain it but give the later fan-out its own
    /// suffix before inserting the primary-keyed recipient-set row.
    pub(super) fn unique_recipient_subject(&self, subject: &str) -> Result<String, HubError> {
        let exists = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM message_recipient_sets WHERE subject = ?1)",
            params![subject],
            |row| row.get::<_, i64>(0),
        )? != 0;
        Ok(if exists {
            format!("{subject}:{}", Uuid::new_v4())
        } else {
            subject.to_owned()
        })
    }

    pub fn get_message(&self, id: &str) -> Result<Option<MessageRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, from_agent, to_agent, workspace_path, task_id, kind, status,
                   subject, body, created_at, acked_at
            FROM messages WHERE id = ?1
            "#,
        )?;
        let row = stmt
            .query_row(params![id], |r| {
                Ok(MessageRecord {
                    id: r.get(0)?,
                    from_agent: r.get(1)?,
                    to_agent: r.get(2)?,
                    workspace_path: r.get(3)?,
                    task_id: r.get(4)?,
                    kind: r.get(5)?,
                    status: r.get(6)?,
                    subject: r.get(7)?,
                    body: r.get(8)?,
                    created_at: r.get(9)?,
                    acked_at: r.get(10)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    pub fn list_messages(
        &self,
        to_agent: Option<&str>,
        status: Option<MessageStatus>,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let mut sql = String::from(
            r#"
            SELECT id, from_agent, to_agent, workspace_path, task_id, kind, status,
                   subject, body, created_at, acked_at
            FROM messages WHERE 1=1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(to) = to_agent {
            sql.push_str(" AND to_agent = ?");
            params_vec.push(Box::new(to.to_string()));
        }
        if let Some(st) = status {
            sql.push_str(" AND status = ?");
            params_vec.push(Box::new(st.as_str().to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 200");

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(MessageRecord {
                id: r.get(0)?,
                from_agent: r.get(1)?,
                to_agent: r.get(2)?,
                workspace_path: r.get(3)?,
                task_id: r.get(4)?,
                kind: r.get(5)?,
                status: r.get(6)?,
                subject: r.get(7)?,
                body: r.get(8)?,
                created_at: r.get(9)?,
                acked_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Lists one Messager-like channel without exposing similarly named channels.
    /// In addition to the canonical `channel:<name>` subject, a colon-delimited
    /// suffix is accepted for future thread/topic metadata.
    pub fn list_channel_messages(
        &self,
        channel: &str,
        limit: usize,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let channel = channel
            .trim()
            .strip_prefix("channel:")
            .unwrap_or(channel.trim());
        if channel.is_empty() {
            return Err(HubError::Invalid("channel must not be empty".into()));
        }
        let subject = format!("channel:{channel}");
        let subject_prefix = format!("{subject}:%");
        let limit = limit.clamp(1, 200) as i64;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, from_agent, to_agent, workspace_path, task_id, kind, status,
                   subject, body, created_at, acked_at
            FROM messages
            WHERE subject = ?1 OR subject LIKE ?2
            ORDER BY created_at DESC
            LIMIT ?3
            "#,
        )?;
        let rows = stmt.query_map(params![subject, subject_prefix, limit], |r| {
            Ok(MessageRecord {
                id: r.get(0)?,
                from_agent: r.get(1)?,
                to_agent: r.get(2)?,
                workspace_path: r.get(3)?,
                task_id: r.get(4)?,
                kind: r.get(5)?,
                status: r.get(6)?,
                subject: r.get(7)?,
                body: r.get(8)?,
                created_at: r.get(9)?,
                acked_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Resolves the unique shared memories referenced by one message body.
    /// Unknown or ambiguous short prefixes are omitted; callers can still use
    /// `parse_memory_references` to present an unresolved reference to users.
    pub fn list_message_memories(&self, message_id: &str) -> Result<Vec<MemoryRecord>, HubError> {
        let message = self
            .get_message(message_id)?
            .ok_or_else(|| HubError::NotFound(message_id.to_string()))?;
        let mut resolved = Vec::new();
        for reference in parse_memory_references(&message.body) {
            let exact = self.get_memory(&reference)?;
            if let Some(memory) = exact {
                resolved.push(memory);
                continue;
            }
            let mut stmt = self.conn.prepare(
                r#"
                SELECT id, scope, workspace_path, tier, agent_id, title, body,
                       tags_json, created_at, updated_at, stale, source_event_id
                FROM memories WHERE id LIKE ?1 ORDER BY id ASC LIMIT 2
                "#,
            )?;
            let matches = stmt
                .query_map(params![format!("{reference}%")], |r| {
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
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            if matches.len() == 1 {
                resolved.push(matches.into_iter().next().expect("one memory match"));
            }
        }
        Ok(resolved)
    }

    /// Acks exactly one message by id, returning the updated record (or
    /// `None` if it no longer exists). Used where a caller needs to
    /// selectively ack a subset of a recipient's pending messages rather
    /// than the all-or-nothing sweep [`poll_messages`] performs.
    pub fn ack_message(&self, id: &str) -> Result<Option<MessageRecord>, HubError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE messages SET status = ?1, acked_at = ?2 WHERE id = ?3",
            params![MessageStatus::Acked.as_str(), now, id],
        )?;
        self.get_message(id)
    }

    pub fn poll_messages(
        &self,
        to_agent: &str,
        mark_acked: bool,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let pending = self.list_messages(Some(to_agent), Some(MessageStatus::Pending))?;
        if mark_acked {
            let now = Utc::now().to_rfc3339();
            for m in &pending {
                self.conn.execute(
                    "UPDATE messages SET status = ?1, acked_at = ?2 WHERE id = ?3",
                    params![MessageStatus::Acked.as_str(), now, m.id],
                )?;
            }
        }
        if mark_acked {
            // re-fetch with acked status for returned records
            let mut out = Vec::with_capacity(pending.len());
            for m in pending {
                if let Some(updated) = self.get_message(&m.id)? {
                    out.push(updated);
                }
            }
            Ok(out)
        } else {
            Ok(pending)
        }
    }
}
