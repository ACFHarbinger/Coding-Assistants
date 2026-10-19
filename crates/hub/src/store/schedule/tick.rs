//! Dispatch/response/re-prompt state machine (`roadmaps/schedule.md` S3).
//!
//! Dispatch reuses `send_message`/`send_message_to_team` + `request_wake`
//! exactly the way `tasks::dispatch_step` already does — governed by the
//! existing `WakePolicy`, no bypass. "Responded" is a `messages` row from
//! the target agent correlated by `task_id = schedule.id`, created after
//! the last dispatch; for a `Team` target the quorum is **All** current
//! roster members. `max_attempts`/`deadline` are checked before anything
//! else on every tick — the only stop conditions that work unconditionally
//! (see `types` module docs on the usage-accounting gap).

use super::super::*;
use chrono::DateTime;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TickOutcome {
    pub id: String,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TickSummary {
    pub outcomes: Vec<TickOutcome>,
}

fn parse(ts: &str) -> Option<DateTime<chrono::Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn seconds_between(earlier: &str, later: &str) -> i64 {
    match (parse(earlier), parse(later)) {
        (Some(a), Some(b)) => (b - a).num_seconds(),
        _ => i64::MAX,
    }
}

fn add_seconds(ts: &str, secs: i64) -> String {
    parse(ts)
        .map(|dt| (dt + chrono::Duration::seconds(secs)).to_rfc3339())
        .unwrap_or_else(|| ts.to_string())
}

impl HubStore {
    /// Process every task due at or before `now` (RFC3339). Pure function
    /// of `now` plus store state, so it is fully testable without a real
    /// clock or sleep.
    pub fn tick_scheduled_tasks(&self, now: &str) -> Result<TickSummary, HubError> {
        let mut outcomes = Vec::new();
        for task in self.due_scheduled_tasks(now)? {
            outcomes.push(self.tick_one(&task, now)?);
        }
        Ok(TickSummary { outcomes })
    }

    fn tick_one(&self, task: &ScheduledTask, now: &str) -> Result<TickOutcome, HubError> {
        if let Some(deadline) = task.deadline.as_deref() {
            if parse(now).zip(parse(deadline)).is_some_and(|(n, d)| n >= d) {
                return self.finish(task, ScheduleStatus::Failed, "deadline reached");
            }
        }
        if task.attempts >= task.max_attempts {
            return self.finish(task, ScheduleStatus::Failed, "max_attempts reached");
        }
        if let Some(allowance) = task.budget.credit_allowance {
            if task.budget.credit_spent >= allowance {
                return self.finish(task, ScheduleStatus::Paused, "credit allowance exhausted");
            }
        }

        match task.status {
            ScheduleStatus::Scheduled => self.dispatch_first(task, now),
            ScheduleStatus::Active => self.tick_active(task, now),
            _ => Ok(TickOutcome {
                id: task.id.clone(),
                action: "noop".into(),
                detail: format!("status is {}", task.status.as_str()),
            }),
        }
    }

    fn finish(
        &self,
        task: &ScheduledTask,
        status: ScheduleStatus,
        reason: &str,
    ) -> Result<TickOutcome, HubError> {
        self.set_scheduled_task_status(&task.id, status, None)?;
        let _ = self.record_audit_event(
            Path::new(&task.workspace),
            Path::new(&task.workspace),
            match status {
                ScheduleStatus::Paused => "schedule.paused_budget",
                _ => "schedule.failed",
            },
            &serde_json::json!({ "schedule_id": task.id, "reason": reason }).to_string(),
            None,
        );
        Ok(TickOutcome {
            id: task.id.clone(),
            action: status.as_str().into(),
            detail: reason.into(),
        })
    }

    fn dispatch_first(&self, task: &ScheduledTask, now: &str) -> Result<TickOutcome, HubError> {
        self.send_schedule_prompt(task, &task.instruction)?;
        self.record_dispatch(&task.id, now)?;
        if task.repeat_interval_secs.is_none() {
            self.set_scheduled_task_status(&task.id, ScheduleStatus::Completed, None)?;
            let _ = self.record_audit_event(
                Path::new(&task.workspace),
                Path::new(&task.workspace),
                "schedule.completed",
                &serde_json::json!({ "schedule_id": task.id, "reason": "one-shot dispatched" })
                    .to_string(),
                None,
            );
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "completed".into(),
                detail: "one-shot dispatched, no gate loop".into(),
            });
        }
        self.set_scheduled_task_status(&task.id, ScheduleStatus::Active, Some(now))?;
        Ok(TickOutcome {
            id: task.id.clone(),
            action: "dispatched".into(),
            detail: "first fire".into(),
        })
    }

    fn tick_active(&self, task: &ScheduledTask, now: &str) -> Result<TickOutcome, HubError> {
        let last_fired = task.last_fired_at.as_deref().unwrap_or(&task.created_at);
        let responded = self.schedule_target_responded(task, last_fired)?;
        let elapsed = seconds_between(last_fired, now);
        let timed_out = elapsed >= task.response_timeout_secs;

        if !responded && !timed_out {
            self.set_scheduled_task_status(&task.id, ScheduleStatus::Active, Some(now))?;
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "waiting".into(),
                detail: "no response yet".into(),
            });
        }

        if responded {
            let gate = self.evaluate_gate_criteria(task)?;
            let summary = gate
                .outcomes
                .iter()
                .map(|o| o.detail.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            if gate.passed {
                self.set_scheduled_task_status(&task.id, ScheduleStatus::Completed, None)?;
                let _ = self.record_audit_event(
                    Path::new(&task.workspace),
                    Path::new(&task.workspace),
                    "schedule.completed",
                    &serde_json::json!({ "schedule_id": task.id, "detail": summary }).to_string(),
                    None,
                );
                return Ok(TickOutcome {
                    id: task.id.clone(),
                    action: "completed".into(),
                    detail: summary,
                });
            }
            if let Some(interval) = task.repeat_interval_secs {
                let next_allowed = add_seconds(last_fired, interval);
                if parse(now)
                    .zip(parse(&next_allowed))
                    .is_some_and(|(n, a)| n < a)
                {
                    self.set_scheduled_task_status(
                        &task.id,
                        ScheduleStatus::Active,
                        Some(&next_allowed),
                    )?;
                    return Ok(TickOutcome {
                        id: task.id.clone(),
                        action: "waiting".into(),
                        detail: format!("gate unmet, waiting out repeat interval: {summary}"),
                    });
                }
            }
            self.send_schedule_prompt(
                task,
                &format!("{}\n\n---\nStill unmet: {summary}", task.instruction),
            )?;
            self.record_dispatch(&task.id, now)?;
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "dispatched".into(),
                detail: summary,
            });
        }

        let note = format!(
            "No response received within {}s; re-prompting.",
            task.response_timeout_secs
        );
        self.send_schedule_prompt(task, &format!("{}\n\n---\n{note}", task.instruction))?;
        self.record_dispatch(&task.id, now)?;
        Ok(TickOutcome {
            id: task.id.clone(),
            action: "dispatched".into(),
            detail: note,
        })
    }

    fn record_dispatch(&self, id: &str, now: &str) -> Result<(), HubError> {
        self.ensure_mutable()?;
        self.conn.execute(
            "UPDATE scheduled_tasks SET attempts = attempts + 1, last_fired_at = ?1,
             next_fire_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    fn send_schedule_prompt(&self, task: &ScheduledTask, body: &str) -> Result<(), HubError> {
        let subject = Some(format!("[schedule] {}", task.name));
        let messages = match &task.target {
            ScheduleTarget::Agent { agent_id } => vec![self.send_message(
                "system",
                agent_id,
                MessageKind::Handoff,
                body,
                subject.as_deref(),
                Some(&task.workspace),
                Some(&task.id),
            )?],
            ScheduleTarget::Team => self.send_message_to_team(
                "system",
                MessageKind::Handoff,
                body,
                subject.as_deref(),
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

    /// Quorum: for `Team`, every current standing-roster member (minus
    /// `system`) must have a `messages` row addressed from them,
    /// correlated by `task_id = schedule.id`, after `since`.
    fn schedule_target_responded(
        &self,
        task: &ScheduledTask,
        since: &str,
    ) -> Result<bool, HubError> {
        let agents: Vec<String> = match &task.target {
            ScheduleTarget::Agent { agent_id } => vec![agent_id.clone()],
            ScheduleTarget::Team => self
                .list_team_members()?
                .into_iter()
                .map(|a| a.id)
                // `human` is a seeded standing team member (the owner),
                // not an agent that will ever send a correlated reply —
                // excluded from the response quorum, same as `system`.
                .filter(|id| id != "system" && id != "human")
                .collect(),
        };
        if agents.is_empty() {
            return Ok(false);
        }
        for agent_id in &agents {
            let replied: bool = self.conn.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM messages
                    WHERE from_agent = ?1 AND task_id = ?2 AND created_at > ?3
                 )",
                params![agent_id, task.id, since],
                |r| r.get(0),
            )?;
            if !replied {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
