use super::super::*;
use std::fs;
use tempfile::tempdir;

fn home() -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    (dir.path().join("home"), dir)
}

fn create_due(store: &HubStore, workspace: &str) -> ScheduledTask {
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
            BudgetPolicy::default(),
            5,
            None,
            "owner",
        )
        .unwrap()
}

#[test]
fn concurrent_ticks_claim_a_single_attempt() {
    let (home, dir) = home();
    let workspace = dir.path().join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let setup = HubStore::open(&home).unwrap();
    let task = create_due(&setup, &workspace);
    let id = task.id.clone();
    drop(setup);

    let a = HubStore::open(&home).unwrap();
    let b = HubStore::open(&home).unwrap();
    std::thread::scope(|scope| {
        scope.spawn(move || a.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap());
        scope.spawn(move || b.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap());
    });

    let store = HubStore::open(&home).unwrap();
    let fetched = store.get_scheduled_task(&id).unwrap().unwrap();
    assert_eq!(fetched.attempts, 1);
    let sent = store
        .list_messages(None, None)
        .unwrap()
        .into_iter()
        .filter(|message| message.task_id.as_deref() == Some(id.as_str()))
        .count();
    assert_eq!(sent, 1);
}

#[test]
fn stale_tick_cannot_overwrite_a_concurrent_cancel() {
    let (home, dir) = home();
    let workspace = dir.path().join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let store = HubStore::open(&home).unwrap();
    let task = create_due(&store, &workspace);
    store.cancel_scheduled_task(&task.id).unwrap();
    let summary = store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    assert!(
        summary.outcomes.is_empty()
            || summary
                .outcomes
                .iter()
                .all(|outcome| outcome.action == "noop")
    );
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Cancelled);
    assert_eq!(fetched.attempts, 0);
}

#[test]
fn undelivered_outbox_retries_without_consuming_another_attempt() {
    let (home, dir) = home();
    let workspace = dir.path().join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let store = HubStore::open(&home).unwrap();
    let task = create_due(&store, &workspace);
    store
        .conn
        .execute(
            "UPDATE scheduled_tasks SET attempts = 1, last_fired_at = ?1,
             next_fire_at = ?1, status = 'active' WHERE id = ?2",
            rusqlite::params!["2020-01-01T00:00:01.000Z", task.id],
        )
        .unwrap();
    store
        .insert_undelivered_schedule_outbox(&task, 1, &task.instruction)
        .unwrap();
    let summary = store.tick_scheduled_tasks("2020-01-01T00:00:02Z").unwrap();
    assert!(summary
        .outcomes
        .iter()
        .any(|outcome| outcome.action == "dispatched"));
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.attempts, 1);
    let sent = store
        .list_messages(None, None)
        .unwrap()
        .into_iter()
        .filter(|message| message.task_id.as_deref() == Some(task.id.as_str()))
        .count();
    assert_eq!(sent, 1);
}

#[test]
fn corrupt_target_json_is_quarantined_and_never_dispatched() {
    let (home, dir) = home();
    let workspace = dir.path().join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let store = HubStore::open(&home).unwrap();
    let task = create_due(&store, &workspace);
    store
        .conn
        .execute(
            "UPDATE scheduled_tasks SET target_json = 'not-json' WHERE id = ?1",
            rusqlite::params![task.id],
        )
        .unwrap();
    let summary = store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    assert!(summary.outcomes.is_empty());
    let events = store.list_audit_events(false).unwrap();
    assert!(events
        .iter()
        .any(|event| event.operation == "schedule.quarantined"));
    assert!(store.get_scheduled_task(&task.id).is_err());
    let sent = store
        .list_messages(None, None)
        .unwrap()
        .into_iter()
        .filter(|message| message.task_id.as_deref() == Some(task.id.as_str()))
        .count();
    assert_eq!(sent, 0);
}
