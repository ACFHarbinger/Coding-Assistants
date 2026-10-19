use super::*;
use crate::commands::commands::schedule::*;
use hub::{BudgetPolicy, GateCombinator, ScheduleStatus, ScheduleTarget};

fn with_home<T>(f: impl FnOnce() -> T) -> T {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-schedule-test-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);
    f()
}

fn args() -> ScheduleCreateArgs {
    ScheduleCreateArgs {
        name: "n".into(),
        instruction: "do it".into(),
        target: ScheduleTarget::Team,
        workspace: std::env::temp_dir().to_string_lossy().to_string(),
        fire_at: "2099-01-01T00:00:00Z".into(),
        repeat_interval_secs: None,
        response_timeout_secs: 3600,
        gate_criteria: vec![],
        gate_combinator: GateCombinator::All,
        budget: BudgetPolicy::default(),
        max_attempts: 3,
        deadline: None,
    }
}

#[test]
fn hub_schedule_create_defaults_created_by_to_human_when_caller_is_none() {
    with_home(|| {
        let task = hub_schedule_create(args(), None).unwrap();
        assert_eq!(task.created_by, "human");
        assert_eq!(task.status, ScheduleStatus::Scheduled);
    });
}

#[test]
fn hub_schedule_create_uses_caller_agent_id_when_present() {
    with_home(|| {
        let task = hub_schedule_create(args(), Some("cursor".into())).unwrap();
        assert_eq!(task.created_by, "cursor");
    });
}

#[test]
fn hub_schedule_list_get_and_cancel_round_trip() {
    with_home(|| {
        let created = hub_schedule_create(args(), None).unwrap();

        let listed = hub_schedule_list().unwrap();
        assert!(listed.iter().any(|t| t.id == created.id));

        let fetched = hub_schedule_get(created.id.clone()).unwrap();
        assert_eq!(fetched.map(|t| t.id), Some(created.id.clone()));

        let cancelled = hub_schedule_cancel(created.id.clone()).unwrap();
        assert_eq!(cancelled.status, ScheduleStatus::Cancelled);
    });
}

#[test]
fn hub_schedule_create_rejects_a_bad_payload_the_same_way_the_store_does() {
    with_home(|| {
        let mut bad = args();
        bad.max_attempts = 0;
        assert!(hub_schedule_create(bad, None).is_err());
    });
}
