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

        let last_msg: Option<String>;
        let mut open: Vec<String> = Vec::new();
        let mut pending: Vec<String> = Vec::new();

        if idxs.len() == 1 {
            let step = &task.steps[idxs[0]];
            let msg_id = self.dispatch_step(id, task, step, from_agent, note, &stage_label)?;
            last_msg = Some(msg_id);
        } else {
            // Screen every member before sending any, so a later input
            // trip cannot orphan an earlier message+wake.
            self.preflight_stage_inputs(id, task, idxs, note)?;
            // Stamp the output watermark before any member can reply, so a
            // later (e.g. remote) dispatch cannot hide an earlier reply.
            last_msg = Some(self.stamp_stage_watermark(
                id,
                task.workspace_path.as_deref(),
                &stage_label,
            )?);
            // Every member starts pending; leave it there until its own
            // dispatch succeeds, including those inside the first window.
            pending = idxs.iter().map(|i| task.steps[*i].agent.clone()).collect();
            let cap = task.max_parallel as usize;
            let wake_now: Vec<usize> = idxs.iter().copied().take(cap.min(idxs.len())).collect();
            for si in &wake_now {
                let step = &task.steps[*si];
                self.dispatch_step(
                    id,
                    task,
                    step,
                    from_agent,
                    note,
                    &format!("{stage_label}/{}", step.agent),
                )?;
                pending.retain(|agent| agent != &step.agent);
                open.push(step.agent.clone());
                self.persist_task_runtime(
                    id,
                    TaskStatus::Running.as_str(),
                    stage_index,
                    last_msg.as_deref(),
                    &attempts,
                    &open,
                    &pending,
                )?;
            }
        }

        self.persist_task_runtime(
            id,
            TaskStatus::Running.as_str(),
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
        if task.status != TaskStatus::Running.as_str() {
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
        // P16 (#360): screen this member's replies before accepting them.
        self.enforce_stage_output_guardrails(&task, &[agent.to_string()], note)?;

        let mut open: Vec<String> = task
            .open_agents
            .iter()
            .filter(|a| *a != agent)
            .cloned()
            .collect();
        let mut pending = task.pending_agents.clone();
        // Keep the stage-activation watermark. Moving last_message_id to a
        // queued member's dispatch would hide earlier members' replies
        // (and the embedded remote result) from later output screening.
        let last_msg = task.last_message_id.clone();
        let stage_index = task.step_index;
        let max_parallel = task.max_parallel;
        let attempts = task.attempts.clone();

        while open.len() < max_parallel as usize && !pending.is_empty() {
            let next_agent = pending[0].clone();
            let stages = Self::workflow_stages(&task.steps);
            let idxs = &stages[stage_index as usize];
            let step = idxs
                .iter()
                .map(|i| &task.steps[*i])
                .find(|s| s.agent == next_agent)
                .ok_or_else(|| {
                    HubError::Invalid(format!("pending agent '{next_agent}' not in stage"))
                })?;
            self.dispatch_step(
                id,
                &task,
                step,
                agent,
                note,
                &format!("{}/{}", stage_index + 1, next_agent),
            )?;
            pending.remove(0);
            open.push(next_agent);
        }

        self.persist_task_runtime(
            id,
            TaskStatus::Running.as_str(),
            stage_index,
            last_msg.as_deref(),
            &attempts,
            &open,
            &pending,
        )?;
        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
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
        if task.status != TaskStatus::Running.as_str() {
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
}
