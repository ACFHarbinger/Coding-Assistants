//! The daily ungated-send quota, the broadcast-recipient limit, and the
//! durable human-approval queue those two limits route a send into when
//! exceeded. [`HubStore::send_tagged_message_gated`] is the entry point
//! callers (the `ca` CLI, Tauri commands) should use instead of the raw
//! [`HubStore::send_tagged_message`] so role limits are actually enforced;
//! the raw function is unchanged and still used directly by this module
//! once a send is confirmed allowed (or explicitly approved).

use super::super::*;

fn today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn row_to_pending(row: &rusqlite::Row) -> rusqlite::Result<PendingGateApproval> {
    let to_agents_json: String = row.get(3)?;
    Ok(PendingGateApproval {
        id: row.get(0)?,
        subject: row.get(1)?,
        from_agent: row.get(2)?,
        to_agents: serde_json::from_str(&to_agents_json).unwrap_or_default(),
        is_task: row.get::<_, i64>(4)? != 0,
        is_wake: row.get::<_, i64>(5)? != 0,
        body: row.get(6)?,
        workspace_path: row.get(7)?,
        task_id: row.get(8)?,
        session_id: row.get(9)?,
        reason: row.get(10)?,
        status: row.get(11)?,
        created_at: row.get(12)?,
        resolved_at: row.get(13)?,
    })
}

const PENDING_COLUMNS: &str = "id, subject, from_agent, to_agents_json, is_task, is_wake, body, \
     workspace_path, task_id, session_id, reason, status, created_at, resolved_at";

impl HubStore {
    /// How many ungated task/wake sends `agent_id` has already made today
    /// (UTC calendar day) — the counter [`Self::check_broadcast_gate`]
    /// compares against a role's `daily_ungated_quota`.
    pub fn gate_quota_used_today(&self, agent_id: &str) -> Result<i64, HubError> {
        self.conn
            .query_row(
                "SELECT ungated_sends_used FROM gate_quota_usage WHERE agent_id = ?1 AND usage_date = ?2",
                params![agent_id, today()],
                |r| r.get(0),
            )
            .optional()
            .map(|v| v.unwrap_or(0))
            .map_err(HubError::from)
    }

    fn record_quota_usage(&self, agent_id: &str) -> Result<(), HubError> {
        self.conn.execute(
            "INSERT INTO gate_quota_usage(agent_id, usage_date, ungated_sends_used) VALUES (?1, ?2, 1)
             ON CONFLICT(agent_id, usage_date) DO UPDATE SET ungated_sends_used = ungated_sends_used + 1",
            params![agent_id, today()],
        )?;
        Ok(())
    }

    /// Whether a task/wake send from `from_agent` targeting
    /// `recipient_count` agents may proceed immediately, based on
    /// `from_agent`'s effective role permissions. Does **not** consume the
    /// daily quota itself — call [`Self::send_tagged_message_gated`],
    /// which does so only for a send it actually allows through.
    pub fn check_broadcast_gate(
        &self,
        from_agent: &str,
        workspace_path: Option<&str>,
        recipient_count: usize,
    ) -> Result<GateVerdict, HubError> {
        let effective = self.effective_agent_permissions(from_agent, workspace_path)?;

        if let Some(max) = effective.max_broadcast_recipients {
            if recipient_count as i64 > max {
                return Ok(GateVerdict::RequiresApproval {
                    reason: format!(
                        "broadcast targets {recipient_count} agents, exceeding {from_agent}'s role limit of {max}"
                    ),
                });
            }
        }

        if let Some(quota) = effective.daily_ungated_quota {
            let used = self.gate_quota_used_today(from_agent)?;
            if used >= quota {
                return Ok(GateVerdict::RequiresApproval {
                    reason: format!(
                        "{from_agent}'s daily ungated task/wake quota ({quota}) is exhausted for today"
                    ),
                });
            }
        }

        Ok(GateVerdict::Allowed)
    }

    /// The gate-aware entry point for a task/wake send: checks
    /// [`Self::check_broadcast_gate`] first. If allowed, consumes one unit
    /// of the daily quota and delegates to the existing, unchanged
    /// [`Self::send_tagged_message`]. If not, durably queues a
    /// [`PendingGateApproval`] instead of sending anything, and returns a
    /// synthetic, unaccepted [`SendOutcome`] per recipient so the caller
    /// sees the same shape it always has.
    #[allow(clippy::too_many_arguments)]
    pub fn send_tagged_message_gated(
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
        let recipients: Vec<String> = to
            .iter()
            .filter(|id| id.as_str() != "system" && id.as_str() != from_agent)
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();

        let verdict = self.check_broadcast_gate(from_agent, workspace_path, recipients.len())?;
        match verdict {
            GateVerdict::Allowed => {
                if self
                    .effective_agent_permissions(from_agent, workspace_path)?
                    .daily_ungated_quota
                    .is_some()
                {
                    self.record_quota_usage(from_agent)?;
                }
                self.send_tagged_message(
                    from_agent,
                    to,
                    is_task,
                    is_wake,
                    body,
                    subject,
                    workspace_path,
                    task_id,
                    session_id,
                )
            }
            GateVerdict::RequiresApproval { reason } => {
                let subject = subject
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("tagged:{}", Uuid::new_v4()));
                self.queue_gated_send(
                    &subject,
                    from_agent,
                    &recipients,
                    is_task,
                    is_wake,
                    body,
                    workspace_path,
                    task_id,
                    session_id,
                    &reason,
                )?;
                let now = Utc::now().to_rfc3339();
                Ok(recipients
                    .iter()
                    .map(|to_agent| SendOutcome {
                        id: Uuid::new_v4().to_string(),
                        subject: subject.clone(),
                        from_agent: from_agent.to_string(),
                        to_agent: to_agent.clone(),
                        is_task,
                        is_wake,
                        accepted: false,
                        enrolled: false,
                        wake_requested: false,
                        reason: Some(reason.clone()),
                        policy_decision: "gate_pending_role_limit".to_string(),
                        message_id: None,
                        created_at: now.clone(),
                    })
                    .collect())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn queue_gated_send(
        &self,
        subject: &str,
        from_agent: &str,
        to_agents: &[String],
        is_task: bool,
        is_wake: bool,
        body: &str,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
        session_id: Option<&str>,
        reason: &str,
    ) -> Result<PendingGateApproval, HubError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let to_agents_json =
            serde_json::to_string(to_agents).map_err(|e| HubError::Invalid(e.to_string()))?;
        self.conn.execute(
            r#"
            INSERT INTO pending_gate_approvals(
                id, subject, from_agent, to_agents_json, is_task, is_wake, body,
                workspace_path, task_id, session_id, reason, status, created_at, resolved_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending', ?12, NULL)
            "#,
            params![
                id,
                subject,
                from_agent,
                to_agents_json,
                is_task as i64,
                is_wake as i64,
                body,
                workspace_path,
                task_id,
                session_id,
                reason,
                now,
            ],
        )?;
        self.get_pending_gate_approval(&id)?
            .ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_pending_gate_approval(
        &self,
        id: &str,
    ) -> Result<Option<PendingGateApproval>, HubError> {
        self.conn
            .query_row(
                &format!("SELECT {PENDING_COLUMNS} FROM pending_gate_approvals WHERE id = ?1"),
                params![id],
                row_to_pending,
            )
            .optional()
            .map_err(HubError::from)
    }

    /// Lists gate approvals, optionally filtered by status (`"pending"`,
    /// `"approved"`, or `"rejected"`); most recent first.
    pub fn list_pending_gate_approvals(
        &self,
        status: Option<&str>,
    ) -> Result<Vec<PendingGateApproval>, HubError> {
        let sql = match status {
            Some(_) => format!(
                "SELECT {PENDING_COLUMNS} FROM pending_gate_approvals WHERE status = ?1 ORDER BY created_at DESC"
            ),
            None => format!("SELECT {PENDING_COLUMNS} FROM pending_gate_approvals ORDER BY created_at DESC"),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = match status {
            Some(s) => stmt.query_map(params![s], row_to_pending)?,
            None => stmt.query_map([], row_to_pending)?,
        };
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// A human resolves a pending gate approval. Approving delivers the
    /// original send for real (via [`Self::send_tagged_message`]).
    /// Rejecting never sends anything — it durably marks the request
    /// rejected and sends `from_agent` an automated notification message
    /// explaining why, so the agent isn't left silently wondering what
    /// happened to its send.
    pub fn resolve_gate_approval(
        &self,
        id: &str,
        approve: bool,
    ) -> Result<PendingGateApproval, HubError> {
        let pending = self
            .get_pending_gate_approval(id)?
            .ok_or_else(|| HubError::NotFound(id.to_string()))?;
        if pending.status != "pending" {
            return Err(HubError::Invalid(format!(
                "gate approval {id} was already {}",
                pending.status
            )));
        }
        let now = Utc::now().to_rfc3339();
        let status = if approve { "approved" } else { "rejected" };
        self.conn.execute(
            "UPDATE pending_gate_approvals SET status = ?1, resolved_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;

        if approve {
            self.send_tagged_message(
                &pending.from_agent,
                &pending.to_agents,
                pending.is_task,
                pending.is_wake,
                &pending.body,
                Some(&pending.subject),
                pending.workspace_path.as_deref(),
                pending.task_id.as_deref(),
                pending.session_id.as_deref(),
            )?;
        } else {
            let notice = format!(
                "Your {} send (subject \"{}\") to {} was rejected by the human approval gate: {}",
                if pending.is_task && pending.is_wake {
                    "task+wake"
                } else if pending.is_task {
                    "task"
                } else {
                    "wake"
                },
                pending.subject,
                pending.to_agents.join(", "),
                pending.reason,
            );
            self.send_message(
                "system",
                &pending.from_agent,
                MessageKind::Message,
                &notice,
                None,
                pending.workspace_path.as_deref(),
                None,
            )?;
        }

        self.get_pending_gate_approval(id)?
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }
}

#[cfg(test)]
#[path = "gate/tests.rs"]
mod tests;
