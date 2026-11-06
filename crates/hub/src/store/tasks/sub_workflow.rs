//! Nested sub-workflows (P19 / #366).
//!
//! A step with `sub_task_id` runs that child task instead of a local
//! handoff, wake, or `peer` dispatch. The wait is stored in `open_agents`
//! as `\u{1f}sub:<child id>`. The unit-separator prefix is not a roster
//! agent id (`upsert_agent` only sees `step.agent`). No new SQL column.
//!
//! `SUB_DISPATCH_TOKEN` is the `dispatch_step` return when nothing was
//! sent. `persist_task_runtime` stores it as a null message id.

use std::cell::Cell;

use super::super::*;

/// Open-agent marker while a stage waits on a child task. Not a roster id.
pub(super) const SUB_WAIT_PREFIX: &str = "\u{1f}sub:";
/// Sentinel returned in place of a handoff message id. Never stored.
pub(super) const SUB_DISPATCH_TOKEN: &str = "\u{1f}sub-workflow";
const MAX_SUB_LINKS: usize = 8;

thread_local! {
    static RECONCILE_DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct ReconcileDepth;

impl ReconcileDepth {
    fn enter() -> Result<Self, HubError> {
        let depth = RECONCILE_DEPTH.with(Cell::get);
        if depth >= MAX_SUB_LINKS as u32 {
            return Err(HubError::Invalid(
                "sub-workflow reconcile depth exceeds 8".into(),
            ));
        }
        RECONCILE_DEPTH.with(|cell| cell.set(depth + 1));
        Ok(Self)
    }
}

impl Drop for ReconcileDepth {
    fn drop(&mut self) {
        RECONCILE_DEPTH.with(|cell| cell.set(cell.get().saturating_sub(1)));
    }
}

pub(super) fn sub_task_ref(step: &WorkflowStep) -> Option<&str> {
    step.sub_task_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
}

fn is_sub_wait(agent: &str) -> bool {
    agent.starts_with(SUB_WAIT_PREFIX)
}

fn current_sub_ids(task: &TaskRecord) -> Vec<String> {
    let stages = HubStore::workflow_stages(&task.steps);
    let Some(idxs) = stages.get(task.step_index.max(0) as usize) else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    for index in idxs {
        let Some(id) = sub_task_ref(&task.steps[*index]) else {
            continue;
        };
        if !ids.iter().any(|existing| existing == id) {
            ids.push(id.to_string());
        }
    }
    ids
}

impl HubStore {
    /// Validate the wait graph, start a pending child once, and return the
    /// no-message token. Re-entry does not call `advance_task` again.
    pub(super) fn begin_sub_workflow(
        &self,
        parent_id: &str,
        child_id: &str,
    ) -> Result<String, HubError> {
        if let Err(err) = self.validate_sub_graph(parent_id) {
            let HubError::Invalid(message) = err else {
                return Err(err);
            };
            self.fail_parent(parent_id)?;
            return Err(HubError::Invalid(message));
        }
        let child = self
            .get_task(child_id)?
            .ok_or_else(|| HubError::Invalid(format!("sub-task {child_id} does not exist")))?;
        if TaskStatus::parse(&child.status)? == TaskStatus::Pending {
            // Existing starter. "parent" is not a second dispatch path.
            self.advance_task(child_id, Some("parent"), None)?;
        }
        let child = self
            .get_task(child_id)?
            .ok_or_else(|| HubError::NotFound(child_id.into()))?;
        if matches!(
            TaskStatus::parse(&child.status)?,
            TaskStatus::Failed | TaskStatus::Cancelled
        ) {
            self.fail_parent(parent_id)?;
            return Err(HubError::Invalid(format!(
                "sub-task {child_id} {}; parent {parent_id} failed",
                child.status
            )));
        }
        Ok(SUB_DISPATCH_TOKEN.to_string())
    }

    /// Keep a non-terminal child visible to `advance_task` via the marker.
    /// Terminal saves keep the caller's sets so a finish can clear the wait.
    pub(super) fn adjusted_sub_wait_sets(
        &self,
        task_id: &str,
        status: &str,
        stage_index: i64,
        open_agents: &[String],
        pending_agents: &[String],
    ) -> Result<(Vec<String>, Vec<String>), HubError> {
        let open = open_agents.to_vec();
        let pending = pending_agents.to_vec();
        if TaskStatus::parse(status)
            .map(TaskStatus::is_terminal)
            .unwrap_or(false)
        {
            return Ok((open, pending));
        }
        let Some(task) = self.get_task(task_id)? else {
            return Ok((open, pending));
        };
        let stages = Self::workflow_stages(&task.steps);
        let Some(idxs) = stages.get(stage_index as usize) else {
            return Ok((open, pending));
        };
        let mut open = open;
        let mut pending = pending;
        let sub_steps: Vec<(String, String)> = idxs
            .iter()
            .filter_map(|index| {
                let step = &task.steps[*index];
                sub_task_ref(step).map(|id| (step.agent.clone(), id.to_string()))
            })
            .collect();
        if sub_steps.is_empty() {
            return Ok((open, pending));
        }
        for (agent, child_id) in &sub_steps {
            let agent_still_needed = idxs.iter().any(|index| {
                let step = &task.steps[*index];
                step.agent == *agent && sub_task_ref(step).is_none()
            });
            if !agent_still_needed {
                open.retain(|name| name != agent);
                pending.retain(|name| name != agent);
            }
            let marker = format!("{SUB_WAIT_PREFIX}{child_id}");
            open.retain(|name| name != &marker);
            pending.retain(|name| name != &marker);
            let Some(child) = self.get_task(child_id)? else {
                open.push(marker);
                continue;
            };
            if TaskStatus::parse(&child.status)? != TaskStatus::Done {
                open.push(marker);
            }
        }
        Ok((open, pending))
    }

    /// Called from `advance_task` before the open-agent check.
    pub(super) fn reconcile_sub_workflow_wait(&self, parent_id: &str) -> Result<(), HubError> {
        let Some(parent) = self.get_task(parent_id)? else {
            return Ok(());
        };
        let status = match TaskStatus::parse(&parent.status) {
            Ok(status) => status,
            Err(_) => return Ok(()),
        };
        if status == TaskStatus::Pending || status.is_terminal() {
            return Ok(());
        }
        let child_ids = current_sub_ids(&parent);
        if child_ids.is_empty() {
            return Ok(());
        }
        for child_id in &child_ids {
            let Some(child) = self.get_task(child_id)? else {
                self.fail_parent(parent_id)?;
                return Err(HubError::Invalid(format!(
                    "sub-task {child_id} does not exist; parent {parent_id} failed"
                )));
            };
            match TaskStatus::parse(&child.status)? {
                TaskStatus::Failed | TaskStatus::Cancelled => {
                    self.fail_parent(parent_id)?;
                    return Err(HubError::Invalid(format!(
                        "sub-task {child_id} {}; parent {parent_id} failed",
                        child.status
                    )));
                }
                TaskStatus::Done => {}
                _ => {
                    return Err(HubError::Invalid(format!("waiting on sub-task {child_id}")));
                }
            }
        }
        self.strip_sub_markers(&parent)
    }

    /// Parents blocked on `task_id` move forward or fail with the child.
    pub(super) fn reconcile_after_terminal(&self, task_id: &str) -> Result<(), HubError> {
        let _guard = ReconcileDepth::enter()?;
        let Some(task) = self.get_task(task_id)? else {
            return Ok(());
        };
        let status = match TaskStatus::parse(&task.status) {
            Ok(status) if status.is_terminal() => status,
            _ => return Ok(()),
        };
        for parent_id in self.parents_waiting_on(task_id)? {
            if status == TaskStatus::Done {
                self.release_parent(&parent_id)?;
            } else {
                self.fail_parent(&parent_id)?;
            }
        }
        Ok(())
    }

    fn validate_sub_graph(&self, root_id: &str) -> Result<(), HubError> {
        let mut visiting = Vec::new();
        self.walk_sub_links(root_id, &mut visiting)
    }

    fn walk_sub_links(&self, task_id: &str, visiting: &mut Vec<String>) -> Result<(), HubError> {
        if visiting.iter().any(|id| id == task_id) {
            let path = format!("{} -> {task_id}", visiting.join(" -> "));
            return Err(HubError::Invalid(format!("sub-workflow cycle: {path}")));
        }
        if visiting.len() > MAX_SUB_LINKS {
            return Err(HubError::Invalid("sub-workflow depth exceeds 8".into()));
        }
        let Some(task) = self.get_task(task_id)? else {
            return Err(HubError::Invalid(format!(
                "sub-task {task_id} does not exist"
            )));
        };
        visiting.push(task_id.to_string());
        for step in &task.steps {
            if let Some(child_id) = sub_task_ref(step) {
                self.walk_sub_links(child_id, visiting)?;
            }
        }
        visiting.pop();
        Ok(())
    }

    fn parents_waiting_on(&self, child_id: &str) -> Result<Vec<String>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM tasks WHERE status IN ('running', 'input_required', 'auth_required')",
        )?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        drop(stmt);
        let mut parents = Vec::new();
        for id in ids {
            if id == child_id {
                continue;
            }
            let Some(task) = self.get_task(&id)? else {
                continue;
            };
            if current_sub_ids(&task).iter().any(|sub| sub == child_id) {
                parents.push(id);
            }
        }
        Ok(parents)
    }

    fn release_parent(&self, parent_id: &str) -> Result<(), HubError> {
        let Some(parent) = self.get_task(parent_id)? else {
            return Ok(());
        };
        let status = match TaskStatus::parse(&parent.status) {
            Ok(status) => status,
            Err(_) => return Ok(()),
        };
        if status.is_terminal() || status == TaskStatus::Pending {
            return Ok(());
        }
        for child_id in current_sub_ids(&parent) {
            let Some(child) = self.get_task(&child_id)? else {
                return self.fail_parent(parent_id);
            };
            match TaskStatus::parse(&child.status)? {
                TaskStatus::Failed | TaskStatus::Cancelled => {
                    return self.fail_parent(parent_id);
                }
                TaskStatus::Done => {}
                _ => return Ok(()),
            }
        }
        if matches!(status, TaskStatus::InputRequired | TaskStatus::AuthRequired) {
            return Ok(());
        }
        let blockers = parent
            .open_agents
            .iter()
            .filter(|agent| !is_sub_wait(agent))
            .count()
            + parent
                .pending_agents
                .iter()
                .filter(|agent| !is_sub_wait(agent))
                .count();
        if blockers > 0 {
            return Ok(());
        }
        self.advance_task(parent_id, Some("parent"), None)?;
        Ok(())
    }

    fn strip_sub_markers(&self, parent: &TaskRecord) -> Result<(), HubError> {
        let open: Vec<String> = parent
            .open_agents
            .iter()
            .filter(|agent| !is_sub_wait(agent))
            .cloned()
            .collect();
        let pending: Vec<String> = parent
            .pending_agents
            .iter()
            .filter(|agent| !is_sub_wait(agent))
            .cloned()
            .collect();
        if open == parent.open_agents && pending == parent.pending_agents {
            return Ok(());
        }
        self.persist_task_runtime(
            &parent.id,
            &parent.status,
            parent.step_index,
            parent.last_message_id.as_deref(),
            parent.output_watermark_id.as_deref(),
            &parent.attempts,
            &open,
            &pending,
        )
    }

    fn fail_parent(&self, parent_id: &str) -> Result<(), HubError> {
        let Some(parent) = self.get_task(parent_id)? else {
            return Ok(());
        };
        if TaskStatus::parse(&parent.status)
            .map(TaskStatus::is_terminal)
            .unwrap_or(false)
        {
            return Ok(());
        }
        self.persist_task_runtime(
            parent_id,
            TaskStatus::Failed.as_str(),
            parent.step_index,
            parent.last_message_id.as_deref(),
            parent.output_watermark_id.as_deref(),
            &parent.attempts,
            &[],
            &[],
        )
    }
}
