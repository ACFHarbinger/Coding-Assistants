//! Atomic claim + outbox delivery for scheduled dispatch (#351).
//!
//! Persist the attempt and outbox row first, then send. Each recipient is
//! claimed with `INSERT OR IGNORE` in the same transaction as the message
//! insert, so concurrent drainers and partial team fan-out cannot duplicate
//! or skip a recipient. Recovery re-checks deadline/pause before send.

use super::super::*;
use super::TickOutcome;

struct OutboxRow {
    id: String,
    schedule_id: String,
    attempt: u32,
    body: String,
    delivered_at: Option<String>,
}

impl HubStore {
    /// Claim the next attempt if this snapshot still owns it. `None` means
    /// another tick or a cancel won the race.
    pub(crate) fn claim_schedule_dispatch(
        &self,
        task: &ScheduledTask,
        now: &str,
        body: &str,
    ) -> Result<Option<u32>, HubError> {
        self.ensure_mutable()?;
        let tx = self.conn.unchecked_transaction()?;
        let updated = tx.execute(
            "UPDATE scheduled_tasks
             SET attempts = attempts + 1,
                 last_fired_at = ?1,
                 next_fire_at = ?1,
                 status = CASE WHEN status = 'scheduled' THEN 'active' ELSE status END,
                 updated_at = ?1
             WHERE id = ?2
               AND status IN ('scheduled', 'active')
               AND attempts = ?3
               AND attempts < max_attempts",
            params![now, task.id, task.attempts as i64],
        )?;
        if updated == 0 {
            tx.commit()?;
            return Ok(None);
        }
        let attempt = task.attempts + 1;
        tx.execute(
            "INSERT INTO scheduled_dispatch_outbox(
                id, schedule_id, attempt, body, created_at, delivered_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![
                Uuid::new_v4().to_string(),
                task.id,
                attempt as i64,
                body,
                now
            ],
        )?;
        tx.commit()?;
        Ok(Some(attempt))
    }

    pub(crate) fn deliver_pending_schedule_outbox(
        &self,
        now: &str,
    ) -> Result<Vec<TickOutcome>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, schedule_id, attempt, body, delivered_at
             FROM scheduled_dispatch_outbox
             WHERE delivered_at IS NULL
             ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(OutboxRow {
                id: row.get(0)?,
                schedule_id: row.get(1)?,
                attempt: row.get::<_, i64>(2)? as u32,
                body: row.get(3)?,
                delivered_at: row.get(4)?,
            })
        })?;
        let pending = rows.collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        let mut outcomes = Vec::new();
        for item in pending {
            if let Some(outcome) = self.deliver_outbox_row(&item, now, true)? {
                outcomes.push(outcome);
            }
        }
        Ok(outcomes)
    }

    pub(crate) fn deliver_claimed_attempt(
        &self,
        task: &ScheduledTask,
        attempt: u32,
        now: &str,
    ) -> Result<(), HubError> {
        let row = self
            .conn
            .query_row(
                "SELECT id, schedule_id, attempt, body, delivered_at
                 FROM scheduled_dispatch_outbox
                 WHERE schedule_id = ?1 AND attempt = ?2",
                params![task.id, attempt as i64],
                |row| {
                    Ok(OutboxRow {
                        id: row.get(0)?,
                        schedule_id: row.get(1)?,
                        attempt: row.get::<_, i64>(2)? as u32,
                        body: row.get(3)?,
                        delivered_at: row.get(4)?,
                    })
                },
            )
            .optional()?;
        if let Some(row) = row {
            let _ = self.deliver_outbox_row(&row, now, false)?;
        }
        Ok(())
    }

    /// `complete_one_shot` is true for crash-recovery drains so a recovered
    /// one-shot takes the same completion path as `dispatch_prompt`.
    fn deliver_outbox_row(
        &self,
        row: &OutboxRow,
        now: &str,
        complete_one_shot: bool,
    ) -> Result<Option<TickOutcome>, HubError> {
        if row.delivered_at.is_some() {
            return Ok(None);
        }
        let Some(task) = self.get_scheduled_task(&row.schedule_id)? else {
            self.mark_outbox_delivered(&row.id, now)?;
            return Ok(None);
        };
        if let Some(stopped) = self.apply_schedule_stops(&task, now)? {
            self.mark_outbox_delivered(&row.id, now)?;
            return Ok(Some(stopped));
        }
        let sent = self.deliver_live_outbox(&task, row, now)?;
        self.mark_outbox_delivered(&row.id, now)?;
        if complete_one_shot {
            if let Some(completed) = self.complete_recovered_one_shot(&task.id)? {
                return Ok(Some(completed));
            }
        }
        if sent {
            Ok(Some(TickOutcome {
                id: task.id,
                action: "dispatched".into(),
                detail: format!("outbox attempt {}", row.attempt),
            }))
        } else {
            Ok(None)
        }
    }

    fn deliver_live_outbox(
        &self,
        task: &ScheduledTask,
        row: &OutboxRow,
        now: &str,
    ) -> Result<bool, HubError> {
        let recipients = self.schedule_target_agent_ids(&task.target)?;
        let subject = schedule_attempt_subject(&task.name, row.attempt);
        let mut newly = Vec::new();
        for agent_id in &recipients {
            let Some((message, inserted)) =
                self.claim_and_insert_recipient(task, row, agent_id, &subject, now)?
            else {
                continue;
            };
            self.replay_schedule_wake(&message, &task.id)?;
            if inserted {
                let _ = self.audit_schedule_handoff(task, agent_id);
                newly.push(message);
            }
        }
        if !newly.is_empty() {
            let _ = self.record_audit_event(
                Path::new(&task.workspace),
                Path::new(&task.workspace),
                "schedule.fired",
                &serde_json::json!({
                    "schedule_id": task.id,
                    "recipients": newly.iter().map(|m| &m.to_agent).collect::<Vec<_>>(),
                })
                .to_string(),
                None,
            );
        }
        Ok(!newly.is_empty())
    }

    /// Recreate a human-gated wake for a committed message unless one
    /// already exists (pending or finished). Does not mint a second
    /// approval after the first wake completed.
    fn replay_schedule_wake(&self, message: &MessageRecord, task_id: &str) -> Result<(), HubError> {
        // Acquire a writer lock before the existence check, so concurrent
        // drainers cannot create two approvals for the same message.
        let tx = self.conn.unchecked_transaction()?;
        let live = tx.execute(
            "UPDATE scheduled_tasks SET updated_at=updated_at
             WHERE id=?1 AND status IN ('scheduled', 'active')",
            params![task_id],
        )?;
        if live == 0 {
            tx.commit()?;
            return Ok(());
        }
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM wake_requests WHERE message_id = ?1)",
            params![message.id],
            |row| row.get(0),
        )?;
        if exists {
            tx.commit()?;
            return Ok(());
        }
        self.request_wake(
            &message.to_agent,
            Some(&format!("schedule {task_id}")),
            Some(&message.id),
            true,
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Claim one recipient and insert its message in the same transaction.
    /// `None` means the row is no longer scheduled/active (cancel/pause won).
    /// Existing claims return the committed message so recovery can replay
    /// its wake; `inserted` is false in that case.
    fn claim_and_insert_recipient(
        &self,
        task: &ScheduledTask,
        row: &OutboxRow,
        agent_id: &str,
        subject: &str,
        now: &str,
    ) -> Result<Option<(MessageRecord, bool)>, HubError> {
        self.ensure_mutable()?;
        self.upsert_agent("system", "system")?;
        self.upsert_agent(agent_id, agent_id)?;
        let message_id = Uuid::new_v4().to_string();
        let tx = self.conn.unchecked_transaction()?;
        let live = tx.execute(
            "UPDATE scheduled_tasks SET updated_at = updated_at
             WHERE id = ?1 AND status IN ('scheduled', 'active')",
            params![task.id],
        )?;
        if live == 0 {
            tx.commit()?;
            return Ok(None);
        }
        let claimed = tx.execute(
            "INSERT OR IGNORE INTO scheduled_dispatch_recipients(
                schedule_id, attempt, to_agent, claimed_at, message_id
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![task.id, row.attempt as i64, agent_id, now, message_id],
        )?;
        if claimed == 0 {
            let existing = tx
                .query_row(
                    "SELECT message_id FROM scheduled_dispatch_recipients
                     WHERE schedule_id = ?1 AND attempt = ?2 AND to_agent = ?3",
                    params![task.id, row.attempt as i64, agent_id],
                    |r| r.get::<_, Option<String>>(0),
                )
                .optional()?;
            tx.commit()?;
            return match existing.flatten() {
                Some(id) => Ok(self.get_message(&id)?.map(|message| (message, false))),
                None => Ok(None),
            };
        }
        tx.execute(
            "INSERT INTO messages(
                id, from_agent, to_agent, workspace_path, task_id,
                kind, status, subject, body, created_at, acked_at, context_body
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11)",
            params![
                message_id,
                "system",
                agent_id,
                task.workspace,
                task.id,
                MessageKind::Handoff.as_str(),
                MessageStatus::Pending.as_str(),
                subject,
                row.body,
                now,
                task.instruction
            ],
        )?;
        tx.commit()?;
        self.get_message(&message_id)?
            .map(|message| Some((message, true)))
            .ok_or_else(|| HubError::NotFound(message_id))
    }

    fn mark_outbox_delivered(&self, id: &str, now: &str) -> Result<(), HubError> {
        self.conn.execute(
            "UPDATE scheduled_dispatch_outbox SET delivered_at = ?1
             WHERE id = ?2 AND delivered_at IS NULL",
            params![now, id],
        )?;
        Ok(())
    }

    /// Test helper: persist an undelivered outbox row without sending.
    #[cfg(test)]
    pub(crate) fn insert_undelivered_schedule_outbox(
        &self,
        task: &ScheduledTask,
        attempt: u32,
        body: &str,
    ) -> Result<(), HubError> {
        self.conn.execute(
            "INSERT INTO scheduled_dispatch_outbox(
                id, schedule_id, attempt, body, created_at, delivered_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![
                Uuid::new_v4().to_string(),
                task.id,
                attempt as i64,
                body,
                super::time::now_utc_rfc3339()
            ],
        )?;
        Ok(())
    }
}

pub(crate) fn schedule_attempt_subject(name: &str, attempt: u32) -> String {
    format!("[schedule] {name}#{attempt}")
}
