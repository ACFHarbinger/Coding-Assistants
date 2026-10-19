//! Shared fixture helpers for the probe regressions (T7 / #347), split
//! out so `probe_tests.rs` stays inside the 500-LoC cap.

use crate::store::{HubStore, ProviderUsage, QuotaSnapshot, ScorecardProbe, SliceStamp};
use rusqlite::params;

pub(crate) fn set_observed_at(store: &HubStore, id: &str, at: &str) {
    store
        .conn
        .execute(
            "UPDATE audit_events SET observed_at = ?1 WHERE id = ?2",
            params![at, id],
        )
        .unwrap();
}

pub(crate) fn snapshot(store: &HubStore, provider: &str, balance: f64, at: &str) -> String {
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

pub(crate) fn usage_row(
    store: &HubStore,
    provider: &str,
    cost: f64,
    at: &str,
    task: Option<&str>,
) -> String {
    usage_full(store, provider, cost, at, task, None, "cursor")
}

pub(crate) fn usage_full(
    store: &HubStore,
    provider: &str,
    cost: f64,
    at: &str,
    task: Option<&str>,
    session: Option<&str>,
    agent: &str,
) -> String {
    let usage = ProviderUsage {
        provider: provider.into(),
        actor_agent: agent.into(),
        prompt_tokens: Some(1),
        completion_tokens: Some(1),
        cache_tokens: None,
        cost: Some(cost),
        via_gateway: None,
        model: None,
        stamp: SliceStamp::registered(
            task.map(str::to_string),
            session.map(str::to_string),
            Some(agent.into()),
        ),
    };
    let event = store.record_provider_usage(&usage).unwrap().unwrap();
    set_observed_at(store, &event.id, at);
    event.id
}

pub(crate) fn task_with_agents(store: &HubStore, id: &str, agents: &[&str]) {
    let steps: Vec<serde_json::Value> = agents
        .iter()
        .map(|agent| serde_json::json!({ "agent": agent, "instruction": "work the slice" }))
        .collect();
    store
        .conn
        .execute(
            "INSERT INTO tasks (id, title, status, step_index, steps_json, created_at, updated_at, \
             attempts_json, open_agents_json, pending_agents_json, max_parallel, require_human_approval) \
             VALUES (?1, 't', 'pending', 0, ?2, '2026-10-19T09:00:00Z', '2026-10-19T09:00:00Z', \
             '{}', '[]', '[]', 4, 1)",
            params![id, serde_json::to_string(&steps).unwrap()],
        )
        .unwrap();
}

pub(crate) fn session_with_members(store: &HubStore, id: &str, agents: &[&str]) {
    store
        .conn
        .execute(
            "INSERT INTO work_sessions (id, name, created_at) VALUES (?1, 's', '2026-10-19T09:00:00Z')",
            params![id],
        )
        .unwrap();
    for agent in agents {
        store
            .conn
            .execute(
                "INSERT INTO work_session_members (session_id, agent_id, created_at) VALUES (?1, ?2, '2026-10-19T09:00:00Z')",
                params![id, agent],
            )
            .unwrap();
    }
}

pub(crate) fn probes_for<'a>(probes: &'a [ScorecardProbe], id: &str) -> Vec<&'a ScorecardProbe> {
    probes.iter().filter(|p| p.probe == id).collect()
}

pub(crate) fn disagreement_count(store: &HubStore) -> i64 {
    store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM audit_events WHERE operation = 'scorecard.disagreement'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

/// Durable task handoff message: timestamped evidence that the Hub handed
/// this agent the slice (assignment fact at the relevant time).
pub(crate) fn task_message(store: &HubStore, id: &str, to: &str, task: &str, at: &str) {
    store
        .conn
        .execute(
            "INSERT INTO messages (id, from_agent, to_agent, task_id, kind, status, body, created_at) \
             VALUES (?1, 'human', ?2, ?3, 'task', 'pending', 'work this slice', ?4)",
            params![id, to, task, at],
        )
        .unwrap();
}
