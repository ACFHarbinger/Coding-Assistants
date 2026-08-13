use super::*;

mod runtime;
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
            self.upsert_agent(&s.agent, &s.agent)?;
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let steps_json = serde_json::to_string(steps)
            .map_err(|e| HubError::Invalid(format!("steps serialize: {e}")))?;
        self.conn.execute(
            r#"
            INSERT INTO tasks(
                id, title, workspace_path, status, step_index, steps_json,
                created_at, updated_at, last_message_id,
                attempts_json, open_agents_json, pending_agents_json, max_parallel,
                require_human_approval
            ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, NULL, '{}', '[]', '[]', ?8, ?9)
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
                   require_human_approval
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
                   require_human_approval
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
        let body = if let Some(n) = note {
            format!("{}\n\n---\nPrior note: {}", step.instruction, n)
        } else {
            step.instruction.clone()
        };
        let subject = Some(format!("[{}] {}", stage_label, task.title));
        let msg = self.send_message(
            from_agent,
            &step.agent,
            MessageKind::Handoff,
            &body,
            subject.as_deref(),
            task.workspace_path.as_deref(),
            Some(task_id),
        )?;
        let _wake = self.request_wake(
            &step.agent,
            Some(&format!("task {task_id} {stage_label}")),
            Some(&msg.id),
            task.require_human_approval,
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
                attempts_json = ?5, open_agents_json = ?6, pending_agents_json = ?7
            WHERE id = ?8
            "#,
            params![
                status,
                stage_index,
                now,
                last_message_id,
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

        let next_stage = if status == TaskStatus::Pending {
            0i64
        } else {
            let ni = task.step_index + 1;
            if ni >= stages.len() as i64 {
                self.persist_task_runtime(
                    id,
                    TaskStatus::Done.as_str(),
                    task.step_index,
                    task.last_message_id.as_deref(),
                    &task.attempts,
                    &[],
                    &[],
                )?;
                return self
                    .get_task(id)?
                    .ok_or_else(|| HubError::NotFound(id.into()));
            }
            ni
        };

        self.activate_stage(id, &task, next_stage, from_agent.unwrap_or("human"), note)
    }
}
