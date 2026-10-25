use super::super::*;

impl HubStore {
    #[allow(clippy::too_many_arguments)]
    pub fn send_message(
        &self,
        from_agent: &str,
        to_agent: &str,
        kind: MessageKind,
        body: &str,
        subject: Option<&str>,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<MessageRecord, HubError> {
        self.send_message_with_context(
            from_agent,
            to_agent,
            kind,
            body,
            subject,
            workspace_path,
            task_id,
            None,
        )
    }

    /// Generated handoffs retain their source content separately from seeded history.
    /// Visible to task and schedule dispatch, which store that source in `context_body`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::store) fn send_message_with_context(
        &self,
        from_agent: &str,
        to_agent: &str,
        kind: MessageKind,
        body: &str,
        subject: Option<&str>,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
        context_body: Option<&str>,
    ) -> Result<MessageRecord, HubError> {
        self.ensure_mutable()?;
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        self.upsert_agent(from_agent, from_agent)?;
        self.upsert_agent(to_agent, to_agent)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO messages(
                id, from_agent, to_agent, workspace_path, task_id,
                kind, status, subject, body, created_at, acked_at, context_body
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11)
            "#,
            params![
                id,
                from_agent,
                to_agent,
                workspace_path,
                task_id,
                kind.as_str(),
                MessageStatus::Pending.as_str(),
                subject,
                body,
                now,
                context_body,
            ],
        )?;
        self.get_message(&id)?.ok_or_else(|| HubError::NotFound(id))
    }
}
