//! Dispatch/response/re-prompt state machine (`roadmaps/schedule.md` S3).
//!
//! Dispatch reuses `send_message`/`send_message_to_team` + `request_wake`
//! through the atomic outbox in `dispatch.rs`. "Responded" is a `messages`
//! row from an eligible target correlated by `task_id = schedule.id`.
//! `max_attempts` caps new dispatches; the in-flight final attempt may
//! still finish or time out. `deadline` is a hard cutoff.

use super::super::*;
use super::time::{add_seconds, instant_le, normalize_rfc3339, parse_utc, seconds_between};

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

impl HubStore {
    /// Process every task due at or before `now` (RFC3339). `now` must be
    /// a real instant — invalid timestamps fail closed.
    pub fn tick_scheduled_tasks(&self, now: &str) -> Result<TickSummary, HubError> {
        let now = normalize_rfc3339("now", now)?;
        let mut outcomes = self.deliver_pending_schedule_outbox(&now)?;
        for task in self.due_scheduled_tasks(&now)? {
            outcomes.push(self.tick_one(&task, &now)?);
        }
        Ok(TickSummary { outcomes })
    }

    fn tick_one(&self, task: &ScheduledTask, now: &str) -> Result<TickOutcome, HubError> {
        if let Some(deadline) = task.deadline.as_deref() {
            if instant_le(deadline, now)? {
                return self.finish(task, ScheduleStatus::Failed, "deadline reached");
            }
        }
        if let Some(allowance) = task.budget.credit_allowance {
            if task.budget.credit_spent >= allowance {
                return self.finish(task, ScheduleStatus::Paused, "credit allowance exhausted");
            }
        }
        match task.status {
            ScheduleStatus::Scheduled => {
                self.dispatch_prompt(task, now, &task.instruction, "first fire")
            }
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
        if !self.transition_scheduled_task(&task.id, status, None)? {
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "noop".into(),
                detail: "lost race to cancel or another tick".into(),
            });
        }
        let _ = self.record_audit_event(
            Path::new(&task.workspace),
            Path::new(&task.workspace),
            match status {
                ScheduleStatus::Paused => "schedule.paused_budget",
                ScheduleStatus::Completed => "schedule.completed",
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

    fn dispatch_prompt(
        &self,
        task: &ScheduledTask,
        now: &str,
        body: &str,
        detail: &str,
    ) -> Result<TickOutcome, HubError> {
        let Some(attempt) = self.claim_schedule_dispatch(task, now, body)? else {
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "noop".into(),
                detail: "dispatch claim lost".into(),
            });
        };
        self.deliver_claimed_attempt(task, attempt, now)?;
        let Some(latest) = self.get_scheduled_task(&task.id)? else {
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "noop".into(),
                detail: "schedule disappeared after claim".into(),
            });
        };
        if latest.status.is_terminal() {
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "noop".into(),
                detail: "cancelled before delivery".into(),
            });
        }
        if latest.repeat_interval_secs.is_none() {
            return self.finish(task, ScheduleStatus::Completed, "one-shot dispatched");
        }
        Ok(TickOutcome {
            id: task.id.clone(),
            action: "dispatched".into(),
            detail: detail.into(),
        })
    }

    fn tick_active(&self, task: &ScheduledTask, now: &str) -> Result<TickOutcome, HubError> {
        let last_fired = task.last_fired_at.as_deref().unwrap_or(&task.created_at);
        let responded = self.schedule_target_responded(task, last_fired)?;
        let elapsed = seconds_between(last_fired, now)?;
        let timed_out = elapsed >= task.response_timeout_secs;

        if !responded && !timed_out {
            let _ = self.transition_scheduled_task(&task.id, ScheduleStatus::Active, Some(now))?;
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
                .map(|outcome| outcome.detail.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            if gate.passed {
                return self.finish(task, ScheduleStatus::Completed, &summary);
            }
            if self.final_attempt_exhausted(task) {
                return self.finish(task, ScheduleStatus::Failed, "max_attempts reached");
            }
            return self.reprompt_after_interval(task, now, last_fired, &summary);
        }

        if self.final_attempt_exhausted(task) {
            return self.finish(task, ScheduleStatus::Failed, "max_attempts reached");
        }
        let note = format!(
            "No response received within {}s; re-prompting.",
            task.response_timeout_secs
        );
        self.reprompt_after_interval(task, now, last_fired, &note)
    }

    fn final_attempt_exhausted(&self, task: &ScheduledTask) -> bool {
        task.attempts >= task.max_attempts
    }

    fn reprompt_after_interval(
        &self,
        task: &ScheduledTask,
        now: &str,
        last_fired: &str,
        detail: &str,
    ) -> Result<TickOutcome, HubError> {
        if let Some(interval) = task.repeat_interval_secs {
            let next_allowed = add_seconds(last_fired, interval)?;
            if parse_utc("now", now)? < parse_utc("next_allowed", &next_allowed)? {
                let _ = self.transition_scheduled_task(
                    &task.id,
                    ScheduleStatus::Active,
                    Some(&next_allowed),
                )?;
                return Ok(TickOutcome {
                    id: task.id.clone(),
                    action: "waiting".into(),
                    detail: format!("waiting out repeat interval: {detail}"),
                });
            }
        }
        self.dispatch_prompt(
            task,
            now,
            &format!("{}\n\n---\n{detail}", task.instruction),
            detail,
        )
    }

    /// Quorum: every eligible standing-roster member must have a
    /// `messages` row from them, correlated by `task_id`, after `since`.
    fn schedule_target_responded(
        &self,
        task: &ScheduledTask,
        since: &str,
    ) -> Result<bool, HubError> {
        let agents = self.schedule_target_agent_ids(&task.target)?;
        if agents.is_empty() {
            return Ok(false);
        }
        let since_dt = parse_utc("since", since)?;
        for agent_id in &agents {
            let mut stmt = self.conn.prepare(
                "SELECT created_at FROM messages WHERE from_agent = ?1 AND task_id = ?2",
            )?;
            let times =
                stmt.query_map(params![agent_id, task.id], |row| row.get::<_, String>(0))?;
            let replied = times.filter_map(Result::ok).any(|created| {
                parse_utc("message.created_at", &created)
                    .ok()
                    .is_some_and(|at| at > since_dt)
            });
            if !replied {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
