use super::*;

mod branch;
mod handoff;
mod runtime;
mod wake_correlation;
pub use branch::{BranchTarget, StepBranch};
pub(crate) use handoff::render_filtered;

impl HubStore {
    pub fn workflow_stages(steps: &[WorkflowStep]) -> Vec<Vec<usize>> {
        let mut stages: Vec<Vec<usize>> = Vec::new();
        let mut i = 0usize;
        while i < steps.len() {
            if let Some(ref g) = steps[i].parallel_group {
                if g.trim().is_empty() {
                    stages.push(vec![i]);
                    i += 1;
                    continue;
                }
                let mut group = vec![i];
                let mut j = i + 1;
                while j < steps.len()
                    && steps[j]
                        .parallel_group
                        .as_ref()
                        .map(|x| x == g)
                        .unwrap_or(false)
                {
                    group.push(j);
                    j += 1;
                }
                stages.push(group);
                i = j;
            } else {
                stages.push(vec![i]);
                i += 1;
            }
        }
        stages
    }

    fn map_task_row(r: &rusqlite::Row<'_>) -> Result<TaskRecord, rusqlite::Error> {
        let steps_json: String = r.get(5)?;
        let steps: Vec<WorkflowStep> = serde_json::from_str(&steps_json).unwrap_or_default();
        let attempts_json: String = r.get(9).unwrap_or_else(|_| "{}".into());
        let open_json: String = r.get(10).unwrap_or_else(|_| "[]".into());
        let pending_json: String = r.get(11).unwrap_or_else(|_| "[]".into());
        let max_parallel: i64 = r.get(12).unwrap_or(4);
        Ok(TaskRecord {
            id: r.get(0)?,
            title: r.get(1)?,
            workspace_path: r.get(2)?,
            status: r.get(3)?,
            step_index: r.get(4)?,
            steps,
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
            last_message_id: r.get(8)?,
            output_watermark_id: r.get(14).unwrap_or(None),
            attempts: serde_json::from_str(&attempts_json).unwrap_or_default(),
            open_agents: serde_json::from_str(&open_json).unwrap_or_default(),
            pending_agents: serde_json::from_str(&pending_json).unwrap_or_default(),
            max_parallel: max_parallel.max(1) as u32,
            require_human_approval: r.get::<_, i64>(13).unwrap_or(1) > 0,
        })
    }

    pub fn create_task(
        &self,
        title: &str,
        workspace_path: Option<&str>,
        steps: &[WorkflowStep],
    ) -> Result<TaskRecord, HubError> {
        self.create_task_with_parallel(title, workspace_path, steps, 4, true)
    }

    pub fn create_task_with_parallel(
        &self,
        title: &str,
        workspace_path: Option<&str>,
        steps: &[WorkflowStep],
        max_parallel: u32,
        require_human_approval: bool,
    ) -> Result<TaskRecord, HubError> {
        self.ensure_mutable()?;
        if title.trim().is_empty() {
            return Err(HubError::Invalid("task title must not be empty".into()));
        }
        if steps.is_empty() {
            return Err(HubError::Invalid(
                "task needs at least one workflow step".into(),
            ));
        }
        let max_parallel = max_parallel.max(1);
        for (i, s) in steps.iter().enumerate() {
            if s.agent.trim().is_empty() {
                return Err(HubError::Invalid(format!("step {i}: agent required")));
            }
            if s.instruction.trim().is_empty() {
                return Err(HubError::Invalid(format!("step {i}: instruction required")));
            }
            Self::validate_step_handoff(s, i)?;
            self.upsert_agent(&s.agent, &s.agent)?;
        }
        Self::validate_branches(steps, workspace_path)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let steps_json = serde_json::to_string(steps)
            .map_err(|e| HubError::Invalid(format!("steps serialize: {e}")))?;
        self.conn.execute(
            r#"
            INSERT INTO tasks(
                id, title, workspace_path, status, step_index, steps_json,
                created_at, updated_at, last_message_id, output_watermark_id,
                attempts_json, open_agents_json, pending_agents_json, max_parallel,
                require_human_approval
            ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, NULL, NULL, '{}', '[]', '[]', ?8, ?9)
            "#,
            params![
                id,
                title,
                workspace_path,
                TaskStatus::Pending.as_str(),
                steps_json,
                now,
                now,
                max_parallel as i64,
                if require_human_approval { 1 } else { 0 },
            ],
        )?;
        self.get_task(&id)?.ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_task(&self, id: &str) -> Result<Option<TaskRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, title, workspace_path, status, step_index, steps_json,
                   created_at, updated_at, last_message_id,
                   attempts_json, open_agents_json, pending_agents_json, max_parallel,
                   require_human_approval, output_watermark_id
            FROM tasks WHERE id = ?1
            "#,
        )?;
        let row = stmt.query_row(params![id], Self::map_task_row).optional()?;
        Ok(row)
    }

    pub fn list_tasks(&self, status: Option<TaskStatus>) -> Result<Vec<TaskRecord>, HubError> {
        let mut sql = String::from(
            r#"
            SELECT id, title, workspace_path, status, step_index, steps_json,
                   created_at, updated_at, last_message_id,
                   attempts_json, open_agents_json, pending_agents_json, max_parallel,
                   require_human_approval, output_watermark_id
            FROM tasks WHERE 1=1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(st) = status {
            sql.push_str(" AND status = ?");
            params_vec.push(Box::new(st.as_str().to_string()));
        }
        sql.push_str(" ORDER BY updated_at DESC LIMIT 100");
        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), Self::map_task_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(super) fn dispatch_step(
        &self,
        task_id: &str,
        task: &TaskRecord,
        step: &WorkflowStep,
        from_agent: &str,
        note: Option<&str>,
        stage_label: &str,
    ) -> Result<String, HubError> {
        let prepared = self.prepare_handoff(task_id, step, note)?;
        self.dispatch_prepared_step(task_id, task, step, from_agent, note, stage_label, prepared)
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_prepared_step(
        &self,
        task_id: &str,
        task: &TaskRecord,
        step: &WorkflowStep,
        from_agent: &str,
        note: Option<&str>,
        stage_label: &str,
        prepared: handoff::PreparedHandoff,
    ) -> Result<String, HubError> {
        // P16 (#360): input guardrails run before anything is sent — a
        // tripwire leaves no message, no wake, and no stage activation.
        self.enforce_guardrails(
            crate::guardrails::GuardrailStage::Input,
            task.workspace_path.as_deref(),
            "task",
            task_id,
            &prepared.body,
        )?;
        // P17 (#361): metadata and filter stats are audit-only. They do
        // not choose the recipient and they are not copied into the body.
        self.record_handoff_audit(task, step, stage_label, &prepared)?;
        let body = prepared.body;
        if let Some(peer) = step
            .peer
            .as_deref()
            .map(str::trim)
            .filter(|peer| !peer.is_empty())
        {
            return self.dispatch_remote_step(
                task_id,
                task,
                step,
                peer,
                from_agent,
                &body,
                &Self::step_dispatch_body(step, note),
                stage_label,
            );
        }
        let subject = Some(format!("[{}] {}", stage_label, task.title));
        let msg = self.send_message_with_context(
            from_agent,
            &step.agent,
            MessageKind::Handoff,
            &body,
            subject.as_deref(),
            task.workspace_path.as_deref(),
            Some(task_id),
            Some(&Self::step_dispatch_body(step, note)),
        )?;
        self.request_wake(
            &step.agent,
            Some(&format!("task {task_id} {stage_label}")),
            Some(&msg.id),
            task.require_human_approval,
        )?;
        Ok(msg.id)
    }

    pub(super) fn step_dispatch_body(step: &WorkflowStep, note: Option<&str>) -> String {
        if let Some(n) = note {
            format!("{}\n\n---\nPrior note: {}", step.instruction, n)
        } else {
            step.instruction.clone()
        }
    }

    /// Remote variant of [`Self::dispatch_step`] (P11a / #334): the step runs
    /// on the TCP peer instead of a local agent. The result is recorded as
    /// the step's Handoff message with identical subject/task linkage; no
    /// local wake is raised (the local namesake did not do the work).
    /// Transport failures keep today's `HubError` contract.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_remote_step(
        &self,
        task_id: &str,
        task: &TaskRecord,
        step: &WorkflowStep,
        peer: &str,
        from_agent: &str,
        body: &str,
        original_body: &str,
        stage_label: &str,
    ) -> Result<String, HubError> {
        let token = crate::secret::resolve(crate::peer::PEER_AUTH_KEY)
            .map(|secret| secret.expose().to_string())
            .filter(|token| !token.trim().is_empty());
        let Some(token) = token else {
            return Err(HubError::Invalid(
                "remote delegation needs a pairing token (CA_TCP_AUTH_TOKEN)".into(),
            ));
        };
        let result = crate::peer::delegate_remote_step(peer, &step.agent, body, &token)
            .map_err(HubError::Invalid)?;
        // P16 (#360): the remote result is embedded in this handoff, not a
        // later agent reply — screen it before accepting it into messages.
        self.enforce_guardrails(
            crate::guardrails::GuardrailStage::Output,
            task.workspace_path.as_deref(),
            "task",
            task_id,
            &result,
        )?;
        let subject = Some(format!("[{}] {}", stage_label, task.title));
        let transcript = format!(
            "{body}\n\n---\nRemote result ({}@{peer}):\n{result}",
            step.agent
        );
        let original = format!(
            "{original_body}\n\n---\nRemote result ({}@{peer}):\n{result}",
            step.agent
        );
        let msg = self.send_message_with_context(
            from_agent,
            &step.agent,
            MessageKind::Handoff,
            &transcript,
            subject.as_deref(),
            task.workspace_path.as_deref(),
            Some(task_id),
            Some(&original),
        )?;
        Ok(msg.id)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn persist_task_runtime(
        &self,
        id: &str,
        status: &str,
        stage_index: i64,
        last_message_id: Option<&str>,
        output_watermark_id: Option<&str>,
        attempts: &std::collections::HashMap<String, u32>,
        open_agents: &[String],
        pending_agents: &[String],
    ) -> Result<(), HubError> {
        let now = Utc::now().to_rfc3339();
        let attempts_json = serde_json::to_string(attempts).unwrap_or_else(|_| "{}".into());
        let open_json = serde_json::to_string(open_agents).unwrap_or_else(|_| "[]".into());
        let pending_json = serde_json::to_string(pending_agents).unwrap_or_else(|_| "[]".into());
        self.conn.execute(
            r#"
            UPDATE tasks
            SET status = ?1, step_index = ?2, updated_at = ?3, last_message_id = ?4,
                output_watermark_id = ?5, attempts_json = ?6, open_agents_json = ?7,
                pending_agents_json = ?8
            WHERE id = ?9
            "#,
            params![
                status,
                stage_index,
                now,
                last_message_id,
                output_watermark_id,
                attempts_json,
                open_json,
                pending_json,
                id,
            ],
        )?;
        Ok(())
    }

    /// Advance to the next **stage** (sequential step or parallel group).
    /// Fails if the current stage still has open parallel agents.
    pub fn advance_task(
        &self,
        id: &str,
        from_agent: Option<&str>,
        note: Option<&str>,
    ) -> Result<TaskRecord, HubError> {
        let task = self
            .get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        let status = TaskStatus::parse(&task.status)?;
        if status.is_terminal() {
            return Err(HubError::Invalid(format!(
                "task is already {}",
                task.status
            )));
        }
        if matches!(status, TaskStatus::InputRequired | TaskStatus::AuthRequired) {
            return Err(HubError::Invalid(
                "approval gates are still unresolved".into(),
            ));
        }
        if !task.open_agents.is_empty() {
            return Err(HubError::Invalid(format!(
                "parallel stage still open for agents: {}",
                task.open_agents.join(", ")
            )));
        }
        if !task.pending_agents.is_empty() {
            return Err(HubError::Invalid(format!(
                "parallel stage still has queued agents: {}",
                task.pending_agents.join(", ")
            )));
        }

        let stages = Self::workflow_stages(&task.steps);
        if stages.is_empty() {
            return Err(HubError::Invalid("task has no steps".into()));
        }
        if status != TaskStatus::Pending {
            // P16 (#360): output guardrails screen the stage's replies and
            // the hand-back note before the workflow acts on them.
            let agents: Vec<String> = stages
                .get(task.step_index.max(0) as usize)
                .map(|stage| stage.iter().map(|i| task.steps[*i].agent.clone()).collect())
                .unwrap_or_default();
            self.enforce_stage_output_guardrails(&task, &agents, note)?;
        }

        let next_stage = if status == TaskStatus::Pending {
            0i64
        } else {
            // P20 (#367): a stage with a branch picks its successor by
            // Schedule's gate evaluator; otherwise linear advance.
            match self.decide_next_stage(&task, &stages)? {
                branch::StageDecision::Activate(index) => index,
                branch::StageDecision::Complete => {
                    return self.finish_task(id, &task, TaskStatus::Done);
                }
                branch::StageDecision::Fail(_reason) => {
                    return self.finish_task(id, &task, TaskStatus::Failed);
                }
            }
        };

        self.activate_stage(id, &task, next_stage, from_agent.unwrap_or("human"), note)
    }

    fn finish_task(
        &self,
        id: &str,
        task: &TaskRecord,
        status: TaskStatus,
    ) -> Result<TaskRecord, HubError> {
        self.persist_task_runtime(
            id,
            status.as_str(),
            task.step_index,
            task.last_message_id.as_deref(),
            task.output_watermark_id.as_deref(),
            &task.attempts,
            &[],
            &[],
        )?;
        self.sweep_oversight_exceptions()?;
        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }
}
