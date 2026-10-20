use super::super::*;
use std::fs;
use tempfile::tempdir;

fn open() -> (HubStore, std::path::PathBuf, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    let store = HubStore::open(&home).unwrap();
    (store, dir.path().to_path_buf(), dir)
}

#[test]
fn pending_gate_waits_without_consuming_attempts_or_resubmitting() {
    use crate::os_tool;

    let (store, ws, _guard) = open();
    let workspace = ws.join("pending-gate-ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = store
        .create_scheduled_task(
            "n",
            "do the thing",
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            &workspace.to_string_lossy(),
            "2020-01-01T00:00:00Z",
            Some(1),
            3600,
            vec![GateCriterion::Command {
                program: "cargo".into(),
                args: vec!["--version".into()],
                check: GateCheck::ExitCodeZero,
            }],
            GateCombinator::All,
            BudgetPolicy::default(),
            5,
            None,
            "owner",
        )
        .unwrap();
    store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    store
        .send_message(
            "cursor",
            "system",
            MessageKind::Message,
            "done",
            None,
            Some(&workspace.to_string_lossy()),
            Some(&task.id),
        )
        .unwrap();

    let first = store.tick_scheduled_tasks("2020-01-01T00:00:05Z").unwrap();
    assert_eq!(first.outcomes[0].action, "waiting");
    assert!(first.outcomes[0].detail.contains("awaiting approval"));

    // A second tick with the same outstanding proposal must not re-prompt or
    // consume an attempt.
    let again = store.tick_scheduled_tasks("2020-01-01T00:00:06Z").unwrap();
    assert_eq!(again.outcomes[0].action, "waiting");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.attempts, 1);
    assert_eq!(fetched.status, ScheduleStatus::Active);

    let proposals = store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .filter(|event| event.operation == "os_tool.proposed")
        .collect::<Vec<_>>();
    assert_eq!(proposals.len(), 1);

    os_tool::approve(&store, &proposals[0].id, os_tool::DEFAULT_TIMEOUT).unwrap();
    let done = store.tick_scheduled_tasks("2020-01-01T00:00:07Z").unwrap();
    assert_eq!(done.outcomes[0].action, "completed");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Completed);
    assert_eq!(fetched.attempts, 1);
}
