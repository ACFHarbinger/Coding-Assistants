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
        let stage_label = format!("{}/{}", stage_index + 1, stages.len());
        let mut attempts = task.attempts.clone();
        *attempts.entry(stage_index.to_string()).or_insert(0) += 1;

        let mut last_msg: Option<String> = None;
        let mut open: Vec<String> = Vec::new();
        let mut pending: Vec<String> = Vec::new();
        let mut gated = false;

        if idxs.len() == 1 {
            let step = &task.steps[idxs[0]];
            let dispatched = self.dispatch_step(id, task, step, from_agent, note, &stage_label)?;
            last_msg = Some(dispatched.message_id);
            gated = dispatched.requires_human_gate;
        } else {
            let mut agents_to_run: Vec<usize> = idxs.clone();
            let cap = task.max_parallel as usize;
            let take = cap.min(agents_to_run.len());
            let wake_now: Vec<usize> = agents_to_run.drain(..take).collect();
            for si in &wake_now {
                let step = &task.steps[*si];
                let dispatched = self.dispatch_step(
                    id,
                    task,
                    step,
                    from_agent,
                    note,
                    &format!("{stage_label}/{}", step.agent),
                )?;
                last_msg = Some(dispatched.message_id);
                gated |= dispatched.requires_human_gate;
                open.push(step.agent.clone());
            }
            for si in agents_to_run {
                pending.push(task.steps[si].agent.clone());
            }
        }

        let status = TaskStatus::after_local_dispatch(task.require_human_approval, gated);
        self.persist_task_runtime(
            id,
            status.as_str(),
            stage_index,
            last_msg.as_deref(),
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
        if !TaskStatus::parse(&task.status)
            .map(|status| status.is_in_progress())
            .unwrap_or(false)
        {
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

        let mut open: Vec<String> = task
            .open_agents
            .iter()
            .filter(|a| *a != agent)
            .cloned()
            .collect();
        let mut pending = task.pending_agents.clone();
        let mut last_msg = task.last_message_id.clone();
        let stage_index = task.step_index;
        let max_parallel = task.max_parallel;
        let attempts = task.attempts.clone();
        let mut dispatched_any = false;
        let mut gated = false;

        while open.len() < max_parallel as usize && !pending.is_empty() {
            let next_agent = pending.remove(0);
            let stages = Self::workflow_stages(&task.steps);
            let idxs = &stages[stage_index as usize];
            let step = idxs
                .iter()
                .map(|i| &task.steps[*i])
                .find(|s| s.agent == next_agent)
                .ok_or_else(|| {
                    HubError::Invalid(format!("pending agent '{next_agent}' not in stage"))
                })?;
            let dispatched = self.dispatch_step(
                id,
                &task,
                step,
                agent,
                note,
                &format!("{}/{}", stage_index + 1, next_agent),
            )?;
            last_msg = Some(dispatched.message_id);
            gated = dispatched.requires_human_gate;
            dispatched_any = true;
            open.push(next_agent);
        }

        let status = if dispatched_any {
            TaskStatus::after_local_dispatch(task.require_human_approval, gated)
        } else {
            TaskStatus::parse(&task.status).unwrap_or(TaskStatus::Running)
        };
        self.persist_task_runtime(
            id,
            status.as_str(),
            stage_index,
            last_msg.as_deref(),
            &attempts,
            &open,
            &pending,
        )?;
        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    /// Re-dispatch the current stage (honours max_retries on the stage).
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

    /// Recompute a waiting task from every gated wake on the current stage.
    ///
    /// A parallel stage dispatches several handoffs and only `last_message_id`
    /// remembers the latest. Any gated cancel on this stage fails the task.
    /// A delivery resumes `running` only when no gated wake for the stage is
    /// still pending. Wakes from other stages, and tasks that are not waiting,
    /// stay put.
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
        let Some(stage_label) = stages
            .get(task.step_index as usize)
            .map(|_| format!("{}/{}", task.step_index + 1, stages.len()))
        else {
            return Ok(());
        };
        let handoffs = self.stage_handoff_ids(&task.id, &stage_label)?;
        if !handoffs.iter().any(|id| id == message_id) {
            return Ok(());
        }
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
        let now = Utc::now().to_rfc3339();
        if any_cancelled {
            self.conn.execute(
                "UPDATE tasks SET status = ?1, updated_at = ?2, \
                 open_agents_json = '[]', pending_agents_json = '[]' \
                 WHERE id = ?3 AND status = ?4",
                params![TaskStatus::Failed.as_str(), now, task.id, task.status],
            )?;
        } else if !any_pending_gate {
            self.conn.execute(
                "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3 AND status = ?4",
                params![TaskStatus::Running.as_str(), now, task.id, task.status],
            )?;
        }
        Ok(())
    }

    fn stage_handoff_ids(&self, task_id: &str, stage_label: &str) -> Result<Vec<String>, HubError> {
        let sequential = format!("[{stage_label}]%");
        let parallel = format!("[{stage_label}/%");
        let mut stmt = self.conn.prepare(
            "SELECT id FROM messages \
             WHERE task_id = ?1 AND kind = 'handoff' \
               AND (subject LIKE ?2 OR subject LIKE ?3)",
        )?;
        let rows = stmt.query_map(params![task_id, sequential, parallel], |row| row.get(0))?;
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
}
