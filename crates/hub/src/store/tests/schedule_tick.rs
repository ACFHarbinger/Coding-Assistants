use super::super::*;
use std::fs;
use tempfile::tempdir;

fn open() -> (HubStore, std::path::PathBuf, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    let store = HubStore::open(&home).unwrap();
    (store, dir.path().to_path_buf(), dir)
}

#[allow(clippy::too_many_arguments)]
fn make(
    store: &HubStore,
    workspace: &str,
    target: ScheduleTarget,
    fire_at: &str,
    repeat: Option<i64>,
    response_timeout: i64,
    criteria: Vec<GateCriterion>,
    max_attempts: u32,
    deadline: Option<&str>,
) -> ScheduledTask {
    store
        .create_scheduled_task(
            "n",
            "do the thing",
            target,
            workspace,
            fire_at,
            repeat,
            response_timeout,
            criteria,
            GateCombinator::All,
            BudgetPolicy::default(),
            max_attempts,
            deadline,
            "owner",
        )
        .unwrap()
}

#[test]
fn one_shot_task_fires_once_and_completes_with_no_gate_loop() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make(
        &store,
        &workspace.to_string_lossy(),
        ScheduleTarget::Agent {
            agent_id: "cursor".into(),
        },
        "2020-01-01T00:00:00Z",
        None,
        3600,
        vec![],
        3,
        None,
    );

    let summary = store
        .tick_scheduled_tasks(&chrono::Utc::now().to_rfc3339())
        .unwrap();
    assert_eq!(summary.outcomes.len(), 1);
    assert_eq!(summary.outcomes[0].action, "completed");

    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Completed);
    assert_eq!(fetched.attempts, 1);
}

#[test]
fn active_task_waits_when_no_response_and_not_timed_out() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make(
        &store,
        &workspace.to_string_lossy(),
        ScheduleTarget::Agent {
            agent_id: "cursor".into(),
        },
        "2020-01-01T00:00:00Z",
        Some(900),
        3600,
        vec![],
        5,
        None,
    );
    let now1 = "2020-01-01T00:00:01Z";
    let s1 = store.tick_scheduled_tasks(now1).unwrap();
    assert_eq!(s1.outcomes[0].action, "dispatched");

    // Shortly after, with no reply yet and well under the 3600s timeout.
    let now2 = "2020-01-01T00:10:00Z";
    let s2 = store.tick_scheduled_tasks(now2).unwrap();
    assert_eq!(s2.outcomes[0].action, "waiting");

    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Active);
    assert_eq!(fetched.attempts, 1, "waiting must not consume an attempt");
}

#[test]
fn active_task_re_prompts_after_response_timeout_elapses() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make(
        &store,
        &workspace.to_string_lossy(),
        ScheduleTarget::Agent {
            agent_id: "cursor".into(),
        },
        "2020-01-01T00:00:00Z",
        Some(60),
        120, // response_timeout_secs
        vec![],
        5,
        None,
    );
    store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();

    // 200s later, still no reply -> past the 120s response timeout.
    let s2 = store.tick_scheduled_tasks("2020-01-01T00:03:21Z").unwrap();
    assert_eq!(s2.outcomes[0].action, "dispatched");

    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.attempts, 2);
}

#[test]
fn active_task_completes_once_target_responds_and_gate_passes() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make(
        &store,
        &workspace.to_string_lossy(),
        ScheduleTarget::Agent {
            agent_id: "cursor".into(),
        },
        "2020-01-01T00:00:00Z",
        Some(60),
        3600,
        vec![],
        5,
        None,
    );
    store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();

    // Agent replies, correlated by task_id.
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

    let s2 = store.tick_scheduled_tasks("2020-01-01T00:00:05Z").unwrap();
    assert_eq!(s2.outcomes[0].action, "completed");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Completed);
}

#[test]
fn active_task_re_prompts_with_unmet_detail_when_gate_fails_after_response() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make(
        &store,
        &workspace.to_string_lossy(),
        ScheduleTarget::Agent {
            agent_id: "cursor".into(),
        },
        "2020-01-01T00:00:00Z",
        Some(0), // no minimum spacing, re-prompt immediately on gate failure
        3600,
        vec![GateCriterion::Command {
            program: "false".into(),
            args: vec![],
            check: GateCheck::ExitCodeZero,
        }],
        5,
        None,
    );
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

    let s2 = store.tick_scheduled_tasks("2020-01-01T00:00:05Z").unwrap();
    assert_eq!(s2.outcomes[0].action, "dispatched");
    assert!(s2.outcomes[0].detail.contains("false"));

    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.attempts, 2);
    assert_eq!(fetched.status, ScheduleStatus::Active);
}

#[test]
fn max_attempts_reached_marks_task_failed_unconditionally() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make(
        &store,
        &workspace.to_string_lossy(),
        ScheduleTarget::Agent {
            agent_id: "cursor".into(),
        },
        "2020-01-01T00:00:00Z",
        Some(0),
        1, // response_timeout_secs: times out almost immediately
        vec![],
        2, // max_attempts
        None,
    );
    store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    // No response ever arrives; each tick times out and re-dispatches
    // until max_attempts trips.
    store.tick_scheduled_tasks("2020-01-01T00:00:03Z").unwrap();
    let s3 = store.tick_scheduled_tasks("2020-01-01T00:00:05Z").unwrap();

    assert_eq!(s3.outcomes[0].action, "failed");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Failed);
    assert!(fetched.attempts >= 2);
    let _ = task;
}

#[test]
fn deadline_reached_marks_task_failed_before_any_other_check() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make(
        &store,
        &workspace.to_string_lossy(),
        ScheduleTarget::Team,
        "2020-01-01T00:00:00Z",
        Some(60),
        3600,
        vec![],
        100,
        Some("2020-01-01T00:00:00Z"),
    );
    let s1 = store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    assert_eq!(s1.outcomes[0].action, "failed");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Failed);
}

#[test]
fn credit_allowance_exhausted_pauses_rather_than_fails() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            &workspace.to_string_lossy(),
            "2020-01-01T00:00:00Z",
            Some(60),
            3600,
            vec![],
            GateCombinator::All,
            BudgetPolicy {
                credit_allowance: Some(1.0),
                credit_spent: 1.0,
                stop_on_quota_exhausted: false,
            },
            5,
            None,
            "owner",
        )
        .unwrap();
    let s1 = store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    assert_eq!(s1.outcomes[0].action, "paused");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Paused);
}

#[test]
fn team_target_requires_all_roster_members_to_respond() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("ws");
    fs::create_dir_all(&workspace).unwrap();
    store.upsert_agent("grok", "Grok").unwrap();
    store.upsert_agent("cursor", "Cursor").unwrap();
    store.set_team_member("grok", true).unwrap();
    store.set_team_member("cursor", true).unwrap();

    let task = make(
        &store,
        &workspace.to_string_lossy(),
        ScheduleTarget::Team,
        "2020-01-01T00:00:00Z",
        Some(0),
        3600,
        vec![],
        5,
        None,
    );
    store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();

    // Only one of the two roster members replies.
    store
        .send_message(
            "grok",
            "system",
            MessageKind::Message,
            "done",
            None,
            Some(&workspace.to_string_lossy()),
            Some(&task.id),
        )
        .unwrap();
    let s2 = store.tick_scheduled_tasks("2020-01-01T00:00:05Z").unwrap();
    // Not a full quorum yet -> still waiting (not timed out: well under
    // the 3600s response_timeout).
    assert_eq!(s2.outcomes[0].action, "waiting");

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
    let s3 = store.tick_scheduled_tasks("2020-01-01T00:00:06Z").unwrap();
    assert_eq!(s3.outcomes[0].action, "completed");
}
