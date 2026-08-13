use super::super::*;
use tempfile::tempdir;

#[test]
fn c11_wake_request_denial_does_not_undo_enrollment_or_delivery() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: false,
        })
        .unwrap();

    let outcomes = store
        .send_tagged_message(
            "human",
            &["gated".to_string()],
            false,
            true,
            "policy should deny this auto-wake",
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let outcome = &outcomes[0];
    // Enrollment and message delivery still happen; only the wake itself
    // is denied by the standing auto-wake-forbidden policy.
    assert!(outcome.accepted);
    assert!(outcome.enrolled);
    assert!(store.is_team_member("gated").unwrap());
    assert!(!store.list_messages(Some("gated"), None).unwrap().is_empty());
    assert!(!outcome.wake_requested);
    assert!(outcome.reason.as_deref().unwrap().contains("denied"));
    assert_eq!(outcome.policy_decision, "wake_denied_policy");
}

#[test]
fn c11_send_tagged_message_requires_a_tag_and_a_recipient() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    assert!(store
        .send_tagged_message(
            "human",
            &["grok".to_string()],
            false,
            false,
            "body",
            None,
            None,
            None,
            None
        )
        .is_err());
    assert!(store
        .send_tagged_message("human", &[], true, false, "body", None, None, None, None)
        .is_err());
}

#[test]
fn c10_session_send_persists_the_explicit_recipient_set() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("grok", true).unwrap();
    store.set_team_member("claude", true).unwrap();
    let session = store.create_work_session("C10 recipients").unwrap();
    let recipients = vec!["grok".to_string(), "claude".to_string()];
    let messages = store
        .send_session_message(
            "human",
            &session.id,
            &recipients,
            "review this routing",
            Some("channel:session:c10-test"),
            None,
            None,
        )
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert!(messages
        .iter()
        .all(|message| recipients.contains(&message.to_agent)));
    let recorded: String = store
        .conn
        .query_row(
            "SELECT recipient_ids_json FROM message_recipient_sets WHERE subject = ?1",
            params!["channel:session:c10-test"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&recorded).unwrap(),
        recipients
    );
    assert!(store
        .send_session_message(
            "human",
            &session.id,
            &["outsider".to_string()],
            "must fail",
            None,
            None,
            None,
        )
        .is_err());
}

#[test]
fn harness_capture_dedups_the_same_body() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let session = store.create_work_session("C12 capture").unwrap();
    let first = store
        .record_harness_capture(
            "grok",
            "grok",
            Some(&session.id),
            "working on the inject path",
            None,
        )
        .unwrap();
    assert!(first.is_some());
    let second = store
        .record_harness_capture(
            "grok",
            "grok",
            Some(&session.id),
            "working on the inject path",
            None,
        )
        .unwrap();
    assert!(second.is_none());
    let listed = store
        .list_channel_messages(&format!("session:{}", session.id), 20)
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].from_agent, "grok");
}

#[test]
fn chat_channels_can_be_created_and_deleted_but_builtins_remain() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let listed = store.list_channels().unwrap();
    assert!(listed
        .iter()
        .any(|channel| channel.id == "general" && channel.builtin));
    let created = store
        .create_channel("#Design Review", Some("UI and docs"))
        .unwrap();
    assert_eq!(created.id, "design-review");
    assert_eq!(created.name, "#design-review");
    assert!(!created.builtin);
    assert!(store.create_channel("design review", None).is_err());
    assert!(store.delete_channel("general").is_err());
    store.delete_channel("design-review").unwrap();
    assert!(!store
        .list_channels()
        .unwrap()
        .iter()
        .any(|channel| channel.id == "design-review"));
    let restored = store
        .create_channel("design-review", Some("again"))
        .unwrap();
    assert_eq!(restored.id, "design-review");
}

#[test]
fn harness_session_registration_is_upserted_per_workspace() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .register_harness_session(
            "grok",
            "/tmp/ca-bridge-ws",
            "session-a",
            Some("/tmp/leader.sock"),
        )
        .unwrap();
    store
        .register_harness_session("grok", "/tmp/ca-bridge-ws", "session-b", None)
        .unwrap();
    let loaded = store
        .get_harness_session("grok", "/tmp/ca-bridge-ws")
        .unwrap()
        .unwrap();
    assert_eq!(loaded.disk_session_id, "session-b");
    assert!(loaded.leader_socket.is_none());
}
