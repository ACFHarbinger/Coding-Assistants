use super::super::*;
use tempfile::tempdir;

fn open() -> (HubStore, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    (store, dir)
}

fn due_task(store: &HubStore, workspace: &str, stop_on_quota: bool) -> ScheduledTask {
    store
        .create_scheduled_task(
            "n",
            "do the thing",
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            workspace,
            "2020-01-01T00:00:00Z",
            Some(60),
            3600,
            vec![],
            GateCombinator::All,
            BudgetPolicy {
                stop_on_quota_exhausted: stop_on_quota,
                ..BudgetPolicy::default()
            },
            5,
            None,
            "owner",
        )
        .unwrap()
}

#[test]
fn pause_scheduled_task_marks_paused_and_drops_it_from_due() {
    let (store, dir) = open();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let task = due_task(&store, &workspace, true);
    let outcome = store
        .pause_scheduled_task(&task.id, "live quota exhausted (5h remaining 0%)")
        .unwrap();
    assert_eq!(outcome.action, "paused");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Paused);
    let due = store.due_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    assert!(due.iter().all(|row| row.id != task.id));
}

#[test]
fn cancel_still_wins_after_a_quota_pause() {
    let (store, dir) = open();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let task = due_task(&store, &workspace, true);
    store.pause_scheduled_task(&task.id, "quota").unwrap();
    let cancelled = store.cancel_scheduled_task(&task.id).unwrap();
    assert_eq!(cancelled.status, ScheduleStatus::Cancelled);
}

#[test]
fn pause_is_a_noop_on_an_already_cancelled_row() {
    let (store, dir) = open();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let task = due_task(&store, &workspace, true);
    store.cancel_scheduled_task(&task.id).unwrap();
    let outcome = store.pause_scheduled_task(&task.id, "quota").unwrap();
    assert_eq!(outcome.action, "noop");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Cancelled);
}
