use super::*;
use tempfile::tempdir;

fn setup_role(store: &HubStore, id: &str, quota: Option<i64>, max_recipients: Option<i64>) {
    store
        .upsert_role(id, id, quota, max_recipients, false, false, false, &[])
        .unwrap();
}

#[test]
fn allowed_send_consumes_one_unit_of_quota() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    setup_role(&store, "capped", Some(2), Some(10));
    store.assign_agent_role("grok", "capped").unwrap();
    store.set_team_member("claude", true).unwrap();

    assert_eq!(store.gate_quota_used_today("grok").unwrap(), 0);
    let outcomes = store
        .send_tagged_message_gated(
            "grok",
            &["claude".to_string()],
            false,
            true,
            "hello",
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(outcomes[0].accepted);
    assert_eq!(store.gate_quota_used_today("grok").unwrap(), 1);
}

#[test]
fn exhausted_quota_routes_to_pending_approval_instead_of_sending() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    setup_role(&store, "capped", Some(1), Some(10));
    store.assign_agent_role("grok", "capped").unwrap();
    store.set_team_member("claude", true).unwrap();

    store
        .send_tagged_message_gated(
            "grok",
            &["claude".to_string()],
            false,
            true,
            "first",
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let outcomes = store
        .send_tagged_message_gated(
            "grok",
            &["claude".to_string()],
            false,
            true,
            "second",
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(!outcomes[0].accepted);
    assert_eq!(outcomes[0].policy_decision, "gate_pending_role_limit");

    let pending = store.list_pending_gate_approvals(Some("pending")).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].body, "second");
    // The quota isn't consumed further by a gated (unsent) attempt.
    assert_eq!(store.gate_quota_used_today("grok").unwrap(), 1);
}

#[test]
fn broadcast_recipient_limit_gates_oversized_sends() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    setup_role(&store, "narrow", Some(10), Some(1));
    store.assign_agent_role("grok", "narrow").unwrap();
    store.set_team_member("claude", true).unwrap();
    store.set_team_member("gemini", true).unwrap();

    let outcomes = store
        .send_tagged_message_gated(
            "grok",
            &["claude".to_string(), "gemini".to_string()],
            false,
            true,
            "broadcast",
            None,
            None,
            None,
            None,
        )
        .unwrap();
    assert!(outcomes.iter().all(|o| !o.accepted));
    assert!(outcomes[0].reason.as_deref().unwrap().contains("exceeding"));
}

#[test]
fn approving_a_pending_gate_actually_delivers_it() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    setup_role(&store, "capped", Some(0), Some(10));
    store.assign_agent_role("grok", "capped").unwrap();
    store.set_team_member("claude", true).unwrap();

    store
        .send_tagged_message_gated(
            "grok",
            &["claude".to_string()],
            false,
            true,
            "needs approval",
            None,
            None,
            None,
            None,
        )
        .unwrap();
    let pending = store.list_pending_gate_approvals(Some("pending")).unwrap();
    assert_eq!(pending.len(), 1);

    let resolved = store.resolve_gate_approval(&pending[0].id, true).unwrap();
    assert_eq!(resolved.status, "approved");

    let delivered = store.list_messages(Some("claude"), None).unwrap();
    assert!(delivered.iter().any(|m| m.body == "needs approval"));
}

#[test]
fn rejecting_a_pending_gate_never_sends_and_notifies_the_sender() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    setup_role(&store, "capped", Some(0), Some(10));
    store.assign_agent_role("grok", "capped").unwrap();
    store.set_team_member("claude", true).unwrap();

    store
        .send_tagged_message_gated(
            "grok",
            &["claude".to_string()],
            false,
            true,
            "needs approval",
            None,
            None,
            None,
            None,
        )
        .unwrap();
    let pending = store.list_pending_gate_approvals(Some("pending")).unwrap();

    let resolved = store.resolve_gate_approval(&pending[0].id, false).unwrap();
    assert_eq!(resolved.status, "rejected");

    // The original message never reached claude...
    let delivered = store.list_messages(Some("claude"), None).unwrap();
    assert!(!delivered.iter().any(|m| m.body == "needs approval"));
    // ...but grok got an automated rejection notice.
    let notices = store.list_messages(Some("grok"), None).unwrap();
    assert!(notices.iter().any(|m| m.body.contains("rejected")));
}

#[test]
fn resolving_an_already_resolved_approval_fails() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    setup_role(&store, "capped", Some(0), Some(10));
    store.assign_agent_role("grok", "capped").unwrap();
    store.set_team_member("claude", true).unwrap();

    store
        .send_tagged_message_gated(
            "grok",
            &["claude".to_string()],
            false,
            true,
            "x",
            None,
            None,
            None,
            None,
        )
        .unwrap();
    let pending = store.list_pending_gate_approvals(Some("pending")).unwrap();
    store.resolve_gate_approval(&pending[0].id, true).unwrap();
    assert!(store.resolve_gate_approval(&pending[0].id, true).is_err());
}
