//! Probe-engine regressions for Team Scorecard T7 (#347), revised per the
//! 2026-10-19 review: assignment-based claimant checks, symmetric quota
//! divergence with comparable-window gating, contract-gated network probe,
//! and atomic deduped persistence.

use super::*;
use crate::settings::ScorecardVisibility;
use tempfile::tempdir;

#[path = "probe_test_support.rs"]
mod support;
use support::*;

#[test]
fn quota_probe_unknown_until_snapshots_exist() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota.len(), 1);
    assert_eq!(quota[0].state, ProbeState::Unknown);
    assert_eq!(quota[0].subject, "global");

    // The two sources-not-landed probes are honestly unknown too.
    assert_eq!(
        probes_for(&probes, PROBE_GIT_SCOPE_VS_TOUCH_SET)[0].state,
        ProbeState::Unknown
    );
    assert_eq!(
        probes_for(&probes, PROBE_NETWORK_VS_PROVIDER_LOG)[0].state,
        ProbeState::Unknown
    );
}

#[test]
fn quota_probe_ok_when_usage_matches_balance_drop() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let first = snapshot(&store, "deepseek", 12.34, "2026-10-19T10:00:00Z");
    usage_row(&store, "deepseek", 1.50, "2026-10-19T11:00:00Z", None);
    let second = snapshot(&store, "deepseek", 10.34, "2026-10-19T12:00:00Z");

    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota.len(), 1);
    assert_eq!(quota[0].state, ProbeState::Ok, "{:?}", quota[0].reason);
    // Valid windows retain their evidence ids even when they pass.
    assert!(quota[0].source_event_ids.contains(&first));
    assert!(quota[0].source_event_ids.contains(&second));
}

#[test]
fn quota_probe_disagrees_when_spend_exceeds_balance_drop() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    snapshot(&store, "deepseek", 12.34, "2026-10-19T10:00:00Z");
    usage_row(&store, "deepseek", 5.00, "2026-10-19T11:00:00Z", None);
    snapshot(&store, "deepseek", 12.00, "2026-10-19T12:00:00Z");

    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota.len(), 1);
    assert_eq!(quota[0].state, ProbeState::Disagreement);
    let reason = quota[0].reason.as_deref().unwrap_or("");
    assert!(reason.contains("diverge"), "{reason}");
    assert_eq!(quota[0].source_event_ids.len(), 3);
}

#[test]
fn quota_probe_disagrees_symmetrically_when_balance_drop_dwarfs_usage() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    snapshot(&store, "deepseek", 12.34, "2026-10-19T10:00:00Z");
    usage_row(&store, "deepseek", 1.00, "2026-10-19T11:00:00Z", None);
    snapshot(&store, "deepseek", 2.34, "2026-10-19T12:00:00Z");

    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota.len(), 1);
    assert_eq!(quota[0].state, ProbeState::Disagreement);
    let reason = quota[0].reason.as_deref().unwrap_or("");
    assert!(reason.contains("diverge"), "{reason}");
}

#[test]
fn quota_probe_disagrees_when_balance_drops_without_usage() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    snapshot(&store, "deepseek", 12.34, "2026-10-19T10:00:00Z");
    snapshot(&store, "deepseek", 10.34, "2026-10-19T12:00:00Z");

    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota[0].state, ProbeState::Disagreement);
    let reason = quota[0].reason.as_deref().unwrap_or("");
    assert!(reason.contains("no measured usage"), "{reason}");
    assert!(reason.contains("out-of-band"), "{reason}");
}

#[test]
fn quota_probe_top_up_only_windows_are_unknown_not_ok() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let first = snapshot(&store, "deepseek", 12.34, "2026-10-19T10:00:00Z");
    usage_row(&store, "deepseek", 99.00, "2026-10-19T11:00:00Z", None);
    let second = snapshot(&store, "deepseek", 20.00, "2026-10-19T12:00:00Z");

    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota.len(), 1);
    assert_eq!(quota[0].state, ProbeState::Unknown, "{:?}", quota[0].reason);
    assert!(quota[0].reason.as_deref().unwrap_or("").contains("top-up"));
    // Incomparability keeps its evidence.
    assert!(quota[0].source_event_ids.contains(&first));
    assert!(quota[0].source_event_ids.contains(&second));
}

#[test]
fn quota_probe_merges_windows_and_persists_one_row_per_subject() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    snapshot(&store, "deepseek", 12.34, "2026-10-19T10:00:00Z");
    usage_row(&store, "deepseek", 5.00, "2026-10-19T11:00:00Z", None);
    snapshot(&store, "deepseek", 12.00, "2026-10-19T12:00:00Z");
    usage_row(&store, "deepseek", 3.00, "2026-10-19T13:00:00Z", None);
    snapshot(&store, "deepseek", 11.90, "2026-10-19T14:00:00Z");

    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota.len(), 1, "windows merge into one subject result");
    assert_eq!(quota[0].state, ProbeState::Disagreement);

    store.persist_probe_disagreements(&probes).unwrap();
    assert_eq!(disagreement_count(&store), 1);
    store.persist_probe_disagreements(&probes).unwrap();
    assert_eq!(disagreement_count(&store), 1, "second read dedupes");
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
fn network_probe_stays_unknown_without_contract_even_with_sample_rows() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    // Even if rows resembling a producer appear, the probe does not enable
    // itself without the agreed local read contract.
    store
        .record_audit_event(
            &store.data_dir.clone(),
            &store.data_dir.clone(),
            "network.sample",
            r#"{"kind":"network.sample","session_id":"s","destination_ip":"1.2.3.4","destination_port":443}"#,
            None,
        )
        .unwrap();

    let probes = store.compute_probe_candidates().unwrap();
    let network = probes_for(&probes, PROBE_NETWORK_VS_PROVIDER_LOG);
    assert_eq!(network.len(), 1);
    assert_eq!(network[0].state, ProbeState::Unknown);
    assert!(network[0]
        .reason
        .as_deref()
        .unwrap_or("")
        .contains("read contract"));
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
