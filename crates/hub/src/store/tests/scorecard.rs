use super::super::*;
use crate::settings::ScorecardVisibility;
use serde_json::json;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn scorecard_returns_expected_structure_on_fresh_store() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let scorecard = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard must succeed");

    assert_eq!(scorecard.aggregation_version, 1);
    // Seeded agents exist
    assert!(!scorecard.agents.is_empty());
    // Harness coverage rows include unregistered defaults
    assert!(scorecard
        .harnesses
        .iter()
        .any(|h| h.coverage == HarnessCoverage::Unregistered));
}

#[test]
fn scorecard_owner_only_hides_data_from_agents() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    // Non-human agent under OwnerOnly
    let view = store
        .get_scorecard(Some("gemini"), ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");
    assert!(view.agents.is_empty());
    assert!(view.harnesses.is_empty());
    assert!(view.providers.is_empty());

    // Owner (None or "human") sees data
    let owner_view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");
    assert!(!owner_view.agents.is_empty());

    let human_view = store
        .get_scorecard(Some("human"), ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");
    assert!(!human_view.agents.is_empty());
}

#[test]
fn scorecard_agents_own_card_returns_own_row_only() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let view = store
        .get_scorecard(Some("gemini"), ScorecardVisibility::AgentsOwnCard)
        .expect("get_scorecard succeeds");

    assert_eq!(view.agents.len(), 1);
    assert_eq!(view.agents[0].agent_id, "gemini");
    // Harnesses only include linked to this agent
    for h in &view.harnesses {
        assert_eq!(h.linked_agent_id.as_deref(), Some("gemini"));
    }
    assert!(view.providers.is_empty());
}

#[test]
fn scorecard_all_agents_allows_any_caller() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let view = store
        .get_scorecard(Some("gemini"), ScorecardVisibility::AllAgents)
        .expect("get_scorecard succeeds");

    assert!(!view.agents.is_empty());
    assert!(view.agents.iter().any(|a| a.agent_id == "claude"));
}

#[test]
fn scorecard_sorts_open_security_before_pending_decision_before_cost() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path();

    // 1. Agent "claude" has high measured cost (1000 tokens), 0 security events
    store
        .record_audit_event(
            root,
            Path::new("src/lib.rs"),
            "provider.usage",
            &json!({
                "provider": "anthropic",
                "actor_agent": "claude",
                "prompt_tokens": 500,
                "completion_tokens": 500,
                "cost": 0.05,
                "currency": "measured"
            })
            .to_string(),
            None,
        )
        .unwrap();

    // 2. Agent "grok" has a pending gate decision (oldest)
    store
        .conn
        .execute(
            "INSERT INTO pending_gate_approvals(id, subject, from_agent, to_agents_json, body, reason, status, created_at) \
             VALUES ('gate-1', 'test', 'grok', '[\"human\"]', 'body', 'reason', 'pending', '2026-10-19T10:00:00Z')",
            [],
        )
        .unwrap();

    // 3. Agent "gemini" has an open security event (denied tool)
    store
        .record_audit_event(
            root,
            Path::new("tool.rs"),
            "os_tool.denied",
            &json!({
                "agent": "gemini",
                "task_id": "task-1"
            })
            .to_string(),
            None,
        )
        .unwrap();

    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .unwrap();

    let gemini_pos = view
        .agents
        .iter()
        .position(|a| a.agent_id == "gemini")
        .unwrap();
    let grok_pos = view
        .agents
        .iter()
        .position(|a| a.agent_id == "grok")
        .unwrap();
    let claude_pos = view
        .agents
        .iter()
        .position(|a| a.agent_id == "claude")
        .unwrap();

    // Gemini (open security event) must come before Grok (pending decision)
    assert!(
        gemini_pos < grok_pos,
        "gemini ({gemini_pos}) should be before grok ({grok_pos})"
    );
    // Grok (pending decision) must come before Claude (only cost)
    assert!(
        grok_pos < claude_pos,
        "grok ({grok_pos}) should be before claude ({claude_pos})"
    );

    // Verify provenance
    let claude_row = &view.agents[claude_pos];
    assert_eq!(claude_row.provenance, "measured");
    assert_eq!(claude_row.cost.provenance, FigureProvenance::Measured);

    let grok_row = &view.agents[grok_pos];
    assert_eq!(grok_row.provenance, "unknown");
    assert_eq!(grok_row.cost.provenance, FigureProvenance::Unknown);
}

#[test]
fn harness_coverage_reflects_registration_mode() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    store
        .conn
        .execute(
            "INSERT INTO harness_session_registrations(harness, workspace, disk_session_id, registered_at, mode, state) \
             VALUES ('claude', '/test/ws', 'sess-1', '2026-10-19T12:00:00Z', 'managed', 'ready')",
            [],
        )
        .unwrap();

    store
        .conn
        .execute(
            "INSERT INTO harness_session_registrations(harness, workspace, disk_session_id, registered_at, mode, state) \
             VALUES ('gemini', '/test/ws', 'sess-2', '2026-10-19T12:00:00Z', 'observed', 'ready')",
            [],
        )
        .unwrap();

    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .unwrap();

    let claude_harness = view
        .harnesses
        .iter()
        .find(|h| h.harness == "claude" && h.workspace == "/test/ws")
        .expect("claude harness row");
    assert_eq!(claude_harness.coverage, HarnessCoverage::Managed);

    let gemini_harness = view
        .harnesses
        .iter()
        .find(|h| h.harness == "gemini" && h.workspace == "/test/ws")
        .expect("gemini harness row");
    assert_eq!(gemini_harness.coverage, HarnessCoverage::Observed);

    let grok_unreg = view
        .harnesses
        .iter()
        .find(|h| h.harness == "grok" && h.workspace == "/test/ws")
        .expect("grok unregistered harness row");
    assert_eq!(grok_unreg.coverage, HarnessCoverage::Unregistered);
}
