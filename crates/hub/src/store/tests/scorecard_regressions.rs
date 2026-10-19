use super::super::*;
use crate::settings::ScorecardVisibility;
use serde_json::json;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn measured_label_without_numeric_cost_is_unknown() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .record_audit_event(
            dir.path(),
            Path::new("usage"),
            "provider.usage",
            &json!({"provider":"test", "currency":"measured", "actor_agent":"claude"}).to_string(),
            None,
        )
        .unwrap();
    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .unwrap();
    let provider = view
        .providers
        .iter()
        .find(|p| p.provider == "test")
        .unwrap();
    assert_eq!(provider.cost.value, None);
    assert_eq!(provider.cost.provenance, FigureProvenance::Unknown);
    assert_eq!(
        provider.cost.coverage.as_ref().unwrap().measured_requests,
        0
    );
    let agent = view.agents.iter().find(|a| a.agent_id == "claude").unwrap();
    assert_eq!(agent.cost.value, None);
}

#[test]
fn token_only_plus_priced_plus_measured_zero_usage() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path();

    // 1. Token-only usage for claude (no cost field)
    store
        .record_audit_event(
            root,
            Path::new("src/lib.rs"),
            "provider.usage",
            &json!({
                "provider": "anthropic",
                "actor_agent": "claude",
                "prompt_tokens": 600,
                "completion_tokens": 400
            })
            .to_string(),
            None,
        )
        .unwrap();

    // 2. Priced usage for claude (cost only, no tokens)
    store
        .record_audit_event(
            root,
            Path::new("src/lib.rs"),
            "provider.usage",
            &json!({
                "provider": "anthropic",
                "actor_agent": "claude",
                "cost": 0.05,
                "currency": "measured"
            })
            .to_string(),
            None,
        )
        .unwrap();

    // 3. Measured zero cost for a local model
    store
        .record_audit_event(
            root,
            Path::new("src/local.rs"),
            "provider.usage",
            &json!({
                "provider": "local-llm",
                "actor_agent": "gemini",
                "prompt_tokens": 200,
                "completion_tokens": 100,
                "cost": 0.0,
                "currency": "measured"
            })
            .to_string(),
            None,
        )
        .unwrap();

    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");

    // Claude: tokens = 1000 measured, cost = 0.05 measured. Never mixed to 1000.05!
    let claude = view
        .agents
        .iter()
        .find(|a| a.agent_id == "claude")
        .expect("claude row");
    assert_eq!(claude.tokens.provenance, FigureProvenance::Measured);
    assert_eq!(claude.tokens.value, Some(1000.0));
    assert_eq!(claude.tokens.unit, "tokens");
    assert_eq!(claude.cost.provenance, FigureProvenance::Measured);
    assert_eq!(claude.cost.value, Some(0.05));
    assert_eq!(claude.cost.unit, "currency");
    let coverage = claude.cost.coverage.as_ref().unwrap();
    assert_eq!(
        (coverage.measured_requests, coverage.total_requests),
        (1, 2)
    );
    let provider = view
        .providers
        .iter()
        .find(|p| p.provider == "anthropic")
        .unwrap();
    assert_eq!(provider.cost.coverage.as_ref().unwrap(), coverage);

    // Gemini: tokens = 300 measured, cost = 0.0 measured. Zero stays measured!
    let gemini = view
        .agents
        .iter()
        .find(|a| a.agent_id == "gemini")
        .expect("gemini row");
    assert_eq!(gemini.tokens.provenance, FigureProvenance::Measured);
    assert_eq!(gemini.tokens.value, Some(300.0));
    assert_eq!(gemini.cost.provenance, FigureProvenance::Measured);
    assert_eq!(gemini.cost.value, Some(0.0));
    assert_eq!(gemini.cost.unit, "currency");
}

#[test]
fn missing_token_fields_remain_unknown() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path();

    // Provider usage with cost only, completely absent token fields
    store
        .record_audit_event(
            root,
            Path::new("src/main.rs"),
            "provider.usage",
            &json!({
                "provider": "replicate",
                "cost": 0.12,
                "currency": "measured"
            })
            .to_string(),
            None,
        )
        .unwrap();

    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");

    let prov = view
        .providers
        .iter()
        .find(|p| p.provider == "replicate")
        .expect("replicate provider row");

    // Absent token fields must NOT be turned into measured 0 tokens
    assert_eq!(prov.tokens.provenance, FigureProvenance::Unknown);
    assert_eq!(prov.tokens.value, None);
    assert_eq!(prov.cost.provenance, FigureProvenance::Measured);
    assert_eq!(prov.cost.value, Some(0.12));
}

#[test]
fn unknown_actor_with_attribution_basis() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path();

    // Record an audit row with attribution: "inferred" but NO explicit actor_agent/agent/actor
    store
        .record_audit_event(
            root,
            Path::new("src/tool.rs"),
            "provider.usage",
            &json!({
                "provider": "openai",
                "attribution": "inferred",
                "prompt_tokens": 150,
                "completion_tokens": 50,
                "cost": 0.02,
                "currency": "measured"
            })
            .to_string(),
            None,
        )
        .unwrap();

    // Record with explicit non-roster actor
    store
        .record_audit_event(
            root,
            Path::new("src/tool.rs"),
            "provider.usage",
            &json!({
                "provider": "openai",
                "actor_agent": "external-worker",
                "cost": 0.03,
                "currency": "measured"
            })
            .to_string(),
            None,
        )
        .unwrap();

    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");

    // "inferred" must NOT become an agent ID
    assert!(
        view.agents.iter().all(|a| a.agent_id != "inferred"),
        "attribution string must never be used as an agent ID"
    );

    // Unattributed operations row preserves the usage
    let unattr = view
        .agents
        .iter()
        .find(|a| a.agent_id == "unattributed")
        .expect("unattributed row must exist");
    assert_eq!(unattr.tokens.value, Some(200.0));
    assert_eq!(unattr.cost.value, Some(0.02));

    // Non-roster explicit actor is preserved
    let external = view
        .agents
        .iter()
        .find(|a| a.agent_id == "external-worker")
        .expect("external-worker row must exist");
    assert_eq!(external.cost.value, Some(0.03));
}

#[test]
fn harness_without_agent_link() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    // Register a harness without writer_owner (agent link)
    store
        .conn
        .execute(
            "INSERT INTO harness_session_registrations(harness, workspace, disk_session_id, registered_at, mode, state, writer_owner) \
             VALUES ('claude', '/workspace/a', 'sess-1', '2026-10-19T12:00:00Z', 'managed', 'ready', 'claude')",
            [],
        )
        .unwrap();

    // Even a lease token equal to an agent name does not establish identity.
    let owner_view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("owner scorecard succeeds");
    let h_row = owner_view
        .harnesses
        .iter()
        .find(|h| h.harness == "claude" && h.workspace == "/workspace/a")
        .expect("harness row exists");
    assert_eq!(h_row.linked_agent_id, None);

    // AgentsOwnCard view for claude: harness must be excluded since linked_agent_id is None
    let agent_view = store
        .get_scorecard(Some("claude"), ScorecardVisibility::AgentsOwnCard)
        .expect("agent own card succeeds");
    assert!(
        agent_view.harnesses.is_empty(),
        "unlinked harness must not appear in agent's own card"
    );
}

#[test]
fn successful_budget_reservation_not_open_security_event() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path();

    // Successful budget reservation defaults to status 'pending' in SQLite schema
    store
        .record_audit_event(
            root,
            Path::new("src/budget.rs"),
            "budget.reserved",
            &json!({
                "actor_agent": "claude",
                "units": 100.0
            })
            .to_string(),
            None,
        )
        .unwrap();

    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");

    let claude = view
        .agents
        .iter()
        .find(|a| a.agent_id == "claude")
        .expect("claude row");
    assert!(
        claude.open_security_events.is_empty(),
        "successful budget.reserved audit row must not be counted as open security event"
    );
}

#[test]
fn agent_only_pending_gate_not_human_decision() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    // Agent-only pending gate (no "human" in to_agents_json)
    store
        .conn
        .execute(
            "INSERT INTO pending_gate_approvals(id, subject, from_agent, to_agents_json, body, reason, status, created_at) \
             VALUES ('gate-agent', 'test', 'claude', '[\"gemini\", \"grok\"]', 'body', 'reason', 'pending', '2026-10-19T08:00:00Z')",
            [],
        )
        .unwrap();

    let view1 = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");
    let claude1 = view1
        .agents
        .iter()
        .find(|a| a.agent_id == "claude")
        .expect("claude row");
    assert_eq!(
        claude1.oldest_pending_human_decision, None,
        "agent-only pending gate must not populate oldest_pending_human_decision"
    );

    // Gate targeting human
    store
        .conn
        .execute(
            "INSERT INTO pending_gate_approvals(id, subject, from_agent, to_agents_json, body, reason, status, created_at) \
             VALUES ('gate-human', 'test', 'claude', '[\"human\"]', 'body', 'reason', 'pending', '2026-10-19T09:00:00Z')",
            [],
        )
        .unwrap();

    let view2 = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");
    let claude2 = view2
        .agents
        .iter()
        .find(|a| a.agent_id == "claude")
        .expect("claude row");
    assert_eq!(
        claude2.oldest_pending_human_decision.as_deref(),
        Some("2026-10-19T09:00:00Z"),
        "human-targeted pending gate must populate oldest_pending_human_decision"
    );
}

#[test]
fn measured_cost_sorting_ignores_estimated_budget_units() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path();

    // 1. Agent "cheap-measured" has tiny measured cost ($0.05) and no budget
    store
        .record_audit_event(
            root,
            Path::new("src/cheap.rs"),
            "provider.usage",
            &json!({
                "provider": "anthropic",
                "actor_agent": "cheap-measured",
                "cost": 0.05,
                "currency": "measured"
            })
            .to_string(),
            None,
        )
        .unwrap();

    // 2. Agent "huge-budget" has large estimated budget units (5000.0) but 0 measured cost
    store.set_agent_budget("huge-budget", 10000.0).unwrap();
    store
        .gate_provider_call(
            "huge-budget",
            5000.0,
            Some("task-budget"),
            "Huge budget run",
            "Context",
            "Rationale",
            Some("human"),
        )
        .unwrap();

    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .expect("get_scorecard succeeds");

    let cheap_pos = view
        .agents
        .iter()
        .position(|a| a.agent_id == "cheap-measured")
        .expect("cheap-measured in list");
    let huge_pos = view
        .agents
        .iter()
        .position(|a| a.agent_id == "huge-budget")
        .expect("huge-budget in list");

    // cheap-measured has measured_cost_units = Some(0.05), huge-budget has None.
    // Comparable measured cost sorting must place cheap-measured BEFORE huge-budget!
    assert!(
        cheap_pos < huge_pos,
        "cheap-measured ({cheap_pos}) with measured cost must sort before huge-budget ({huge_pos}) with only estimated units"
    );
}
