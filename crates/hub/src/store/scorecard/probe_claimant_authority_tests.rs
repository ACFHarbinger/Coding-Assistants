//! Time-valid authority regressions for the claimant probe (T7 / #347).
//! Split from `probe_claimant_tests.rs` to stay under the 500-line cap.

use super::*;
use crate::store::scorecard::probe_test_support::*;
use tempfile::tempdir;

#[test]
fn reassignment_does_not_flag_the_agent_who_already_had_a_handoff() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    // Cursor's handoff predates Cursor's work. Grok is the only name on
    // the current task and has no handoff. Cursor must not be flagged;
    // Grok's current enrolment is not authority for the 12:00 action.
    task_message(&store, "msg-1", "cursor", "task-x", "2026-10-19T10:00:00Z");
    task_with_agents(&store, "task-x", &["grok"]);
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        Some("task-x"),
        None,
        "cursor",
    );
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T12:00:00Z",
        Some("task-x"),
        None,
        "grok",
    );
    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant[0].state, ProbeState::Unknown);
    assert_eq!(claimant[0].linked_agents, vec!["grok"]);
}

#[test]
fn ordinary_message_and_chat_wake_do_not_authorize() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    task_with_agents(&store, "task-x", &["grok"]);
    chat_message(&store, "msg-1", "grok", "task-x", "2026-10-19T10:00:00Z");
    store
        .conn
        .execute(
            "INSERT INTO wake_requests (id, target_agent, message_id, reason, status, requires_human_gate, created_at) \
             VALUES ('wake-1', 'grok', 'msg-1', 'ping', 'pending', 1, '2026-10-19T10:05:00Z')",
            [],
        )
        .unwrap();
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        Some("task-x"),
        None,
        "grok",
    );
    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant[0].state, ProbeState::Unknown);
    assert_eq!(claimant[0].linked_agents, vec!["grok"]);
}

#[test]
fn wake_linked_to_another_agents_handoff_does_not_authorize() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    // The handoff row names Cursor; the dispatch wake names Grok. Grok's
    // work has no handoff to Grok. A wake is not a reassignment record.
    task_message(&store, "msg-1", "cursor", "task-x", "2026-10-19T10:00:00Z");
    store
        .conn
        .execute(
            "INSERT INTO wake_requests (id, target_agent, message_id, reason, status, requires_human_gate, created_at) \
             VALUES ('wake-1', 'grok', 'msg-1', 'task task-x', 'pending', 1, '2026-10-19T10:05:00Z')",
            [],
        )
        .unwrap();
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        Some("task-x"),
        None,
        "grok",
    );
    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(
        claimant[0].state,
        ProbeState::Unknown,
        "{:?}",
        claimant[0].reason
    );
}

#[test]
fn session_enrolment_after_the_action_does_not_approve() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        None,
        Some("sess-1"),
        "cursor",
    );
    session_with_members_at(&store, "sess-1", &["cursor"], "2026-10-19T12:00:00Z");
    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant[0].subject, "session:sess-1");
    assert_eq!(claimant[0].state, ProbeState::Unknown);
    assert_eq!(claimant[0].linked_agents, vec!["cursor"]);
}

#[test]
fn later_wake_cannot_backdate_authority_for_a_different_target() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    task_message(&store, "msg-1", "cursor", "task-x", "2026-10-19T10:00:00Z");
    store.conn.execute(
        "INSERT INTO wake_requests (id, target_agent, message_id, reason, status, requires_human_gate, created_at) VALUES ('wake-1', 'grok', 'msg-1', 'task task-x', 'pending', 1, '2026-10-19T12:00:00Z')",
        [],
    ).unwrap();
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        Some("task-x"),
        None,
        "grok",
    );
    let probes = store.compute_probe_candidates().unwrap();
    assert_eq!(
        probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY)[0].state,
        ProbeState::Unknown
    );
}
