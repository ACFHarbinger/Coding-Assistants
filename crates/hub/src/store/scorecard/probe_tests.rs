//! Probe-engine regressions for Team Scorecard T7 (#347), revised per the
//! 2026-10-19 review: assignment-based claimant checks, symmetric quota
//! divergence with comparable-window gating, contract-gated network probe,
//! and atomic deduped persistence. Claimant tests live in
//! `probe_claimant_tests.rs` (500-LoC cap).

use super::*;
use crate::store::scorecard::probe_test_support::*;
use tempfile::tempdir;

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
