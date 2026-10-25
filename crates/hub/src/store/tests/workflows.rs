use super::super::*;
use tempfile::tempdir;

#[test]
fn c6_budget_exhaustion_pauses_writes_handoff_and_blocks_wakes() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let set = store.set_agent_budget("claude", 10.0).unwrap();
    assert_eq!(set.spent_units, 0.0);
    assert!(!set.paused);

    // Under the limit: no pause, wakes still allowed.
    let under = store.record_budget_usage("claude", 4.0).unwrap();
    assert!(!under.paused);
    store
        .request_wake("claude", Some("still fine"), None, true)
        .unwrap();

    // Crossing the limit flips paused, but record_budget_usage alone
    // does not yet write a handoff or block new wakes on its own -- the
    // caller must call pause_for_budget to do that explicitly.
    let over = store.record_budget_usage("claude", 10.0).unwrap();
    assert!(over.paused);
    assert_eq!(over.spent_units, 14.0);

    let outcome = store
        .pause_for_budget(
            "claude",
            Some("task-42"),
            "Implement C6 budget handoff.",
            "Schema + store methods + tests.",
            "CLI/Tauri wiring and roadmap docs.",
            Some("grok"),
        )
        .unwrap();
    assert!(outcome.status.paused);
    assert!(outcome.summary_path.exists());
    let summary = fs::read_to_string(&outcome.summary_path).unwrap();
    assert!(summary.contains("Implement C6 budget handoff."));
    assert!(summary.contains("Delegated to"));
    assert!(summary.contains("grok"));

    let handoff = store
        .get_message(&outcome.handoff_message_id)
        .unwrap()
        .unwrap();
    assert_eq!(handoff.from_agent, "claude");
    assert_eq!(handoff.to_agent, "grok");
    assert_eq!(handoff.kind, "handoff");

    // Paused agent cannot receive further wakes until resumed.
    let err = store
        .request_wake("claude", Some("try again"), None, true)
        .unwrap_err();
    assert!(err.to_string().contains("budget-paused"));

    let resumed = store.resume_agent("claude").unwrap();
    assert!(!resumed.paused);
    store
        .request_wake("claude", Some("resumed"), None, true)
        .unwrap();
}

#[test]
fn c6_shutdown_records_reviewable_handoff() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let outcome = store
        .record_shutdown(
            "claude",
            Some("task-99"),
            "Finish the migration",
            "owner cancelled the active provider call",
            Some("grok"),
        )
        .unwrap();
    assert!(outcome.summary_path.exists());
    let summary = fs::read_to_string(&outcome.summary_path).unwrap();
    assert!(summary.contains("Finish the migration"));
    assert!(summary.contains("owner cancelled"));
    let message = store
        .get_message(&outcome.handoff_message_id)
        .unwrap()
        .unwrap();
    assert_eq!(message.to_agent, "grok");
    assert_eq!(message.kind, "handoff");
}

#[test]
fn channel_queries_and_memory_reference_resolution_are_isolated() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let memory = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("chat"),
            None,
            Some("Channel query contract"),
            "Messages can reference durable shared memory.",
            &[],
        )
        .unwrap();
    let short_id = &memory.id[..8];
    let general = store
        .send_message(
            "chat",
            "grok",
            MessageKind::Message,
            &format!("Review this [Memory #{short_id}] twice [Memory #{short_id}]."),
            Some("channel:general"),
            None,
            None,
        )
        .unwrap();
    store
        .send_message(
            "chat",
            "grok",
            MessageKind::Message,
            "Coordination-only message.",
            Some("channel:team-coordination"),
            None,
            None,
        )
        .unwrap();
    store
        .send_message(
            "chat",
            "grok",
            MessageKind::Message,
            "A general thread detail.",
            Some("channel:general:thread-1"),
            None,
            None,
        )
        .unwrap();

    let channel = store.list_channel_messages("general", 10).unwrap();
    assert_eq!(channel.len(), 2);
    assert!(channel.iter().all(|message| message
        .subject
        .as_deref()
        .unwrap()
        .starts_with("channel:general")));
    assert!(!channel
        .iter()
        .any(|message| message.body == "Coordination-only message."));
    assert_eq!(
        store
            .list_channel_messages("channel:general", 1)
            .unwrap()
            .len(),
        1
    );
    assert!(store.list_channel_messages("", 10).is_err());

    assert_eq!(
        parse_memory_references(&general.body),
        vec![short_id.to_string()]
    );
    assert!(parse_memory_references("[Memory #not-an-id] [Memory #]").is_empty());
    let linked = store.list_message_memories(&general.id).unwrap();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].id, memory.id);
}

#[test]
fn work_sessions_start_with_the_team_and_accept_later_team_members() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("grok", true).unwrap();

    let session = store.create_work_session("Cloud sync design").unwrap();
    assert!(session.member_ids.contains(&"human".to_string()));
    assert!(session.member_ids.contains(&"grok".to_string()));
    assert!(!session.member_ids.contains(&"claude".to_string()));

    store.set_team_member("claude", true).unwrap();
    let updated = store
        .add_work_session_member(&session.id, "claude")
        .unwrap();
    assert!(updated.member_ids.contains(&"claude".to_string()));

    let unchanged = store
        .add_work_session_member(&session.id, "claude")
        .unwrap();
    assert_eq!(
        unchanged
            .member_ids
            .iter()
            .filter(|agent_id| agent_id.as_str() == "claude")
            .count(),
        1
    );
    assert_eq!(store.list_work_sessions().unwrap()[0].id, session.id);
}

#[test]
fn work_sessions_can_start_with_an_explicit_or_empty_roster() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("grok", true).unwrap();

    let explicit = store
        .create_work_session_with_members("Focused work", Some(&["claude".into()]))
        .unwrap();
    assert_eq!(explicit.member_ids, vec!["claude"]);

    let empty = store
        .create_work_session_with_members("Private draft", Some(&[]))
        .unwrap();
    assert!(empty.member_ids.is_empty());
    assert!(store
        .create_work_session_with_members("Missing", Some(&["absent".into()]))
        .is_err());
}

#[test]
fn work_sessions_reject_empty_or_oversized_name() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    assert!(store.create_work_session("   ").is_err());
    let long_name = "a".repeat(121);
    assert!(store.create_work_session(&long_name).is_err());
}
