//! Owner digest regressions (Team Scorecard T4 / #344).

use super::super::*;
use crate::settings::ScorecardVisibility;
use serde_json::json;
use std::path::Path;
use tempfile::tempdir;

fn pending_gate_row(store: &HubStore, id: &str, session: Option<&str>, task: Option<&str>) {
    store
        .conn
        .execute(
            "INSERT INTO pending_gate_approvals(id, subject, from_agent, to_agents_json, is_wake, body, reason, status, created_at, session_id, task_id) \
             VALUES (?1, 'subject', 'grok', '[\"human\"]', 1, 'body', 'reason', 'pending', '2026-10-19T09:00:00Z', ?2, ?3)",
            params![id, session, task],
        )
        .unwrap();
}

#[test]
fn digest_groups_pending_decisions_by_work_session() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path();

    let session = store.create_work_session("release-cut").unwrap();

    // Pending gate approval stamped with the work session and a Hub task id.
    pending_gate_row(&store, "gate-1", Some(&session.id), Some("task-9"));

    // Pending os_tool proposal stamped with session, task, agent, and an
    // optional GitHub-issue label.
    store
        .record_audit_event(
            root,
            Path::new("tool.rs"),
            "os_tool.proposed",
            &json!({
                "kind": "os_tool",
                "exe": "cargo",
                "cmdline": ["cargo", "test", "-p", "hub"],
                "agent": "grok",
                "task_id": "task-9",
                "work_session_id": session.id,
                "github_issue": "#344",
            })
            .to_string(),
            None,
        )
        .unwrap();

    // Human-gated wake with no session stamp — ungrouped bucket.
    store
        .request_wake("grok", Some("continue slice"), None, true)
        .unwrap();

    let view = store
        .get_scorecard(None, ScorecardVisibility::OwnerOnly)
        .unwrap();
    let digest = view.digest.expect("owner sees the digest");

    assert_eq!(digest.summary.pending_count, 3);

    let session_group = digest
        .groups
        .iter()
        .find(|g| g.work_session_id.as_deref() == Some(session.id.as_str()))
        .expect("session group exists");
    assert_eq!(session_group.work_session_name.as_deref(), Some("release-cut"));
    assert_eq!(session_group.pending.len(), 2);

    let os_tool = session_group
        .pending
        .iter()
        .find(|d| d.kind == DigestDecisionKind::OsTool)
        .expect("os_tool decision");
    assert_eq!(os_tool.task_id.as_deref(), Some("task-9"));
    assert_eq!(os_tool.agent.as_deref(), Some("grok"));
    assert_eq!(os_tool.github_issue.as_deref(), Some("#344"));
    assert!(os_tool.age_seconds.is_some(), "age is measured, not zero");

    let ungrouped = digest
        .groups
        .iter()
        .find(|g| g.work_session_id.is_none())
        .expect("ungrouped bucket exists");
    assert!(ungrouped.pending.iter().any(|d| d.kind == DigestDecisionKind::Wake));

    // Oldest pending timestamp is surfaced for queue age.
    assert!(digest.summary.oldest_pending_at.is_some());
}

#[test]
fn digest_counts_resolutions_by_via_without_double_counting() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    // Two wakes resolved remotely, one gate approval decided on desktop.
    // Rejection keeps the fixture honest: approving a tagged send delivers a
    // follow-up human-gated wake to `human` by default policy, and that new
    // pending decision is the existing queue behavior, not this slice's.
    let wake_tg = store
        .request_wake("grok", Some("telegram slice"), None, true)
        .unwrap();
    let wake_tcp = store
        .request_wake("muse", Some("tcp slice"), None, true)
        .unwrap();
    pending_gate_row(&store, "gate-2", None, None);

    store
        .set_wake_status_via(&wake_tg.id, WakeStatus::Delivered, "telegram")
        .unwrap();
    store
        .set_wake_status_via(&wake_tcp.id, WakeStatus::Cancelled, "tcp")
        .unwrap();
    store.resolve_gate_approval("gate-2", false).unwrap();

    let digest = store
        .get_owner_digest(chrono::Utc::now(), DIGEST_WINDOW_DAYS)
        .unwrap();

    assert_eq!(digest.summary.pending_count, 0, "all three were resolved");
    assert_eq!(digest.summary.resolved_window.desktop, 1);
    assert_eq!(digest.summary.resolved_window.telegram, 1);
    assert_eq!(digest.summary.resolved_window.tcp, 1);
    assert_eq!(digest.summary.resolved_window.total, 3);

    // The wake.resolved audit rows carry the channel.
    let resolved = store.list_audit_events(false).unwrap();
    let via_values: Vec<String> = resolved
        .iter()
        .filter(|e| e.operation == "wake.resolved")
        .filter_map(|e| serde_json::from_str::<serde_json::Value>(&e.process_json).ok())
        .filter_map(|v| {
            v.get("via")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect();
    assert!(via_values.contains(&"telegram".to_string()));
    assert!(via_values.contains(&"tcp".to_string()));
    assert_eq!(via_values.len(), 2);
}

#[test]
fn digest_is_owner_only_under_every_visibility_mode() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    for visibility in [
        ScorecardVisibility::OwnerOnly,
        ScorecardVisibility::AgentsOwnCard,
        ScorecardVisibility::AllAgents,
    ] {
        let view = store
            .get_scorecard(Some("gemini"), visibility)
            .unwrap();
        assert!(
            view.digest.is_none(),
            "agent callers never see the owner digest ({visibility:?})"
        );
    }

    let owner = store
        .get_scorecard(None, ScorecardVisibility::AllAgents)
        .unwrap();
    assert!(owner.digest.is_some(), "the owner sees the digest");
    let human = store
        .get_scorecard(Some("human"), ScorecardVisibility::OwnerOnly)
        .unwrap();
    assert!(human.digest.is_some(), "\"human\" is the owner clock");
}

#[test]
fn digest_counts_open_findings_excluding_pending_proposals() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path();

    let session = store.create_work_session("audit-pass").unwrap();

    // Hard-denied tool: a finding, not a pending decision.
    store
        .record_audit_event(
            root,
            Path::new("tool.rs"),
            "os_tool.denied",
            &json!({
                "kind": "os_tool",
                "exe": "sudo",
                "cmdline": ["sudo", "rm", "-rf", "/"],
                "agent": "grok",
                "work_session_id": session.id,
            })
            .to_string(),
            None,
        )
        .unwrap();

    // Pending proposal: a decision, not a finding.
    store
        .record_audit_event(
            root,
            Path::new("tool.rs"),
            "os_tool.proposed",
            &json!({
                "kind": "os_tool",
                "exe": "git",
                "cmdline": ["git", "push"],
                "agent": "grok",
                "work_session_id": session.id,
            })
            .to_string(),
            None,
        )
        .unwrap();

    let digest = store
        .get_owner_digest(chrono::Utc::now(), DIGEST_WINDOW_DAYS)
        .unwrap();

    let session_group = digest
        .groups
        .iter()
        .find(|g| g.work_session_id.as_deref() == Some(session.id.as_str()))
        .expect("session group exists");
    assert_eq!(session_group.pending.len(), 1, "the proposal is a decision");
    assert_eq!(session_group.unresolved_findings, 1, "the denial is a finding");
    assert_eq!(digest.summary.unresolved_findings, 1);
}

#[test]
fn set_wake_status_via_rejects_unknown_channels() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let wake = store
        .request_wake("grok", Some("slice"), None, true)
        .unwrap();

    let err = store
        .set_wake_status_via(&wake.id, WakeStatus::Delivered, "carrier-pigeon")
        .unwrap_err();
    assert!(matches!(err, HubError::Invalid(_)));

    // The wake is untouched and still a pending decision.
    let digest = store
        .get_owner_digest(chrono::Utc::now(), DIGEST_WINDOW_DAYS)
        .unwrap();
    assert_eq!(digest.summary.pending_count, 1);
    assert_eq!(digest.summary.resolved_window.total, 0);
}
