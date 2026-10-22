//! Parallel-stage dispatch and output screening (#360, #366, #361, #365).
//!
//! - `output_watermark_id` is set exactly once, before any member of a
//!   parallel stage can respond, and is never overwritten by a later
//!   dispatch, refill, or completion of that same stage.
//! - The watermark is an internal screening cursor. External result
//!   readers (`last_message_id`, A2A `task_result`) see the actual
//!   handoff — a different field.
//! - A dispatch (initial or refill) is persisted as part of the same
//!   operation that sends it. "Succeeded but not yet saved" is not a
//!   representable state.
//! - Completing the last open member can leave a pending-only window if
//!   the queued send fails. `retry_task` then dispatches those queued
//!   members only — it does not redispatch completed work or consume a
//!   stage retry.
//! - A full window with queued members still waiting is not, by itself,
//!   a reason to refuse `retry_task`: if nothing in the stage has
//!   completed yet, a stalled *open* member can still be recovered via
//!   an explicit stage retry. Queued-dispatch recovery only short-
//!   circuits that retry once something has actually finished, since
//!   re-activating the stage at that point would redispatch completed
//!   work.
//! - Every member of a parallel stage (initial or queued/refilled) gets
//!   a handoff identifier in one consistent format (`handoff_label`);
//!   "initial" and "queued" are not different kinds of handoff, just
//!   different dispatch timings.
//! - A stage's waiting state is a function of *all* handoffs belonging
//!   to its *current* attempt only — never a previous attempt's, even
//!   if the stage subject repeats on retry. Recomputed each time a
//!   handoff resolves, never short-circuited by the most recent one.

use super::super::*;
impl HubStore {
    pub(super) fn activate_stage(
        &self,
        id: &str,
        task: &TaskRecord,
        stage_index: i64,
        from_agent: &str,
        note: Option<&str>,
    ) -> Result<TaskRecord, HubError> {
        let stages = Self::workflow_stages(&task.steps);
        let idxs = &stages[stage_index as usize];
        let mut attempts = task.attempts.clone();
        *attempts.entry(stage_index.to_string()).or_insert(0) += 1;
        let attempt = *attempts.get(&stage_index.to_string()).unwrap_or(&1);

        let last_msg: Option<String>;
        let watermark: Option<String>;
        let mut open: Vec<String> = Vec::new();
        let mut pending: Vec<String> = Vec::new();

        if idxs.len() == 1 {
            let step = &task.steps[idxs[0]];
            let msg_id = self.dispatch_step(
                id,
                task,
                step,
                from_agent,
                note,
                &Self::handoff_label(stage_index, stages.len(), attempt, &step.agent),
            )?;
            last_msg = Some(msg_id.clone());
            watermark = Some(msg_id);
        } else {
            self.preflight_stage_inputs(id, task, idxs, note)?;
            watermark = Some(self.stamp_stage_watermark(
                id,
                task.workspace_path.as_deref(),
                &format!("{}/{}", stage_index + 1, stages.len()),
            )?);
            pending = idxs.iter().map(|i| task.steps[*i].agent.clone()).collect();
            let mut last_handoff: Option<String> = None;
            let cap = task.max_parallel as usize;
            let wake_now: Vec<usize> = idxs.iter().copied().take(cap.min(idxs.len())).collect();
            for si in &wake_now {
                let step = &task.steps[*si];
                self.dispatch_and_persist(
                    id,
                    task,
                    step,
                    from_agent,
                    note,
                    &Self::handoff_label(stage_index, stages.len(), attempt, &step.agent),
                    stage_index,
                    &mut last_handoff,
                    watermark.as_deref(),
                    &attempts,
                    &mut open,
                    &mut pending,
                )?;
            }
            last_msg = last_handoff;
        }

        let status = self.aggregate_wait_status(task, stage_index, attempt)?;
        self.persist_task_runtime(
            id,
            status.as_str(),
            stage_index,
            last_msg.as_deref(),
            watermark.as_deref(),
            &attempts,
            &open,
            &pending,
        )?;
        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    /// Mark one agent finished in the current parallel stage.
    pub fn complete_parallel_member(
        &self,
        id: &str,
        agent: &str,
        note: Option<&str>,
    ) -> Result<TaskRecord, HubError> {
        let task = self
            .get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        let status = TaskStatus::parse(&task.status).unwrap_or(TaskStatus::Pending);
        if matches!(status, TaskStatus::InputRequired | TaskStatus::AuthRequired) {
            return Err(HubError::Invalid(
                "approval gates are still unresolved".into(),
            ));
        }
        if !status.is_in_progress() {
            return Err(HubError::Invalid("task is not running".into()));
        }
        if task.open_agents.is_empty() && task.pending_agents.is_empty() {
            return Err(HubError::Invalid(
                "no open parallel stage (use advance_task for sequential steps)".into(),
            ));
        }
        if !task.open_agents.iter().any(|a| a == agent) {
            return Err(HubError::Invalid(format!(
                "agent '{agent}' is not in the open parallel set"
            )));
        }
        self.enforce_stage_output_guardrails(&task, &[agent.to_string()], note)?;

        let mut open: Vec<String> = task
            .open_agents
            .iter()
            .filter(|a| *a != agent)
            .cloned()
            .collect();
        let mut pending = task.pending_agents.clone();
        let mut last_handoff = task.last_message_id.clone();
        let watermark = task.output_watermark_id.clone();
        let stage_index = task.step_index;
        let attempts = task.attempts.clone();
        let attempt = attempts.get(&stage_index.to_string()).copied().unwrap_or(1);

        // Persist the completion (agent removed from `open`) before the
        // refill attempt, so a `fill_parallel_window` failure never loses
        // the already-finished member's state.
        self.persist_task_runtime(
            id,
            TaskStatus::Running.as_str(),
            stage_index,
            last_handoff.as_deref(),
            watermark.as_deref(),
            &attempts,
            &open,
            &pending,
        )?;
        self.fill_parallel_window(
            id,
            &task,
            agent,
            note,
            &mut last_handoff,
            watermark.as_deref(),
            &mut open,
            &mut pending,
        )?;

        let final_status = self.aggregate_wait_status(&task, stage_index, attempt)?;
        if final_status == TaskStatus::Failed {
            open.clear();
            pending.clear();
        }
        self.persist_task_runtime(
            id,
            final_status.as_str(),
            stage_index,
            last_handoff.as_deref(),
            watermark.as_deref(),
            &attempts,
            &open,
            &pending,
        )?;

        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    /// Dispatch queued members into free parallel slots. Does not change
    /// the stage attempt counter or the screening watermark.
    #[allow(clippy::too_many_arguments)]
    fn fill_parallel_window(
        &self,
        id: &str,
        task: &TaskRecord,
        from_agent: &str,
        note: Option<&str>,
        last_handoff: &mut Option<String>,
        watermark: Option<&str>,
        open: &mut Vec<String>,
        pending: &mut Vec<String>,
    ) -> Result<(), HubError> {
        let stage_index = task.step_index;
        let stages = Self::workflow_stages(&task.steps);
        let idxs = stages
            .get(stage_index as usize)
            .ok_or_else(|| HubError::Invalid("invalid stage index".into()))?;
        let attempt = task
            .attempts
            .get(&stage_index.to_string())
            .copied()
            .unwrap_or(1);
        while open.len() < task.max_parallel as usize && !pending.is_empty() {
            let next_agent = pending[0].clone();
            let step = idxs
                .iter()
                .map(|i| &task.steps[*i])
                .find(|s| s.agent == next_agent)
                .ok_or_else(|| {
                    HubError::Invalid(format!("pending agent '{next_agent}' not in stage"))
                })?;
            self.dispatch_and_persist(
                id,
                task,
                step,
                from_agent,
                note,
                &Self::handoff_label(stage_index, stages.len(), attempt, &next_agent),
                stage_index,
                last_handoff,
                watermark,
                &task.attempts,
                open,
                pending,
            )?;
        }
        Ok(())
    }

    /// Send one member and persist that send in the same operation.
    /// Updates `last_message_id` to this handoff; the watermark is only
    /// written through, never replaced.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_and_persist(
        &self,
        id: &str,
        task: &TaskRecord,
        step: &WorkflowStep,
        from_agent: &str,
        note: Option<&str>,
        stage_label: &str,
        stage_index: i64,
        last_handoff: &mut Option<String>,
        watermark: Option<&str>,
        attempts: &std::collections::HashMap<String, u32>,
        open: &mut Vec<String>,
        pending: &mut Vec<String>,
    ) -> Result<(), HubError> {
        let msg_id = self.dispatch_step(id, task, step, from_agent, note, stage_label)?;
        *last_handoff = Some(msg_id);
        pending.retain(|name| name != &step.agent);
        if !open.iter().any(|name| name == &step.agent) {
            open.push(step.agent.clone());
        }
        self.persist_task_runtime(
            id,
            TaskStatus::Running.as_str(),
            stage_index,
            last_handoff.as_deref(),
            watermark,
            attempts,
            open,
            pending,
        )
    }

    fn stamp_stage_watermark(
        &self,
        task_id: &str,
        workspace: Option<&str>,
        stage_label: &str,
    ) -> Result<String, HubError> {
        let msg = self.send_message(
            "system",
            "system",
            MessageKind::System,
            &format!("stage {stage_label} output watermark"),
            Some(&format!("[{stage_label}]")),
            workspace,
            Some(task_id),
        )?;
        Ok(msg.id)
    }

    fn preflight_stage_inputs(
        &self,
        task_id: &str,
        task: &TaskRecord,
        idxs: &[usize],
        note: Option<&str>,
    ) -> Result<(), HubError> {
        for si in idxs {
            let body = Self::step_dispatch_body(&task.steps[*si], note);
            self.enforce_guardrails(
                crate::guardrails::GuardrailStage::Input,
                task.workspace_path.as_deref(),
                "task",
                task_id,
                &body,
            )?;
        }
        Ok(())
    }

    /// Retry queued parallel dispatches, or re-dispatch the current stage.
    ///
    /// Queued members (`pending_agents`) with a free window slot are
    /// dispatched in place: completed work is left alone and the stage
    /// attempt counter is not incremented. That queued-dispatch recovery
    /// is distinct from an explicit stage retry: ordinary queued members
    /// waiting for a slot must never disable recovery of a failed or
    /// stalled *open* member just because the window happens to be full.
    /// A full-stage retry is only safe when nothing in the stage has
    /// completed yet (`open.len() + pending.len() == idxs.len()`) — once
    /// any member has finished, re-activating the stage would redispatch
    /// already-completed work, so a genuinely full window with no free
    /// slot and no completion still reports "window is full" rather than
    /// risk resending to a done member. Either way a full-stage retry
    /// honours `max_retries`.
    pub fn retry_task(
        &self,
        id: &str,
        from_agent: Option<&str>,
        note: Option<&str>,
    ) -> Result<TaskRecord, HubError> {
        let task = self
            .get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        if !TaskStatus::parse(&task.status)
            .map(|status| status.is_in_progress())
            .unwrap_or(false)
        {
            return Err(HubError::Invalid("can only retry a running task".into()));
        }
        let stages = Self::workflow_stages(&task.steps);
        let stage_index = task.step_index as usize;
        if stage_index >= stages.len() {
            return Err(HubError::Invalid("invalid stage index".into()));
        }
        let idxs = &stages[stage_index];
        if !task.pending_agents.is_empty() {
            let window_full = task.open_agents.len() >= task.max_parallel as usize;
            let nothing_completed_yet =
                task.open_agents.len() + task.pending_agents.len() == idxs.len();
            if !window_full || !nothing_completed_yet {
                if window_full {
                    // Window full and something in the stage has already
                    // completed: a stage retry would redispatch that
                    // completed member's work, which is unsafe. Queued
                    // members alone don't justify that risk — surface the
                    // same "nothing to do yet" error as before.
                    return Err(HubError::Invalid(
                        "parallel window is full; complete an open member to wake queued agents"
                            .into(),
                    ));
                }
                let mut open = task.open_agents.clone();
                let mut pending = task.pending_agents.clone();
                let mut last_handoff = task.last_message_id.clone();
                self.fill_parallel_window(
                    id,
                    &task,
                    from_agent.unwrap_or("human"),
                    note,
                    &mut last_handoff,
                    task.output_watermark_id.as_deref(),
                    &mut open,
                    &mut pending,
                )?;
                return self
                    .get_task(id)?
                    .ok_or_else(|| HubError::NotFound(id.into()));
            }
            // Window full, but nothing in the stage has completed yet: an
            // explicit stage retry below is safe and is exactly the
            // "recover a stalled open member" case queued-dispatch
            // recovery cannot handle — fall through.
        }
        let max_retries = idxs
            .iter()
            .map(|i| task.steps[*i].max_retries)
            .max()
            .unwrap_or(0);
        let attempts = *task.attempts.get(&stage_index.to_string()).unwrap_or(&1);
        // After first dispatch attempts=1. With max_retries=1, one more dispatch is allowed
        // (activate will bump to 2). Block when attempts already exceeds max_retries.
        if attempts > max_retries {
            self.persist_task_runtime(
                id,
                TaskStatus::Failed.as_str(),
                task.step_index,
                task.last_message_id.as_deref(),
                task.output_watermark_id.as_deref(),
                &task.attempts,
                &[],
                &[],
            )?;
            return Err(HubError::Invalid(format!(
                "max_retries ({max_retries}) exhausted for stage {stage_index} (attempts={attempts}); task marked failed"
            )));
        }

        self.persist_task_runtime(
            id,
            TaskStatus::Running.as_str(),
            task.step_index,
            task.last_message_id.as_deref(),
            task.output_watermark_id.as_deref(),
            &task.attempts,
            &[],
            &[],
        )?;
        let task = self
            .get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        self.activate_stage(
            id,
            &task,
            task.step_index,
            from_agent.unwrap_or("human"),
            note.or(Some("retry")),
        )
    }

    pub fn cancel_task(&self, id: &str) -> Result<TaskRecord, HubError> {
        let n = self.conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2, open_agents_json = '[]', pending_agents_json = '[]' WHERE id = ?3",
            params![
                TaskStatus::Cancelled.as_str(),
                Utc::now().to_rfc3339(),
                id
            ],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    /// Recompute a waiting task from the current attempt's handoffs only.
    ///
    /// A previous attempt of the same stage is a different set, even when
    /// the stage index repeats. One member resolving does not unblock the
    /// stage while another handoff in this attempt is still pending, and a
    /// cancel counts only once every handoff in the attempt has resolved.
    pub(crate) fn apply_wake_resolution_to_task(
        &self,
        message_id: Option<&str>,
        status: WakeStatus,
    ) -> Result<(), HubError> {
        let Some(message_id) = message_id.map(str::trim).filter(|id| !id.is_empty()) else {
            return Ok(());
        };
        if status == WakeStatus::Pending {
            return Ok(());
        }
        let task_id: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT task_id FROM messages WHERE id = ?1",
                params![message_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(task_id) = task_id.flatten().filter(|id| !id.is_empty()) else {
            return Ok(());
        };
        let Some(task) = self.get_task(&task_id)? else {
            return Ok(());
        };
        if !matches!(
            TaskStatus::parse(&task.status),
            Ok(TaskStatus::InputRequired | TaskStatus::AuthRequired)
        ) {
            return Ok(());
        }
        let stages = Self::workflow_stages(&task.steps);
        if stages.get(task.step_index as usize).is_none() {
            return Ok(());
        }
        let attempt = task
            .attempts
            .get(&task.step_index.to_string())
            .copied()
            .unwrap_or(1);
        let handoffs =
            self.attempt_handoff_ids(&task.id, task.step_index, stages.len(), attempt)?;
        if !handoffs.iter().any(|id| id == message_id) {
            return Ok(());
        }
        let next = self.aggregate_wait_status(&task, task.step_index, attempt)?;
        self.persist_wait_status(&task.id, &task.status, next)
    }

    /// `{stage}/{count}/a{attempt}/{agent}` — initial and queued members share it.
    fn handoff_label(stage_index: i64, stage_count: usize, attempt: u32, agent: &str) -> String {
        format!(
            "{}/{}/a{attempt}/{agent}",
            stage_index + 1,
            stage_count.max(1)
        )
    }

    /// Waiting state of one attempt. Pending gates keep the task interrupted.
    /// When every gated handoff has resolved, a cancel fails the task and a
    /// full delivery resumes `running`. Handoffs with no gate do not wait.
    fn aggregate_wait_status(
        &self,
        task: &TaskRecord,
        stage_index: i64,
        attempt: u32,
    ) -> Result<TaskStatus, HubError> {
        let stage_count = Self::workflow_stages(&task.steps).len();
        let handoffs = self.attempt_handoff_ids(&task.id, stage_index, stage_count, attempt)?;
        let mut any_cancelled = false;
        let mut any_pending_gate = false;
        for id in &handoffs {
            let Some((wake_status, gate)) = self.latest_wake_for_message(id)? else {
                continue;
            };
            if gate == 0 {
                continue;
            }
            if wake_status == WakeStatus::Cancelled.as_str() {
                any_cancelled = true;
            } else if wake_status == WakeStatus::Pending.as_str() {
                any_pending_gate = true;
            }
        }
        if any_pending_gate {
            return Ok(TaskStatus::after_local_dispatch(
                task.require_human_approval,
                true,
            ));
        }
        if any_cancelled {
            Ok(TaskStatus::Failed)
        } else {
            Ok(TaskStatus::Running)
        }
    }

    fn attempt_handoff_ids(
        &self,
        task_id: &str,
        stage_index: i64,
        stage_count: usize,
        attempt: u32,
    ) -> Result<Vec<String>, HubError> {
        let like = format!("[{}/{}/a{attempt}/%", stage_index + 1, stage_count.max(1));
        let mut stmt = self.conn.prepare(
            "SELECT id FROM messages \
             WHERE task_id = ?1 AND kind = 'handoff' AND subject LIKE ?2",
        )?;
        let rows = stmt.query_map(params![task_id, like], |row| row.get(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn latest_wake_for_message(&self, message_id: &str) -> Result<Option<(String, i64)>, HubError> {
        Ok(self
            .conn
            .query_row(
                "SELECT status, requires_human_gate FROM wake_requests \
                 WHERE message_id = ?1 ORDER BY created_at DESC LIMIT 1",
                params![message_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?)
    }

    fn persist_wait_status(
        &self,
        task_id: &str,
        previous: &str,
        next: TaskStatus,
    ) -> Result<(), HubError> {
        let now = Utc::now().to_rfc3339();
        if next == TaskStatus::Failed {
            self.conn.execute(
                "UPDATE tasks SET status = ?1, updated_at = ?2, \
                 open_agents_json = '[]', pending_agents_json = '[]' \
                 WHERE id = ?3 AND status = ?4",
                params![next.as_str(), now, task_id, previous],
            )?;
        } else if next.as_str() != previous {
            self.conn.execute(
                "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3 AND status = ?4",
                params![next.as_str(), now, task_id, previous],
            )?;
        }
        Ok(())
    }
}
