use super::super::*;
use tempfile::tempdir;

fn step(agent: &str, group: Option<&str>) -> WorkflowStep {
    WorkflowStep {
        agent: agent.into(),
        role: None,
        instruction: agent.into(),
        max_retries: 0,
        parallel_group: group.map(str::to_string),
        peer: None,
        branch: None,
        sub_task_id: None,
        handoff_metadata: None,
        context_filter: None,
    }
}

fn set_runtime(store: &HubStore, id: &str, status: &str, stage: i64, open: &str, pending: &str) {
    store
        .conn
        .execute(
            "UPDATE tasks
             SET status = ?1, step_index = ?2, open_agents_json = ?3, pending_agents_json = ?4
             WHERE id = ?5",
            rusqlite::params![status, stage, open, pending, id],
        )
        .unwrap();
}

fn step_status(view: &TaskTimelineView, agent: &str) -> TimelineEventStatus {
    view.events
        .iter()
        .find(|event| event.event_type == TimelineEventType::WorkflowStep && event.actor == agent)
        .unwrap()
        .status
        .clone()
}

#[test]
fn timeline_preserves_terminal_and_interrupted_task_status() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task(
            "lifecycle",
            None,
            &[step("claude", None), step("gemini", None)],
        )
        .unwrap();

    set_runtime(&store, &task.id, "done", 1, "[]", "[]");
    let done = store.get_task_timeline(&task.id).unwrap();
    assert_eq!(done.status, "done");
    assert!(!done.is_paused);
    assert_eq!(step_status(&done, "claude"), TimelineEventStatus::Completed);
    assert_eq!(step_status(&done, "gemini"), TimelineEventStatus::Completed);

    set_runtime(&store, &task.id, "input_required", 0, "[]", "[]");
    let waiting = store.get_task_timeline(&task.id).unwrap();
    assert_eq!(waiting.status, "input_required");
    assert!(waiting.is_paused);
    assert_eq!(step_status(&waiting, "claude"), TimelineEventStatus::Paused);
    assert_eq!(
        step_status(&waiting, "gemini"),
        TimelineEventStatus::Pending
    );

    set_runtime(&store, &task.id, "auth_required", 1, "[]", "[]");
    let auth = store.get_task_timeline(&task.id).unwrap();
    assert_eq!(auth.status, "auth_required");
    assert!(auth.is_paused);
    assert_eq!(step_status(&auth, "gemini"), TimelineEventStatus::Paused);

    set_runtime(&store, &task.id, "failed", 1, "[]", "[]");
    let failed = store.get_task_timeline(&task.id).unwrap();
    assert_eq!(failed.status, "failed");
    assert!(!failed.is_paused);
    assert!(failed.failure_reason.is_some());
    assert_eq!(step_status(&failed, "gemini"), TimelineEventStatus::Failed);

    set_runtime(&store, &task.id, "cancelled", 0, "[]", "[]");
    let cancelled = store.get_task_timeline(&task.id).unwrap();
    assert_eq!(cancelled.status, "cancelled");
    assert!(!cancelled.is_paused);
}

#[test]
fn timeline_uses_stage_index_and_open_pending_agents() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task(
            "parallel",
            None,
            &[
                step("planner", None),
                step("dev_a", Some("impl")),
                step("dev_b", Some("impl")),
                step("dev_c", Some("impl")),
                step("reviewer", None),
            ],
        )
        .unwrap();
    // Stage 1 is the parallel group. dev_b already finished, dev_c is queued.
    set_runtime(
        &store,
        &task.id,
        "running",
        1,
        r#"["dev_a"]"#,
        r#"["dev_c"]"#,
    );
    let view = store.get_task_timeline(&task.id).unwrap();
    assert_eq!(view.status, "running");
    assert_eq!(
        step_status(&view, "planner"),
        TimelineEventStatus::Completed
    );
    assert_eq!(step_status(&view, "dev_a"), TimelineEventStatus::InProgress);
    assert_eq!(step_status(&view, "dev_b"), TimelineEventStatus::Completed);
    assert_eq!(step_status(&view, "dev_c"), TimelineEventStatus::Pending);
    assert_eq!(step_status(&view, "reviewer"), TimelineEventStatus::Pending);
}

#[test]
fn timeline_maps_persisted_wake_statuses() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task("wakes", None, &[step("claude", None)])
        .unwrap();
    let reason = format!("task {}", task.id);
    let pending = store
        .request_wake("claude", Some(&reason), None, true)
        .unwrap();
    let delivered = store
        .request_wake("gemini", Some(&reason), None, true)
        .unwrap();
    store
        .set_wake_status(&delivered.id, WakeStatus::Delivered)
        .unwrap();
    let cancelled = store
        .request_wake("grok", Some(&reason), None, true)
        .unwrap();
    store
        .set_wake_status(&cancelled.id, WakeStatus::Cancelled)
        .unwrap();

    set_runtime(&store, &task.id, "done", 0, "[]", "[]");
    let view = store.get_task_timeline(&task.id).unwrap();
    assert_eq!(view.status, "done");
    assert!(!view.is_paused);
    let wake_status = |id: &str| {
        view.events
            .iter()
            .find(|event| event.id == format!("evt-wake-{id}"))
            .unwrap()
            .status
            .clone()
    };
    assert_eq!(wake_status(&pending.id), TimelineEventStatus::Paused);
    assert_eq!(wake_status(&delivered.id), TimelineEventStatus::Completed);
    assert_eq!(wake_status(&cancelled.id), TimelineEventStatus::Failed);
    assert!(view
        .events
        .iter()
        .filter(|event| event.event_type == TimelineEventType::Wake)
        .all(|event| event.status != TimelineEventStatus::InProgress));
}

#[test]
fn resumed_task_does_not_inherit_a_historical_pause_alert() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task("resumed", None, &[step("worker", None)])
        .unwrap();
    set_runtime(&store, &task.id, "running", 0, "[]", "[]");
    store
        .record_audit_event(
            std::path::Path::new("."),
            std::path::Path::new("."),
            "task.paused",
            &serde_json::json!({"task_id": task.id}).to_string(),
            None,
        )
        .unwrap();
    let view = store.get_task_timeline(&task.id).unwrap();
    assert!(view
        .events
        .iter()
        .any(|event| event.status == TimelineEventStatus::Paused));
    assert!(!view.is_paused);
}
