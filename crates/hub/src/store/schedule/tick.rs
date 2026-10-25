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

    /// Pause a live schedule because live provider quota is exhausted.
    /// Same `schedule.paused_budget` audit as credit-allowance exhaustion.
    /// No-op if the row is already terminal/paused or lost a race to cancel.
    pub fn pause_scheduled_task(&self, id: &str, reason: &str) -> Result<TickOutcome, HubError> {
        let Some(task) = self.get_scheduled_task(id)? else {
            return Err(HubError::NotFound(id.into()));
        };
        if task.status.is_terminal() || task.status == ScheduleStatus::Paused {
            return Ok(TickOutcome {
                id: task.id,
                action: "noop".into(),
                detail: format!("status is {}", task.status.as_str()),
            });
        }
        self.finish(&task, ScheduleStatus::Paused, reason)
    }

    fn tick_one(&self, task: &ScheduledTask, now: &str) -> Result<TickOutcome, HubError> {
        if let Some(stopped) = self.apply_schedule_stops(task, now)? {
            return Ok(stopped);
        }
        if task.repeat_interval_secs.is_none() && task.status == ScheduleStatus::Active {
            if let Some(completed) = self.complete_recovered_one_shot(&task.id)? {
                return Ok(completed);
            }
            // The sender/recovery owns this attempt. Never re-prompt a
            // one-shot merely because delivery is still pending.
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "waiting".into(),
                detail: "one-shot delivery pending".into(),
            });
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
        if !self.transition_live_schedule(&task.id, status, None, Some(task.attempts))? {
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

    /// Stop checks used by both `tick_one` and outbox recovery. Paused and
    /// terminal rows never send. Deadline/allowance finish the row first.
    pub(crate) fn apply_schedule_stops(
        &self,
        task: &ScheduledTask,
        now: &str,
    ) -> Result<Option<TickOutcome>, HubError> {
        if task.status == ScheduleStatus::Paused || task.status.is_terminal() {
            return Ok(Some(TickOutcome {
                id: task.id.clone(),
                action: "noop".into(),
                detail: format!("status is {}", task.status.as_str()),
            }));
        }
        if let Some(deadline) = task.deadline.as_deref() {
            if instant_le(deadline, now)? {
                return Ok(Some(self.finish(
                    task,
                    ScheduleStatus::Failed,
                    "deadline reached",
                )?));
            }
        }
        if let Some(allowance) = task.budget.credit_allowance {
            if task.budget.credit_spent >= allowance {
                return Ok(Some(self.finish(
                    task,
                    ScheduleStatus::Paused,
                    "credit allowance exhausted",
                )?));
            }
        }
        Ok(None)
    }

    pub(crate) fn complete_recovered_one_shot(
        &self,
        id: &str,
    ) -> Result<Option<TickOutcome>, HubError> {
        let Some(task) = self.get_scheduled_task(id)? else {
            return Ok(None);
        };
        if task.repeat_interval_secs.is_some()
            || task.status != ScheduleStatus::Active
            || task.attempts == 0
        {
            return Ok(None);
        }
        // A claim is not delivery: another tick can observe an active
        // one-shot while its sender is still working through recipients.
        let delivered: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM scheduled_dispatch_outbox
             WHERE schedule_id=?1 AND attempt=?2 AND delivered_at IS NOT NULL)",
            params![task.id, task.attempts],
            |row| row.get(0),
        )?;
        if !delivered {
            return Ok(None);
        }
        Ok(Some(self.finish(
            &task,
            ScheduleStatus::Completed,
            "one-shot dispatched",
        )?))
    }

    fn dispatch_prompt(
        &self,
        task: &ScheduledTask,
        now: &str,
        body: &str,
        detail: &str,
    ) -> Result<TickOutcome, HubError> {
        // P16 (#360): input guardrails run before the attempt is claimed, so
        // a tripped prompt burns no attempt and sends nothing.
        let body = self.schedule_handoff_body(task, body)?;
        let screened = self.screen_content(
            crate::guardrails::GuardrailStage::Input,
            Some(&task.workspace),
            "schedule",
            &task.id,
            &body,
        )?;
        if let Some(trip) = screened.tripped() {
            return self.finish(
                task,
                ScheduleStatus::Failed,
                &crate::store::guardrails::trip_message(
                    crate::guardrails::GuardrailStage::Input,
                    trip,
                ),
            );
        }
        let Some(attempt) = self.claim_schedule_dispatch(task, now, &body)? else {
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
        if latest.status.is_terminal() || latest.status == ScheduleStatus::Paused {
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "noop".into(),
                detail: "stopped before delivery".into(),
            });
        }
        if latest.repeat_interval_secs.is_none() {
            return self.finish(&latest, ScheduleStatus::Completed, "one-shot dispatched");
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
            if !self.transition_live_schedule(
                &task.id,
                ScheduleStatus::Active,
                Some(now),
                Some(task.attempts),
            )? {
                return Ok(TickOutcome {
                    id: task.id.clone(),
                    action: "noop".into(),
                    detail: "lost race to pause or cancel".into(),
                });
            }
            return Ok(TickOutcome {
                id: task.id.clone(),
                action: "waiting".into(),
                detail: "no response yet".into(),
            });
        }

        if responded {
            // P16 (#360): output guardrails screen each reply before any
            // gate criterion is evaluated against it. A trip counts as an
            // unmet attempt: re-prompt, or fail on the final one.
            if let Some(trip) = self.schedule_output_trip(task, last_fired)? {
                if self.final_attempt_exhausted(task) {
                    return self.finish(task, ScheduleStatus::Failed, &trip);
                }
                return self.reprompt_after_interval(task, now, last_fired, &trip);
            }
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
            if gate.pending {
                // Same attempt CAS as the other wait paths. Pending approval
                // must not burn the in-flight attempt or lose a pause race.
                if !self.transition_live_schedule(
                    &task.id,
                    ScheduleStatus::Active,
                    Some(now),
                    Some(task.attempts),
                )? {
                    return Ok(TickOutcome {
                        id: task.id.clone(),
                        action: "noop".into(),
                        detail: "lost race to pause or cancel".into(),
                    });
                }
                return Ok(TickOutcome {
                    id: task.id.clone(),
                    action: "waiting".into(),
                    detail: format!("gate awaiting approval: {summary}"),
                });
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
                if !self.transition_live_schedule(
                    &task.id,
                    ScheduleStatus::Active,
                    Some(&next_allowed),
                    Some(task.attempts),
                )? {
                    return Ok(TickOutcome {
                        id: task.id.clone(),
                        action: "noop".into(),
                        detail: "lost race to pause or cancel".into(),
                    });
                }
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

#[cfg(test)]
mod finish_race_tests {
    use super::*;

    #[test]
    fn stale_tick_waits_for_one_shot_delivery_even_after_response_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let task = store
            .create_scheduled_task(
                "race",
                "work",
                ScheduleTarget::Agent {
                    agent_id: "cursor".into(),
                },
                &dir.path().to_string_lossy(),
                "2020-01-01T00:00:00Z",
                None,
                1,
                vec![],
                GateCombinator::All,
                BudgetPolicy::default(),
                3,
                None,
                "human",
            )
            .unwrap();
        store
            .claim_schedule_dispatch(&task, "2020-01-01T00:00:01Z", "work")
            .unwrap();
        let active = store.get_scheduled_task(&task.id).unwrap().unwrap();
        assert_eq!(
            store
                .tick_one(&active, "2020-01-01T00:00:03Z")
                .unwrap()
                .action,
            "waiting"
        );
        let current = store.get_scheduled_task(&task.id).unwrap().unwrap();
        assert_eq!(current.attempts, 1);
        assert_eq!(current.status, ScheduleStatus::Active);
    }

    #[test]
    fn stale_finish_does_not_complete_a_newer_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let task = store
            .create_scheduled_task(
                "race",
                "work",
                ScheduleTarget::Agent {
                    agent_id: "cursor".into(),
                },
                &dir.path().to_string_lossy(),
                "2020-01-01T00:00:00Z",
                Some(60),
                3600,
                vec![],
                GateCombinator::All,
                BudgetPolicy::default(),
                3,
                None,
                "human",
            )
            .unwrap();
        store
            .claim_schedule_dispatch(&task, "2020-01-01T00:00:01Z", "work")
            .unwrap();
        let outcome = store
            .finish(&task, ScheduleStatus::Completed, "old gate result")
            .unwrap();
        assert_eq!(outcome.action, "noop");
        assert_eq!(
            store.get_scheduled_task(&task.id).unwrap().unwrap().status,
            ScheduleStatus::Active
        );
    }
}
