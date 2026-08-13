use super::*;

mod queries;
mod wakes;
#[allow(clippy::too_many_arguments)]
impl HubStore {
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
                kind, status, subject, body, created_at, acked_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)
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
            ],
        )?;
        self.get_message(&id)?.ok_or_else(|| HubError::NotFound(id))
    }

    /// Fan out a team message while retaining one shared subject so clients
    /// can render the broadcast once instead of once per recipient.
    pub fn send_message_to_team(
        &self,
        from_agent: &str,
        kind: MessageKind,
        body: &str,
        subject: Option<&str>,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let subject = subject
            .map(str::to_string)
            .unwrap_or_else(|| format!("team:{}", Uuid::new_v4()));
        let recipients = self
            .list_team_members()?
            .into_iter()
            .filter(|agent| agent.id != from_agent && agent.id != "system")
            .map(|agent| agent.id)
            .collect::<Vec<_>>();
        if recipients.is_empty() {
            return Ok(vec![self.send_message(
                from_agent,
                "team",
                kind,
                body,
                Some(&subject),
                workspace_path,
                task_id,
            )?]);
        }
        recipients
            .into_iter()
            .map(|recipient| {
                self.send_message(
                    from_agent,
                    &recipient,
                    kind,
                    body,
                    Some(&subject),
                    workspace_path,
                    task_id,
                )
            })
            .collect()
    }

    pub fn is_team_member(&self, agent_id: &str) -> Result<bool, HubError> {
        Ok(self
            .conn
            .query_row(
                "SELECT team_member FROM agents WHERE id = ?1",
                params![agent_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .map(|value| value != 0)
            .unwrap_or(false))
    }

    pub fn is_session_member(&self, session_id: &str, agent_id: &str) -> Result<bool, HubError> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM work_session_members WHERE session_id = ?1 AND agent_id = ?2",
                params![session_id, agent_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// C11: enforce distinct task vs. wake semantics per recipient.
    ///
    /// "Currently present" is defined as: enrolled on the standing team
    /// (`agents.team_member`), and — when `session_id` is given — also a
    /// member of that session. There is no live-heartbeat signal in this
    /// schema yet, so presence is this durable enrollment state, not a
    /// point-in-time process check.
    ///
    /// - Task-tagged recipients who are not currently present are rejected:
    ///   no message is sent and no membership is mutated.
    /// - Wake-tagged recipients who are not yet a team member are enrolled
    ///   (and added to the session, if any) before delivery, then a durable
    ///   wake request is filed through the existing policy/budget/human-gate
    ///   path (`request_wake`) — a denial there does not undo the enrollment
    ///   or the message send, it only leaves the recipient unwoken.
    /// - Every recipient gets exactly one durable `tagged_send_outcomes` row,
    ///   whether accepted or rejected.
    #[allow(clippy::too_many_arguments)]
    pub fn send_tagged_message(
        &self,
        from_agent: &str,
        to: &[String],
        is_task: bool,
        is_wake: bool,
        body: &str,
        subject: Option<&str>,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<Vec<SendOutcome>, HubError> {
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        if !is_task && !is_wake {
            return Err(HubError::Invalid(
                "send_tagged_message requires at least one of task/wake".into(),
            ));
        }
        let subject = subject
            .map(str::to_string)
            .unwrap_or_else(|| format!("tagged:{}", Uuid::new_v4()));

        let mut recipients: Vec<String> = Vec::new();
        for id in to {
            if id != "system" && id != from_agent && !recipients.contains(id) {
                recipients.push(id.clone());
            }
        }
        if recipients.is_empty() {
            return Err(HubError::Invalid(
                "send_tagged_message requires at least one recipient".into(),
            ));
        }
        self.record_recipient_set(&subject, session_id, &recipients)?;

        let mut outcomes = Vec::with_capacity(recipients.len());
        for recipient in recipients {
            let present = if let Some(session_id) = session_id {
                self.is_team_member(&recipient)?
                    && self.is_session_member(session_id, &recipient)?
            } else {
                self.is_team_member(&recipient)?
            };

            if is_task && !present {
                outcomes.push(self.record_send_outcome(
                    &subject,
                    from_agent,
                    &recipient,
                    is_task,
                    is_wake,
                    false,
                    false,
                    false,
                    Some("task target is not a current team/session member".into()),
                    None,
                )?);
                continue;
            }

            let mut enrolled = false;
            if is_wake && !self.is_team_member(&recipient)? {
                self.upsert_agent(&recipient, &recipient)?;
                self.set_team_member(&recipient, true)?;
                if let Some(session_id) = session_id {
                    self.add_work_session_member(session_id, &recipient)?;
                }
                enrolled = true;
            }

            let kind = if is_wake {
                MessageKind::Wake
            } else {
                MessageKind::Message
            };
            let message = self.send_message(
                from_agent,
                &recipient,
                kind,
                body,
                Some(&subject),
                workspace_path,
                task_id,
            )?;

            let mut wake_requested = false;
            let mut reason = None;
            if is_wake {
                let wake_reason = format!("tagged send: {subject}");
                match self.request_wake(&recipient, Some(&wake_reason), Some(&message.id), false) {
                    Ok(_) => wake_requested = true,
                    Err(error) => reason = Some(format!("wake request denied: {error}")),
                }
            }

            outcomes.push(self.record_send_outcome(
                &subject,
                from_agent,
                &recipient,
                is_task,
                is_wake,
                true,
                enrolled,
                wake_requested,
                reason,
                Some(message.id),
            )?);
        }

        Ok(outcomes)
    }

    /// C10: send an untagged work-session post to an explicit recipient set.
    /// The set is recorded once by subject, rather than reconstructed later
    /// from fan-out rows.
    #[allow(clippy::too_many_arguments)]
    pub fn send_session_message(
        &self,
        from_agent: &str,
        session_id: &str,
        to: &[String],
        body: &str,
        subject: Option<&str>,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<MessageRecord>, HubError> {
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        self.get_work_session(session_id)?;
        let mut recipients = Vec::new();
        for id in to {
            if id != "system" && id != from_agent && !recipients.contains(id) {
                if !self.is_session_member(session_id, id)? {
                    return Err(HubError::Invalid(format!(
                        "recipient {id} is not a member of work session {session_id}"
                    )));
                }
                recipients.push(id.clone());
            }
        }
        if recipients.is_empty() {
            return Err(HubError::Invalid(
                "session message requires at least one recipient".into(),
            ));
        }
        let subject = subject
            .map(str::to_owned)
            .unwrap_or_else(|| format!("channel:session:{session_id}:{}", Uuid::new_v4()));
        self.record_recipient_set(&subject, Some(session_id), &recipients)?;
        recipients
            .iter()
            .map(|recipient| {
                self.send_message(
                    from_agent,
                    recipient,
                    MessageKind::Message,
                    body,
                    Some(&subject),
                    workspace_path,
                    task_id,
                )
            })
            .collect()
    }

    fn record_recipient_set(
        &self,
        subject: &str,
        session_id: Option<&str>,
        recipients: &[String],
    ) -> Result<(), HubError> {
        let recipient_ids_json = serde_json::to_string(recipients)
            .map_err(|error| HubError::Invalid(format!("recipient set serialize: {error}")))?;
        self.conn.execute(
            "INSERT INTO message_recipient_sets(subject, session_id, recipient_ids_json, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![subject, session_id, recipient_ids_json, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn record_send_outcome(
        &self,
        subject: &str,
        from_agent: &str,
        to_agent: &str,
        is_task: bool,
        is_wake: bool,
        accepted: bool,
        enrolled: bool,
        wake_requested: bool,
        reason: Option<String>,
        message_id: Option<String>,
    ) -> Result<SendOutcome, HubError> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO tagged_send_outcomes(
                id, subject, from_agent, to_agent, is_task, is_wake,
                accepted, enrolled, wake_requested, reason, message_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                id,
                subject,
                from_agent,
                to_agent,
                is_task as i64,
                is_wake as i64,
                accepted as i64,
                enrolled as i64,
                wake_requested as i64,
                reason,
                message_id,
                created_at,
            ],
        )?;
        Ok(SendOutcome {
            id,
            subject: subject.to_string(),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.to_string(),
            is_task,
            is_wake,
            accepted,
            enrolled,
            wake_requested,
            reason,
            message_id,
            created_at,
        })
    }

    pub fn list_tagged_send_outcomes(&self, subject: &str) -> Result<Vec<SendOutcome>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, subject, from_agent, to_agent, is_task, is_wake,
                   accepted, enrolled, wake_requested, reason, message_id, created_at
            FROM tagged_send_outcomes
            WHERE subject = ?1
            ORDER BY created_at
            "#,
        )?;
        let rows = stmt.query_map(params![subject], |row| {
            Ok(SendOutcome {
                id: row.get(0)?,
                subject: row.get(1)?,
                from_agent: row.get(2)?,
                to_agent: row.get(3)?,
                is_task: row.get::<_, i64>(4)? != 0,
                is_wake: row.get::<_, i64>(5)? != 0,
                accepted: row.get::<_, i64>(6)? != 0,
                enrolled: row.get::<_, i64>(7)? != 0,
                wake_requested: row.get::<_, i64>(8)? != 0,
                reason: row.get(9)?,
                message_id: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
