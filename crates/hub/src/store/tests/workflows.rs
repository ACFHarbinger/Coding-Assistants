use super::super::*;
use tempfile::tempdir;

#[test]
fn c4_task_policy_controls_wake_gate() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: true,
        })
        .unwrap();
    let steps = vec![WorkflowStep {
        agent: "claude".into(),
        role: None,
        instruction: "Run the delegated step.".into(),
        max_retries: 0,
        parallel_group: None,
    }];
    let task = store
        .create_task_with_parallel("ungated task", None, &steps, 1, false)
        .unwrap();
    store.advance_task(&task.id, Some("human"), None).unwrap();
    let wakes = store.list_wakes(Some("claude"), true).unwrap();
    assert_eq!(wakes.len(), 1);
    assert!(!wakes[0].requires_human_gate);
    assert!(
        !store
            .get_task(&task.id)
            .unwrap()
            .unwrap()
            .require_human_approval
    );
}

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
fn work_sessions_reject_empty_or_oversized_name() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    assert!(store.create_work_session("   ").is_err());
    let long_name = "a".repeat(121);
    assert!(store.create_work_session(&long_name).is_err());
}

#[test]
fn c11_task_tag_rejects_absent_recipient_without_side_effects() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("grok", true).unwrap();
    // "outsider" has never been seen, so it starts out absent.
    let outcomes = store
        .send_tagged_message(
            "human",
            &["grok".to_string(), "outsider".to_string()],
            true,
            false,
            "ship the release",
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let grok = outcomes.iter().find(|o| o.to_agent == "grok").unwrap();
    assert!(grok.accepted);
    assert!(!grok.enrolled);
    assert!(grok.message_id.is_some());

    let outsider = outcomes.iter().find(|o| o.to_agent == "outsider").unwrap();
    assert!(!outsider.accepted);
    assert!(!outsider.enrolled);
    assert!(outsider.message_id.is_none());
    assert!(outsider
        .reason
        .as_deref()
        .unwrap()
        .contains("not a current"));

    // No membership mutation and no message actually delivered to "outsider".
    assert!(!store.is_team_member("outsider").unwrap());
    assert!(store
        .list_messages(Some("outsider"), None)
        .unwrap()
        .is_empty());

    // Durable per-recipient audit trail survives independent of the caller.
    let replayed = store
        .list_tagged_send_outcomes(&outcomes[0].subject)
        .unwrap();
    assert_eq!(replayed.len(), 2);
}

#[test]
fn c11_wake_tag_enrolls_and_requests_wake_for_a_new_identity() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    assert!(!store.is_team_member("newbie").unwrap());

    let outcomes = store
        .send_tagged_message(
            "human",
            &["newbie".to_string()],
            false,
            true,
            "join the session and pick up C12",
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let outcome = &outcomes[0];
    assert!(outcome.accepted);
    assert!(outcome.enrolled);
    assert!(outcome.wake_requested);
    assert!(store.is_team_member("newbie").unwrap());
    let pending = store.list_wakes(Some("newbie"), true).unwrap();
    assert_eq!(pending.len(), 1);
}

#[test]
fn c11_task_and_wake_together_apply_both_rules_per_recipient() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("grok", true).unwrap();
    let session = store.create_work_session("Cloud sync design").unwrap();

    let outcomes = store
        .send_tagged_message(
            "human",
            &["grok".to_string(), "fresh".to_string()],
            true,
            true,
            "session kickoff",
            None,
            None,
            None,
            Some(&session.id),
        )
        .unwrap();

    // "grok" is already a team+session member: task passes, wake is a no-op enroll.
    let grok = outcomes.iter().find(|o| o.to_agent == "grok").unwrap();
    assert!(grok.accepted);
    assert!(!grok.enrolled);

    // "fresh" is present in neither team nor session, so the task check
    // fails first — task always wins over wake for the same recipient.
    let fresh = outcomes.iter().find(|o| o.to_agent == "fresh").unwrap();
    assert!(!fresh.accepted);
    assert!(!fresh.enrolled);
    assert!(!store.is_team_member("fresh").unwrap());
}
