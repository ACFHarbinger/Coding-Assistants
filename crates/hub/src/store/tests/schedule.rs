use super::super::*;
use tempfile::tempdir;

fn open() -> HubStore {
    let dir = tempdir().unwrap();
    HubStore::open(dir.keep()).unwrap()
}

#[test]
fn create_scheduled_task_round_trips_and_defaults_status_to_scheduled() {
    let store = open();
    let task = store
        .create_scheduled_task(
            "nightly LoC sweep",
            "Keep files under 500 lines",
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            "/tmp/ws",
            "2026-10-20T00:00:00Z",
            Some(3600),
            1800,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            5,
            None,
            "owner",
        )
        .unwrap();
    assert_eq!(task.status, ScheduleStatus::Scheduled);
    assert_eq!(task.max_attempts, 5);
    assert_eq!(
        task.next_fire_at.as_deref(),
        Some("2026-10-20T00:00:00.000Z")
    );

    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.id, task.id);
    assert_eq!(fetched.name, "nightly LoC sweep");
    assert_eq!(fetched.target, task.target);
}

#[test]
fn create_scheduled_task_rejects_empty_name_instruction_and_zero_attempts() {
    let store = open();
    let base = |name: &str, instruction: &str, max_attempts: u32| {
        store.create_scheduled_task(
            name,
            instruction,
            ScheduleTarget::Team,
            "/tmp/ws",
            "2026-10-20T00:00:00Z",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            max_attempts,
            None,
            "owner",
        )
    };
    assert!(base("", "do it", 3).is_err());
    assert!(base("name", "", 3).is_err());
    assert!(
        base("name", "do it", 0).is_err(),
        "max_attempts=0 must be rejected — it is a mandatory stop condition"
    );
}

#[test]
fn create_scheduled_task_rejects_relative_workspace_and_bad_timestamps() {
    let store = open();
    assert!(store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            "relative/path",
            "2026-10-20T00:00:00Z",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .is_err());
    assert!(store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            "/tmp/ws",
            "not-a-timestamp",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .is_err());
}

#[test]
fn list_scheduled_tasks_orders_newest_first() {
    let store = open();
    let make = |name: &str| {
        store
            .create_scheduled_task(
                name,
                "i",
                ScheduleTarget::Team,
                "/tmp/ws",
                "2026-10-20T00:00:00Z",
                None,
                60,
                vec![],
                GateCombinator::All,
                BudgetPolicy::default(),
                3,
                None,
                "owner",
            )
            .unwrap()
    };
    let first = make("first");
    std::thread::sleep(std::time::Duration::from_millis(5));
    let second = make("second");

    let all = store.list_scheduled_tasks().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, second.id);
    assert_eq!(all[1].id, first.id);
}

#[test]
fn due_scheduled_tasks_only_returns_scheduled_and_active_rows_at_or_before_now() {
    let store = open();
    let due = store
        .create_scheduled_task(
            "due",
            "i",
            ScheduleTarget::Team,
            "/tmp/ws",
            "2020-01-01T00:00:00Z",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap();
    let not_yet = store
        .create_scheduled_task(
            "not yet",
            "i",
            ScheduleTarget::Team,
            "/tmp/ws",
            "2099-01-01T00:00:00Z",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap();

    let due_now = store
        .due_scheduled_tasks(&chrono::Utc::now().to_rfc3339())
        .unwrap();
    let ids: Vec<&str> = due_now.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&due.id.as_str()));
    assert!(!ids.contains(&not_yet.id.as_str()));
}

#[test]
fn cancel_scheduled_task_is_terminal_and_idempotent() {
    let store = open();
    let task = store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            "/tmp/ws",
            "2026-10-20T00:00:00Z",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap();
    let cancelled = store.cancel_scheduled_task(&task.id).unwrap();
    assert_eq!(cancelled.status, ScheduleStatus::Cancelled);
    assert!(cancelled.next_fire_at.is_none());

    // Cancelling an already-terminal task is a no-op, not an error.
    let again = store.cancel_scheduled_task(&task.id).unwrap();
    assert_eq!(again.status, ScheduleStatus::Cancelled);
}

#[test]
fn create_scheduled_task_snapshots_token_baseline_for_target() {
    let store = open();
    store
        .record_agent_metrics("cursor", 10, 4242, 0, 100)
        .unwrap();
    let task = store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            "/tmp/ws",
            "2026-10-20T00:00:00Z",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap();
    assert_eq!(task.token_baseline.get("cursor"), Some(&4242));
}

#[test]
fn gate_criteria_and_budget_round_trip_through_json_columns() {
    let store = open();
    let criteria = vec![
        GateCriterion::Command {
            program: "wc".into(),
            args: vec!["-l".into()],
            check: GateCheck::AtMost { max: 500 },
        },
        GateCriterion::MaxLinesPerFile {
            max_lines: 500,
            globs: vec!["**/*.rs".into()],
        },
        GateCriterion::MinTokensWritten {
            min_tokens: 100_000,
        },
    ];
    let budget = BudgetPolicy {
        credit_allowance: Some(5.0),
        credit_spent: 0.0,
        stop_on_quota_exhausted: true,
    };
    let task = store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            "/tmp/ws",
            "2026-10-20T00:00:00Z",
            Some(900),
            60,
            criteria.clone(),
            GateCombinator::Any,
            budget.clone(),
            3,
            None,
            "owner",
        )
        .unwrap();
    assert_eq!(task.gate_criteria.len(), 3);
    assert_eq!(task.gate_combinator, GateCombinator::Any);
    assert_eq!(task.budget.credit_allowance, Some(5.0));
    assert!(task.budget.stop_on_quota_exhausted);

    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.gate_criteria.len(), 3);
    assert_eq!(fetched.gate_combinator, GateCombinator::Any);
}

#[test]
fn create_scheduled_task_rejects_nonpositive_repeat_interval() {
    let store = open();
    let make = |repeat| {
        store.create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            "/tmp/ws",
            "2026-10-20T00:00:00Z",
            repeat,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
    };
    assert!(make(Some(0)).is_err());
    assert!(make(Some(-30)).is_err());
    assert!(make(Some(1)).is_ok());
}

#[test]
fn create_scheduled_task_normalizes_offset_fire_at_to_utc() {
    let store = open();
    let task = store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            "/tmp/ws",
            "2026-10-20T10:00:00+02:00",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap();
    assert_eq!(task.fire_at, "2026-10-20T08:00:00.000Z");
    let due = store.due_scheduled_tasks("2026-10-20T08:00:00Z").unwrap();
    assert!(due.iter().any(|row| row.id == task.id));
}

#[test]
fn team_token_baseline_omits_human_system_and_missing_metrics() {
    let store = open();
    store.upsert_agent("human", "Human").unwrap();
    store.upsert_agent("cursor", "Cursor").unwrap();
    store.set_team_member("human", true).unwrap();
    store.set_team_member("cursor", true).unwrap();
    store.record_agent_metrics("cursor", 0, 50, 0, 0).unwrap();
    let task = store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            "/tmp/ws",
            "2026-10-20T00:00:00Z",
            None,
            60,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap();
    assert_eq!(task.token_baseline.get("cursor"), Some(&50));
    assert!(!task.token_baseline.contains_key("human"));
    assert!(!task.token_baseline.contains_key("system"));
}

#[test]
fn corrupt_gates_and_budget_json_are_not_decoded_as_defaults() {
    let store = open();
    let task = store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            "/tmp/ws",
            "2026-10-20T00:00:00Z",
            None,
            60,
            vec![GateCriterion::MinTokensWritten { min_tokens: 9 }],
            GateCombinator::All,
            BudgetPolicy {
                credit_allowance: Some(3.0),
                credit_spent: 0.0,
                stop_on_quota_exhausted: true,
            },
            3,
            None,
            "owner",
        )
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE scheduled_tasks SET gate_criteria_json = '{' WHERE id = ?1",
            rusqlite::params![task.id],
        )
        .unwrap();
    assert!(store.get_scheduled_task(&task.id).is_err());
    store
        .conn
        .execute(
            "UPDATE scheduled_tasks SET gate_criteria_json = '[]', budget_json = 'nope' WHERE id = ?1",
            rusqlite::params![task.id],
        )
        .unwrap();
    assert!(store.get_scheduled_task(&task.id).is_err());
}
