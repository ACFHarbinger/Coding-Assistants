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

#[test]
fn stale_tick_cannot_resurrect_a_quota_paused_row() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let setup = HubStore::open(&home).unwrap();
    let task = due_task(&setup, &workspace, true);
    let id = task.id.clone();
    drop(setup);

    let pauser = HubStore::open(&home).unwrap();
    let ticker = HubStore::open(&home).unwrap();
    pauser.pause_scheduled_task(&id, "quota").unwrap();
    assert!(!ticker
        .transition_scheduled_task(&id, ScheduleStatus::Active, Some("2020-01-01T00:00:01Z"))
        .unwrap());
    let fetched = ticker.get_scheduled_task(&id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Paused);
}

#[test]
fn pending_outbox_is_not_sent_after_quota_pause() {
    let (store, dir) = open();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let task = due_task(&store, &workspace, true);
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
    store.pause_scheduled_task(&task.id, "quota").unwrap();
    store.tick_scheduled_tasks("2020-01-01T00:00:02Z").unwrap();
    let sent = store
        .list_messages(None, None)
        .unwrap()
        .into_iter()
        .filter(|message| message.task_id.as_deref() == Some(task.id.as_str()))
        .count();
    assert_eq!(sent, 0);
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Paused);
}
