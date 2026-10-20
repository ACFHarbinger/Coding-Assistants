use super::super::*;
use std::fs;
use tempfile::tempdir;

fn home() -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    (dir.path().join("home"), dir)
}

fn workspace(dir: &tempfile::TempDir) -> String {
    let path = dir.path().join("ws");
    fs::create_dir_all(&path).unwrap();
    path.to_string_lossy().into_owned()
}

fn due_team(store: &HubStore, workspace: &str) -> ScheduledTask {
    store
        .create_scheduled_task(
            "n",
            "do the thing",
            ScheduleTarget::Team,
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

fn due_agent(store: &HubStore, workspace: &str) -> ScheduledTask {
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

fn prime_undelivered(store: &HubStore, task: &ScheduledTask) {
    store
        .conn
        .execute(
            "UPDATE scheduled_tasks SET attempts = 1, last_fired_at = ?1,
             next_fire_at = ?1, status = 'active' WHERE id = ?2",
            rusqlite::params!["2020-01-01T00:00:01.000Z", task.id],
        )
        .unwrap();
    store
        .insert_undelivered_schedule_outbox(task, 1, &task.instruction)
        .unwrap();
}

fn sent_to(store: &HubStore, id: &str) -> Vec<String> {
    store
        .list_messages(None, None)
        .unwrap()
        .into_iter()
        .filter(|message| message.task_id.as_deref() == Some(id))
        .map(|message| message.to_agent)
        .collect()
}

#[test]
fn concurrent_outbox_drainers_send_each_recipient_once() {
    let (home, dir) = home();
    let workspace = workspace(&dir);
    let setup = HubStore::open(&home).unwrap();
    let task = due_agent(&setup, &workspace);
    prime_undelivered(&setup, &task);
    let id = task.id.clone();
    drop(setup);

    let a = HubStore::open(&home).unwrap();
    let b = HubStore::open(&home).unwrap();
    std::thread::scope(|scope| {
        scope.spawn(move || a.tick_scheduled_tasks("2020-01-01T00:00:02Z").unwrap());
        scope.spawn(move || b.tick_scheduled_tasks("2020-01-01T00:00:02Z").unwrap());
    });

    let store = HubStore::open(&home).unwrap();
    let sent = sent_to(&store, &id);
    assert_eq!(sent, vec!["cursor".to_string()]);
}

#[test]
fn partial_team_fanout_still_sends_remaining_recipients() {
    let (home, dir) = home();
    let workspace = workspace(&dir);
    let store = HubStore::open(&home).unwrap();
    store.upsert_agent("cursor", "Cursor").unwrap();
    store.upsert_agent("grok", "Grok").unwrap();
    store.set_team_member("cursor", true).unwrap();
    store.set_team_member("grok", true).unwrap();
    let task = due_team(&store, &workspace);
    prime_undelivered(&store, &task);
    store
        .send_message(
            "system",
            "grok",
            MessageKind::Handoff,
            &task.instruction,
            Some("[schedule] n#1"),
            Some(&workspace),
            Some(&task.id),
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO scheduled_dispatch_recipients(
                schedule_id, attempt, to_agent, claimed_at, message_id
             ) VALUES (?1, 1, 'grok', '2020-01-01T00:00:01.000Z', 'already')",
            rusqlite::params![task.id],
        )
        .unwrap();

    store.tick_scheduled_tasks("2020-01-01T00:00:02Z").unwrap();
    let mut sent = sent_to(&store, &task.id);
    sent.sort();
    assert_eq!(sent, vec!["cursor".to_string(), "grok".to_string()]);
    assert_eq!(sent.iter().filter(|id| *id == "grok").count(), 1);
}

#[test]
fn cancel_before_outbox_drain_does_not_send() {
    let (home, dir) = home();
    let workspace = workspace(&dir);
    let store = HubStore::open(&home).unwrap();
    let task = due_agent(&store, &workspace);
    prime_undelivered(&store, &task);
    store.cancel_scheduled_task(&task.id).unwrap();
    store.tick_scheduled_tasks("2020-01-01T00:00:02Z").unwrap();
    assert!(sent_to(&store, &task.id).is_empty());
    assert_eq!(
        store.get_scheduled_task(&task.id).unwrap().unwrap().status,
        ScheduleStatus::Cancelled
    );
}
