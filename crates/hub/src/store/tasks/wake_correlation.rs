//! Current-attempt-only wake correlation for parallel stages (#366, #365).
//!
//! - Every member of a parallel stage (initial or queued/refilled) gets a
//!   handoff identifier in one consistent format (`handoff_label`);
//!   "initial" and "queued" are not different kinds of handoff, just
//!   different dispatch timings.
//! - A stage's waiting state is a function of *all* handoffs belonging
//!   to its *current* attempt only — never a previous attempt's, even
//!   if the stage subject repeats on retry. Recomputed each time a
//!   handoff resolves, never short-circuited by the most recent one.

use super::super::*;
impl HubStore {
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
    pub(super) fn handoff_label(
        stage_index: i64,
        stage_count: usize,
        attempt: u32,
        agent: &str,
    ) -> String {
        format!(
            "{}/{}/a{attempt}/{agent}",
            stage_index + 1,
            stage_count.max(1)
        )
    }

    /// Waiting state of one attempt. Pending gates keep the task interrupted.
    /// When every gated handoff has resolved, a cancel fails the task and a
    /// full delivery resumes `running`. Handoffs with no gate do not wait.
    pub(super) fn aggregate_wait_status(
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
