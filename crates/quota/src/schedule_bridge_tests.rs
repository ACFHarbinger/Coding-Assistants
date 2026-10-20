//! Tests for [`super::tick_with_quota_guard`], moved verbatim (renamed
//! from `tick_in_app`) from `src-tauri`'s S6 schedule-poller suite so the
//! pause-semantics coverage follows the extracted bridge.

use super::tick_with_quota_guard;
use crate::codex::{ProviderQuota, ProviderQuotaWindow};
use hub::{BudgetPolicy, GateCombinator, HubStore, ScheduleStatus, ScheduleTarget, ScheduledTask};
use tempfile::tempdir;

fn quota(agent: &str, status: &str, remaining: Option<i32>, detail: Option<&str>) -> ProviderQuota {
    ProviderQuota {
        agent_id: agent.into(),
        provider: agent.into(),
        harness_title: agent.into(),
        status: status.into(),
        detail: detail.map(str::to_string),
        windows: remaining
            .map(|percent| {
                vec![ProviderQuotaWindow {
                    label: "5h".into(),
                    family: None,
                    used_percent: 100 - percent.max(0),
                    remaining_percent: percent,
                    resets_at: None,
                    window_minutes: Some(300),
                }]
            })
            .unwrap_or_default(),
        fetched_at: 0,
        balance: None,
        balance_info: None,
        local_usage: None,
    }
}

fn create_due(store: &HubStore, workspace: &str, stop_on_quota: bool) -> ScheduledTask {
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

fn with_store(f: impl FnOnce(&HubStore, &str)) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    f(&store, &workspace.to_string_lossy());
}

#[test]
fn exhausted_quota_pauses_and_does_not_dispatch() {
    with_store(|store, workspace| {
        let task = create_due(store, workspace, true);
        let summary = tick_with_quota_guard(store, "2020-01-01T00:00:01Z", |_| {
            quota("cursor", "ok", Some(0), None)
        })
        .unwrap();
        assert!(summary.outcomes.iter().any(|o| o.action == "paused"));
        assert!(!summary.outcomes.iter().any(|o| o.action == "dispatched"));
        let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
        assert_eq!(fetched.status, ScheduleStatus::Paused);
        assert_eq!(fetched.attempts, 0);
    });
}

#[test]
fn available_quota_still_uses_shared_tick() {
    with_store(|store, workspace| {
        let task = create_due(store, workspace, true);
        let summary = tick_with_quota_guard(store, "2020-01-01T00:00:01Z", |_| {
            quota("cursor", "ok", Some(42), None)
        })
        .unwrap();
        assert!(summary.outcomes.iter().any(|o| o.action == "dispatched"));
        let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
        assert_eq!(fetched.status, ScheduleStatus::Active);
        assert_eq!(fetched.attempts, 1);
        assert_eq!(fetched.id, task.id);
    });
}

#[test]
fn unavailable_quota_does_not_invent_a_stop() {
    with_store(|store, workspace| {
        let task = create_due(store, workspace, true);
        let summary = tick_with_quota_guard(store, "2020-01-01T00:00:01Z", |_| {
            quota(
                "cursor",
                "unavailable",
                None,
                Some("no machine-readable usage surface"),
            )
        })
        .unwrap();
        assert!(!summary.outcomes.iter().any(|o| o.action == "paused"));
        assert!(summary.outcomes.iter().any(|o| o.action == "dispatched"));
        let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
        assert_eq!(fetched.status, ScheduleStatus::Active);
        assert_eq!(fetched.attempts, 1);
    });
}

#[test]
fn stop_on_quota_exhausted_false_ignores_an_empty_window() {
    with_store(|store, workspace| {
        let task = create_due(store, workspace, false);
        let summary = tick_with_quota_guard(store, "2020-01-01T00:00:01Z", |_| {
            quota("cursor", "ok", Some(0), None)
        })
        .unwrap();
        assert!(!summary.outcomes.iter().any(|o| o.action == "paused"));
        assert!(summary.outcomes.iter().any(|o| o.action == "dispatched"));
        let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
        assert_eq!(fetched.status, ScheduleStatus::Active);
    });
}

#[test]
fn concurrent_headless_tick_shares_one_attempt() {
    let dir = tempdir().unwrap();
    let home = dir.path().to_path_buf();
    let workspace = home.join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let setup = HubStore::open(&home).unwrap();
    let task = create_due(&setup, &workspace, true);
    let id = task.id.clone();
    drop(setup);

    let a = HubStore::open(&home).unwrap();
    let b = HubStore::open(&home).unwrap();
    std::thread::scope(|scope| {
        scope.spawn(move || {
            tick_with_quota_guard(&a, "2020-01-01T00:00:01Z", |_| {
                quota("cursor", "ok", Some(50), None)
            })
            .unwrap()
        });
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
fn exhausted_bridge_then_headless_tick_does_not_dispatch() {
    with_store(|store, workspace| {
        let task = create_due(store, workspace, true);
        tick_with_quota_guard(store, "2020-01-01T00:00:01Z", |_| {
            quota("cursor", "ok", Some(0), None)
        })
        .unwrap();
        let summary = store.tick_scheduled_tasks("2020-01-01T00:00:02Z").unwrap();
        assert!(summary.outcomes.is_empty() || summary.outcomes.iter().all(|o| o.action == "noop"));
        let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
        assert_eq!(fetched.status, ScheduleStatus::Paused);
        assert_eq!(fetched.attempts, 0);
    });
}
