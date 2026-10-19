//! Probe-engine regressions for Team Scorecard T7 (#347).

use super::*;
use crate::settings::ScorecardVisibility;
use crate::store::{
    HubStore, ProbeState, ProviderUsage, QuotaSnapshot, ScorecardProbe, SliceStamp,
};
use rusqlite::params;
use tempfile::tempdir;

fn set_observed_at(store: &HubStore, id: &str, at: &str) {
    store
        .conn
        .execute(
            "UPDATE audit_events SET observed_at = ?1 WHERE id = ?2",
            params![at, id],
        )
        .unwrap();
}

fn snapshot(store: &HubStore, provider: &str, balance: f64, at: &str) -> String {
    let event = store
        .record_quota_snapshot(&QuotaSnapshot {
            provider: provider.into(),
            balance: Some(balance),
            used_percent: None,
            window_label: None,
        })
        .unwrap()
        .unwrap();
    set_observed_at(store, &event.id, at);
    event.id
}

fn usage_row(store: &HubStore, provider: &str, cost: f64, at: &str, task: Option<&str>) -> String {
    let usage = ProviderUsage {
        provider: provider.into(),
        actor_agent: "cursor".into(),
        prompt_tokens: Some(1),
        completion_tokens: Some(1),
        cache_tokens: None,
        cost: Some(cost),
        via_gateway: None,
        model: None,
        stamp: SliceStamp::registered(task.map(str::to_string), None, Some("cursor".into())),
    };
    let event = store.record_provider_usage(&usage).unwrap().unwrap();
    set_observed_at(store, &event.id, at);
    event.id
}

fn probes_for<'a>(probes: &'a [ScorecardProbe], id: &str) -> Vec<&'a ScorecardProbe> {
    probes.iter().filter(|p| p.probe == id).collect()
}

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
    snapshot(&store, "deepseek", 12.34, "2026-10-19T10:00:00Z");
    usage_row(&store, "deepseek", 1.50, "2026-10-19T11:00:00Z", None);
    snapshot(&store, "deepseek", 10.34, "2026-10-19T12:00:00Z");

    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota.len(), 1);
    assert_eq!(quota[0].state, ProbeState::Ok, "{:?}", quota[0].reason);
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
    assert!(reason.contains("exceeds balance decrease"), "{reason}");
    assert_eq!(quota[0].source_event_ids.len(), 3);
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
    assert!(quota[0]
        .reason
        .as_deref()
        .unwrap_or("")
        .contains("no measured usage"));
}

#[test]
fn quota_probe_skips_top_up_windows() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    snapshot(&store, "deepseek", 12.34, "2026-10-19T10:00:00Z");
    usage_row(&store, "deepseek", 99.00, "2026-10-19T11:00:00Z", None);
    snapshot(&store, "deepseek", 20.00, "2026-10-19T12:00:00Z");

    let probes = store.compute_probe_candidates().unwrap();
    let quota = probes_for(&probes, PROBE_QUOTA_VS_COST);
    assert_eq!(quota.len(), 1);
    assert_eq!(quota[0].state, ProbeState::Ok, "{:?}", quota[0].reason);
}

#[test]
fn claimant_probe_flags_divergent_agents_and_dedupes_persisted_rows() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .record_tagged_send_audit("cursor", "slice A", None, Some("task-x"), None, 1)
        .unwrap();
    store
        .record_tagged_send_audit("grok", "slice B", None, Some("task-x"), None, 1)
        .unwrap();
    // Same agent on a second slice must not fire.
    store
        .record_tagged_send_audit("cursor", "slice C", None, Some("task-y"), None, 1)
        .unwrap();

    let probes = store.compute_probe_candidates().unwrap();
    let claimant = probes_for(&probes, PROBE_CLAIMANT_CONSISTENCY);
    assert_eq!(claimant.len(), 1);
    assert_eq!(claimant[0].subject, "task-x");
    assert_eq!(claimant[0].state, ProbeState::Disagreement);
    assert_eq!(claimant[0].linked_agents, vec!["cursor", "grok"]);

    store.persist_probe_disagreements(&probes).unwrap();
    let count: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM audit_events WHERE operation = 'scorecard.disagreement'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Recompute + persist again: the unresolved row dedupes, no duplicate.
    let again = store.compute_probe_candidates().unwrap();
    store.persist_probe_disagreements(&again).unwrap();
    let count: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM audit_events WHERE operation = 'scorecard.disagreement'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Badge counts name both claimants; unrelated agents stay clean.
    let badges = store.unresolved_disagreement_counts().unwrap();
    assert_eq!(badges.get("cursor").copied(), Some(1));
    assert_eq!(badges.get("grok").copied(), Some(1));
    assert_eq!(badges.get("chat").copied(), None);
}

#[test]
fn scorecard_serves_owner_probes_and_badges_but_not_to_non_owner() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.upsert_agent("cursor", "Cursor").unwrap();
    store.upsert_agent("grok", "Grok").unwrap();
    store
        .record_tagged_send_audit("cursor", "slice A", None, Some("task-x"), None, 1)
        .unwrap();
    store
        .record_tagged_send_audit("grok", "slice B", None, Some("task-x"), None, 1)
        .unwrap();

    let owner = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .unwrap();
    assert!(!owner.probes.is_empty());
    let cursor_row = owner
        .agents
        .iter()
        .find(|a| a.agent_id == "cursor")
        .expect("cursor row");
    assert_eq!(cursor_row.unresolved_disagreements, 1);

    // Non-owner under all_agents: cards + badges visible (that is the mode's
    // meaning), but the probe list and reviewer signature stay owner-only.
    let agent_view = store
        .get_scorecard(Some("cursor"), ScorecardVisibility::AllAgents)
        .unwrap();
    assert!(agent_view.probes.is_empty());
    assert!(agent_view.reviewers.is_empty());
    // The roster is seeded, so every agent card is present under all_agents.
    let cursor_card = agent_view
        .agents
        .iter()
        .find(|a| a.agent_id == "cursor")
        .expect("cursor card");
    assert_eq!(cursor_card.unresolved_disagreements, 1);
}

#[test]
fn read_only_store_computes_without_persisting() {
    let dir = tempdir().unwrap();
    let path = dir.path().to_path_buf();
    {
        let store = HubStore::open(&path).unwrap();
        store
            .record_tagged_send_audit("cursor", "slice A", None, Some("task-x"), None, 1)
            .unwrap();
        store
            .record_tagged_send_audit("grok", "slice B", None, Some("task-x"), None, 1)
            .unwrap();
    }
    let store = HubStore::open_existing_read_only(&path).unwrap();
    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .unwrap();
    assert!(view
        .probes
        .iter()
        .any(|p| p.probe == PROBE_CLAIMANT_CONSISTENCY && p.state == ProbeState::Disagreement));
    let count: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM audit_events WHERE operation = 'scorecard.disagreement'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}
