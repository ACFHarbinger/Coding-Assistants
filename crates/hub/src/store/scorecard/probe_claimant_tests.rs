//! Claimant-probe regressions (T7 / #347): assignment-grounded approval,
//! reassignment safety, namespace separation, and badge attribution.

use super::*;
use crate::settings::ScorecardVisibility;
use crate::store::scorecard::probe_test_support::*;
use tempfile::tempdir;

#[test]
fn claimant_probe_evaluates_a_single_actor() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    task_with_agents(&store, "task-one", &["cursor"]);
    usage_full(
        &store,
        "openai",
        0.1,
        "2026-10-19T10:00:00Z",
        Some("task-one"),
        None,
        "grok",
    );
    let probes = store.compute_probe_candidates().unwrap();
    let result = probes
        .iter()
        .find(|p| p.subject == "task:task-one")
        .unwrap();
    assert_eq!(result.state, ProbeState::Disagreement);
    assert_eq!(result.linked_agents, vec!["grok"]);
}

#[test]
fn claimant_probe_unknown_without_assignment_record() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    // Two action claims on one task, but the Hub has no task record —
    // plurality is not an ownership verdict.
    usage_row(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        Some("task-x"),
    );
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:05:00Z",
        Some("task-x"),
        None,
        "grok",
    );

    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant.len(), 1);
    assert_eq!(claimant[0].subject, "task:task-x");
    assert_eq!(claimant[0].state, ProbeState::Unknown);
    assert!(claimant[0]
        .reason
        .as_deref()
        .unwrap_or("")
        .contains("not an ownership verdict"));
}

#[test]
fn claimant_probe_ok_when_collaboration_is_assigned() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    task_with_agents(&store, "task-x", &["cursor", "grok"]);
    usage_row(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        Some("task-x"),
    );
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:05:00Z",
        Some("task-x"),
        None,
        "grok",
    );

    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant.len(), 1);
    assert_eq!(
        claimant[0].state,
        ProbeState::Ok,
        "{:?}",
        claimant[0].reason
    );
}

#[test]
fn claimant_probe_flags_claimant_outside_assignment() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    task_with_agents(&store, "task-x", &["cursor"]);
    usage_row(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        Some("task-x"),
    );
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:05:00Z",
        Some("task-x"),
        None,
        "grok",
    );

    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant.len(), 1);
    assert_eq!(claimant[0].state, ProbeState::Disagreement);
    assert_eq!(claimant[0].linked_agents, vec!["grok"]);
    let reason = claimant[0].reason.as_deref().unwrap_or("");
    assert!(reason.contains("unapproved claimant"), "{reason}");
    assert!(reason.contains("cursor"), "{reason}");
}

#[test]
fn claimant_probe_ignores_communication_and_separates_namespaces() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    // Communication rows from two agents on one task: legitimate
    // collaboration, never claim material, so no probe at all.
    store
        .record_tagged_send_audit("cursor", "slice A", None, Some("task-x"), None, 1)
        .unwrap();
    store
        .record_tagged_send_audit("grok", "slice B", None, Some("task-x"), None, 1)
        .unwrap();
    let probes = store.compute_probe_candidates().unwrap();
    assert!(probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY).is_empty());

    // The same id string in the session namespace is a DIFFERENT subject.
    // Two claimants and no member record → unknown, named by namespace.
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        None,
        Some("shared-id"),
        "cursor",
    );
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:05:00Z",
        None,
        Some("shared-id"),
        "grok",
    );
    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant.len(), 1);
    assert_eq!(claimant[0].subject, "session:shared-id");
    assert_eq!(claimant[0].state, ProbeState::Unknown);

    // Members make the same session an approved collaboration — without the
    // task-namespace rows leaking into the verdict.
    session_with_members(&store, "shared-id", &["cursor", "grok"]);
    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant.len(), 1);
    assert_eq!(claimant[0].subject, "session:shared-id");
    assert_eq!(
        claimant[0].state,
        ProbeState::Ok,
        "{:?}",
        claimant[0].reason
    );
}

#[test]
fn disagreement_badges_name_only_unapproved_claimants() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.upsert_agent("cursor", "Cursor").unwrap();
    store.upsert_agent("grok", "Grok").unwrap();
    task_with_agents(&store, "task-x", &["cursor"]);
    usage_row(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:00:00Z",
        Some("task-x"),
    );
    usage_full(
        &store,
        "openai",
        0.01,
        "2026-10-19T11:05:00Z",
        Some("task-x"),
        None,
        "grok",
    );

    let owner = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .unwrap();
    let grok_row = owner.agents.iter().find(|a| a.agent_id == "grok").unwrap();
    assert_eq!(grok_row.unresolved_disagreements, 1);
    let cursor_row = owner
        .agents
        .iter()
        .find(|a| a.agent_id == "cursor")
        .unwrap();
    assert_eq!(cursor_row.unresolved_disagreements, 0);

    // Non-owner under all_agents: own card + own badge, no probe list.
    let agent_view = store
        .get_scorecard(Some("grok"), ScorecardVisibility::AllAgents)
        .unwrap();
    assert!(agent_view.probes.is_empty());
    assert!(agent_view.reviewers.is_empty());
    let grok_card = agent_view
        .agents
        .iter()
        .find(|a| a.agent_id == "grok")
        .unwrap();
    assert_eq!(grok_card.unresolved_disagreements, 1);
}

#[test]
fn read_only_store_computes_without_persisting() {
    let dir = tempdir().unwrap();
    let path = dir.path().to_path_buf();
    {
        let store = HubStore::open(&path).unwrap();
        task_with_agents(&store, "task-x", &["cursor"]);
        usage_row(
            &store,
            "openai",
            0.01,
            "2026-10-19T11:00:00Z",
            Some("task-x"),
        );
        usage_full(
            &store,
            "openai",
            0.01,
            "2026-10-19T11:05:00Z",
            Some("task-x"),
            None,
            "grok",
        );
    }
    let store = HubStore::open_existing_read_only(&path).unwrap();
    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .unwrap();
    assert!(view
        .probes
        .iter()
        .any(|p| p.probe == PROBE_CLAIMANT_CONSISTENCY && p.state == ProbeState::Disagreement));
    assert_eq!(disagreement_count(&store), 0);
}

#[test]
fn claimant_probe_reassignment_keeps_historical_work_approved() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    // Cursor was handed the task at 10:00 and worked it at 11:00. The task
    // has since been reassigned: the current record names only Grok, whose
    // work lands at 12:00. Neither claimant may be flagged.
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
    assert_eq!(claimant.len(), 1);
    assert_eq!(
        claimant[0].state,
        ProbeState::Ok,
        "{:?}",
        claimant[0].reason
    );
}

#[test]
fn claimant_probe_handoff_after_action_does_not_approve() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    // Grok acted at 11:00 but was only handed the task at 12:00, and the
    // current assignment names Cursor. The late handoff is not retroactive
    // approval: Grok disagrees.
    task_message(&store, "msg-1", "grok", "task-x", "2026-10-19T12:00:00Z");
    task_with_agents(&store, "task-x", &["cursor"]);
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
    assert_eq!(claimant.len(), 1);
    assert_eq!(claimant[0].state, ProbeState::Disagreement);
    assert_eq!(claimant[0].linked_agents, vec!["grok"]);
}

#[test]
fn claimant_probe_handoff_evidence_is_per_agent() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    // Only Cursor was handed the task. Grok claiming it is still
    // unapproved — another agent's handoff never transfers.
    task_message(&store, "msg-1", "cursor", "task-x", "2026-10-19T10:00:00Z");
    task_with_agents(&store, "task-x", &["cursor"]);
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
        "2026-10-19T11:05:00Z",
        Some("task-x"),
        None,
        "grok",
    );

    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant.len(), 1);
    assert_eq!(claimant[0].state, ProbeState::Disagreement);
    assert_eq!(claimant[0].linked_agents, vec!["grok"]);
}
