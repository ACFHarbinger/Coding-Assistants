//! Map persisted task and wake vocabulary onto timeline event statuses.
//!
//! `task.step_index` is a workflow stage index (`HubStore::workflow_stages`),
//! not a flat offset into `task.steps`. Open and pending agents name who is
//! still running or queued inside the current stage.

use super::*;

pub(super) fn workflow_step_status(
    task: &TaskRecord,
    stages: &[Vec<usize>],
    step_idx: usize,
    agent: &str,
) -> TimelineEventStatus {
    let stage = stages
        .iter()
        .position(|group| group.contains(&step_idx))
        .map(|index| index as i64)
        .unwrap_or(step_idx as i64);
    if stage < task.step_index {
        return TimelineEventStatus::Completed;
    }
    if stage > task.step_index {
        return TimelineEventStatus::Pending;
    }
    let tracking = !task.open_agents.is_empty() || !task.pending_agents.is_empty();
    if tracking {
        if task.open_agents.iter().any(|name| name == agent) {
            return task_status_event(&task.status);
        }
        if task.pending_agents.iter().any(|name| name == agent) {
            return match TaskStatus::parse(&task.status) {
                Ok(TaskStatus::Done) => TimelineEventStatus::Completed,
                Ok(TaskStatus::Failed | TaskStatus::Cancelled) => TimelineEventStatus::Failed,
                _ => TimelineEventStatus::Pending,
            };
        }
        return TimelineEventStatus::Completed;
    }
    task_status_event(&task.status)
}

fn task_status_event(status: &str) -> TimelineEventStatus {
    match TaskStatus::parse(status) {
        Ok(TaskStatus::Pending) => TimelineEventStatus::Pending,
        Ok(TaskStatus::Running) => TimelineEventStatus::InProgress,
        Ok(TaskStatus::Done) => TimelineEventStatus::Completed,
        Ok(TaskStatus::Failed | TaskStatus::Cancelled) => TimelineEventStatus::Failed,
        Ok(TaskStatus::InputRequired | TaskStatus::AuthRequired) => TimelineEventStatus::Paused,
        Err(_) => TimelineEventStatus::InProgress,
    }
}

/// `WakeStatus` persists `pending`, `delivered`, and `cancelled`.
pub(super) fn wake_event_status(status: &str) -> TimelineEventStatus {
    match status {
        "delivered" | "completed" | "resolved" => TimelineEventStatus::Completed,
        "cancelled" | "failed" | "rejected" => TimelineEventStatus::Failed,
        "pending" => TimelineEventStatus::Paused,
        _ => TimelineEventStatus::InProgress,
    }
}

/// Keep the stored task status on the view. Terminal tasks stay terminal.
/// Interrupted tasks (`input_required`, `auth_required`) raise the pause alert.
pub(super) fn timeline_presentation(
    task_status: &str,
    events: &[TaskTimelineEvent],
) -> (String, bool) {
    let parsed = TaskStatus::parse(task_status).ok();
    let terminal = parsed.is_some_and(TaskStatus::is_terminal);
    let interrupted = matches!(
        parsed,
        Some(TaskStatus::InputRequired | TaskStatus::AuthRequired)
    );
    let event_paused = events
        .iter()
        .any(|event| event.status == TimelineEventStatus::Paused);
    (
        task_status.to_string(),
        !terminal && (interrupted || event_paused),
    )
}
