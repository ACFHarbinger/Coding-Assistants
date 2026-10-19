//! Atomic claim + outbox delivery for scheduled dispatch (#351).
//!
//! Persist the attempt and outbox row first, then send. Delivery is
//! idempotent on `(schedule_id, attempt)` so a crash between send and
//! `delivered_at` cannot consume a second attempt.

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
        let mut outcomes = Vec::new();
        for row in rows {
            let item = row?;
            if item.delivered_at.is_some() {
                continue;
            }
            self.deliver_outbox_row(&item, now)?;
            outcomes.push(TickOutcome {
                id: item.schedule_id,
                action: "dispatched".into(),
                detail: format!("outbox attempt {}", item.attempt),
            });
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
            self.deliver_outbox_row(&row, now)?;
        }
        Ok(())
    }

    fn deliver_outbox_row(&self, row: &OutboxRow, now: &str) -> Result<(), HubError> {
        if row.delivered_at.is_some() {
            return Ok(());
        }
        let Some(task) = self.get_scheduled_task(&row.schedule_id)? else {
            self.mark_outbox_delivered(&row.id, now)?;
            return Ok(());
        };
        if !task.status.is_terminal() {
            let subject = schedule_attempt_subject(&task.name, row.attempt);
            if !self.schedule_attempt_already_sent(&task.id, &subject)? {
                self.send_schedule_prompt(&task, &row.body, &subject)?;
            }
        }
        self.mark_outbox_delivered(&row.id, now)
    }

    fn mark_outbox_delivered(&self, id: &str, now: &str) -> Result<(), HubError> {
        self.conn.execute(
            "UPDATE scheduled_dispatch_outbox SET delivered_at = ?1
             WHERE id = ?2 AND delivered_at IS NULL",
            params![now, id],
        )?;
        Ok(())
    }

    fn schedule_attempt_already_sent(
        &self,
        schedule_id: &str,
        subject: &str,
    ) -> Result<bool, HubError> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE task_id = ?1 AND subject = ?2)",
            params![schedule_id, subject],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    fn send_schedule_prompt(
        &self,
        task: &ScheduledTask,
        body: &str,
        subject: &str,
    ) -> Result<(), HubError> {
        let messages = match &task.target {
            ScheduleTarget::Agent { agent_id } => vec![self.send_message(
                "system",
                agent_id,
                MessageKind::Handoff,
                body,
                Some(subject),
                Some(&task.workspace),
                Some(&task.id),
            )?],
            ScheduleTarget::Team => self.send_message_to_team(
                "system",
                MessageKind::Handoff,
                body,
                Some(subject),
                Some(&task.workspace),
                Some(&task.id),
            )?,
        };
        for msg in &messages {
            if msg.to_agent == "team" {
                continue;
            }
            let _ = self.request_wake(
                &msg.to_agent,
                Some(&format!("schedule {}", task.id)),
                Some(&msg.id),
                true,
            );
        }
        let _ = self.record_audit_event(
            Path::new(&task.workspace),
            Path::new(&task.workspace),
            "schedule.fired",
            &serde_json::json!({
                "schedule_id": task.id,
                "recipients": messages.iter().map(|m| &m.to_agent).collect::<Vec<_>>(),
            })
            .to_string(),
            None,
        );
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
