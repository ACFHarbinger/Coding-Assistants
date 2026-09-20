//! End-to-end runtime budget policy (platform.md P10). Affine typing is
//! intentionally not used: the gate is a runtime decision + durable handoff.

use super::super::*;
use std::fs;
use tempfile::tempdir;

fn gate(store: &HubStore, agent: &str, amount: f64) -> ProviderCallGate {
    store
        .gate_provider_call(
            agent,
            amount,
            Some("task-p10"),
            "Finish the runtime budget gate.",
            "Store methods landed.",
            "Orchestrator wiring.",
            Some("human"),
        )
        .unwrap()
}

#[test]
fn p10_unmetered_when_no_budget_is_configured() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    assert!(matches!(
        gate(&store, "claude", 1.0),
        ProviderCallGate::Unmetered
    ));
    store
        .request_wake("claude", Some("unmetered"), None, true)
        .unwrap();
}

#[test]
fn p10_reserve_under_limit_then_exhaustion_writes_handoff_and_blocks() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_agent_budget("claude", 2.0).unwrap();

    match gate(&store, "claude", 1.0) {
        ProviderCallGate::Reserved { status } => {
            assert!(!status.paused);
            assert_eq!(status.spent_units, 1.0);
        }
        other => panic!("expected reserved, got {other:?}"),
    }

    // Remaining 1.0; reserving 1.5 exceeds without an exact fill, so the
    // gate itself writes the C6 handoff and forbids the call.
    match gate(&store, "claude", 1.5) {
        ProviderCallGate::Stopped { status, handoff } => {
            assert!(status.paused);
            let handoff = handoff.expect("first over-limit writes a handoff");
            assert!(handoff.summary_path.exists());
            let summary = fs::read_to_string(&handoff.summary_path).unwrap();
            assert!(summary.contains("Finish the runtime budget gate."));
            let message = store
                .get_message(&handoff.handoff_message_id)
                .unwrap()
                .unwrap();
            assert_eq!(message.kind, "handoff");
            assert_eq!(message.to_agent, "human");
        }
        other => panic!("expected stopped with handoff, got {other:?}"),
    }

    match gate(&store, "claude", 1.0) {
        ProviderCallGate::Stopped { handoff, .. } => {
            assert!(handoff.is_none(), "already paused must not spam handoffs");
        }
        other => panic!("expected stopped without handoff, got {other:?}"),
    }

    let err = store
        .request_wake("claude", Some("blocked"), None, true)
        .unwrap_err();
    assert!(err.to_string().contains("budget-paused"));

    let resumed = store.resume_agent("claude").unwrap();
    assert!(!resumed.paused);
    match gate(&store, "claude", 0.5) {
        ProviderCallGate::Reserved { status } => {
            assert!(!status.paused);
            assert_eq!(status.spent_units, 1.5);
        }
        other => panic!("expected reserved after resume, got {other:?}"),
    }
}

#[test]
fn p10_exact_fill_is_the_last_allowed_call() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_agent_budget("claude", 1.0).unwrap();

    match gate(&store, "claude", 1.0) {
        ProviderCallGate::Reserved { status } => {
            assert!(status.paused);
            assert_eq!(status.spent_units, 1.0);
        }
        other => panic!("expected last-unit reserved, got {other:?}"),
    }

    let outcome = store
        .pause_for_budget(
            "claude",
            Some("task-p10"),
            "Finish the runtime budget gate.",
            "Last allowed call completed.",
            "No further provider calls.",
            Some("human"),
        )
        .unwrap();
    assert!(outcome.summary_path.exists());

    match gate(&store, "claude", 1.0) {
        ProviderCallGate::Stopped { handoff, .. } => {
            assert!(handoff.is_none());
        }
        other => panic!("expected stopped after last-unit handoff, got {other:?}"),
    }
}

#[test]
fn p10_shutdown_pauses_budget_so_calls_and_wakes_stop() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_agent_budget("claude", 10.0).unwrap();
    store.try_consume_budget("claude", 1.0).unwrap();

    let outcome = store
        .record_shutdown(
            "claude",
            Some("task-p10"),
            "Cancelled mid-run",
            "owner cancelled the active provider call",
            Some("grok"),
        )
        .unwrap();
    assert!(outcome.summary_path.exists());
    assert!(store.get_budget("claude").unwrap().unwrap().paused);

    match gate(&store, "claude", 1.0) {
        ProviderCallGate::Stopped { handoff, .. } => assert!(handoff.is_none()),
        other => panic!("expected stopped after shutdown, got {other:?}"),
    }
    let err = store
        .request_wake("claude", Some("after-shutdown"), None, true)
        .unwrap_err();
    assert!(err.to_string().contains("budget-paused"));
}

#[test]
fn p10_shutdown_without_a_budget_still_writes_handoff() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let outcome = store
        .record_shutdown("claude", None, "No budget configured", "cancelled", None)
        .unwrap();
    assert!(outcome.summary_path.exists());
    assert!(store.get_budget("claude").unwrap().is_none());
    assert!(matches!(
        gate(&store, "claude", 1.0),
        ProviderCallGate::Unmetered
    ));
}

#[test]
fn p10_rejects_non_positive_gate_amount() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_agent_budget("claude", 5.0).unwrap();
    assert!(store
        .gate_provider_call("claude", 0.0, None, "x", "y", "z", None)
        .is_err());
    assert!(!store.get_budget("claude").unwrap().unwrap().paused);
}
