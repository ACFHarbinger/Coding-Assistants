use super::super::*;
use tempfile::tempdir;

#[test]
fn fresh_and_untouched_legacy_rosters_require_explicit_agent_enrollment() {
    let fresh_dir = tempdir().unwrap();
    let fresh = HubStore::open(fresh_dir.path()).unwrap();
    let fresh_members: Vec<_> = fresh
        .list_team_members()
        .unwrap()
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    assert_eq!(fresh_members, vec!["human"]);

    let legacy_dir = tempdir().unwrap();
    let legacy = HubStore::open(legacy_dir.path()).unwrap();
    legacy
            .conn
            .execute(
                "UPDATE agents SET team_member = 1 WHERE id IN ('human', 'claude', 'chat', 'gemini', 'grok')",
                [],
            )
            .unwrap();
    legacy
        .conn
        .execute(
            "UPDATE meta SET value = '1' WHERE key = 'team_roster_seeded'",
            [],
        )
        .unwrap();
    drop(legacy);
    let migrated = HubStore::open(legacy_dir.path()).unwrap();
    let migrated_members: Vec<_> = migrated
        .list_team_members()
        .unwrap()
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    assert_eq!(migrated_members, vec!["human"]);
}

#[test]
fn memory_message_wake_roundtrip() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("claude", true).unwrap();

    let mem = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("grok"),
            None,
            Some("first handoff"),
            "Grok left a note for Claude about the hub schema.",
            &["hub".into(), "schema".into()],
        )
        .unwrap();
    assert_eq!(mem.tier, "episodic");

    let found = store.search_memories("hub schema").unwrap();
    assert_eq!(found.len(), 1);

    let msg = store
        .send_message(
            "grok",
            "claude",
            MessageKind::Handoff,
            "Please review the hub schema.",
            Some("schema review"),
            None,
            Some("task-1"),
        )
        .unwrap();
    assert_eq!(msg.status, "pending");

    let team_messages = store
        .send_message_to_team(
            "grok",
            MessageKind::Message,
            "A shared team update.",
            None,
            None,
            None,
        )
        .unwrap();
    assert!(team_messages
        .iter()
        .all(|message| message.from_agent == "grok"));
    assert!(team_messages
        .iter()
        .all(|message| message.to_agent != "grok"));
    assert!(team_messages.iter().all(|message| message
        .subject
        .as_deref()
        .unwrap()
        .starts_with("team:")));

    let polled = store.poll_messages("claude", true).unwrap();
    assert_eq!(polled.len(), 2);
    assert!(polled
        .iter()
        .any(|message| message.id == msg.id && message.status == "acked"));

    let wake = store
        .request_wake("claude", Some("schema ready"), Some(&msg.id), true)
        .unwrap();
    let duplicate = store
        .request_wake("claude", Some("schema ready"), Some(&msg.id), true)
        .unwrap();
    assert!(wake.requires_human_gate);
    assert_eq!(wake.id, duplicate.id);
    assert_eq!(store.list_wakes(Some("claude"), true).unwrap().len(), 1);
    assert!(dir
        .path()
        .join("wake")
        .join(format!("{}.json", wake.id))
        .exists());

    let journal = store
        .append_private_journal("grok", "Private note: do not share.")
        .unwrap();
    assert!(journal.exists());
    // private journal must not appear in shared memory tables
    assert!(store.search_memories("do not share").unwrap().is_empty());

    let export = store.export_markdown(None).unwrap();
    let text = fs::read_to_string(export).unwrap();
    assert!(text.contains("first handoff"));
}

#[test]
fn audit_events_are_reviewable_and_hash_chained() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path().join("watched");
    fs::create_dir_all(&root).unwrap();

    let first = store
        .record_audit_event(
            &root,
            Path::new("journals/chat.md"),
            "modified",
            r#"{"pid":123,"attribution":"test"}"#,
            Some("abc"),
        )
        .unwrap();
    let second = store
        .record_audit_event(
            &root,
            Path::new("journals/chat.md"),
            "modified",
            r#"{"pid":123,"attribution":"test"}"#,
            Some("def"),
        )
        .unwrap();
    assert_eq!(
        second.previous_hash.as_deref(),
        Some(first.event_hash.as_str())
    );
    assert_eq!(store.verify_audit_chain().unwrap(), 2);
    assert_eq!(store.list_audit_events(true).unwrap().len(), 2);
    store.set_audit_status(&first.id, "approved").unwrap();
    assert_eq!(store.list_audit_events(true).unwrap().len(), 1);
    assert!(store.set_audit_status("missing", "approved").is_err());
}

#[test]
fn settings_audit_events_are_approved_and_isolated_from_other_audit_rows() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path().join("watched");
    fs::create_dir_all(&root).unwrap();

    // A non-settings audit row (e.g. an observed filesystem change) should
    // not leak into the settings-scoped view.
    store
        .record_audit_event(&root, Path::new("journals/chat.md"), "modified", "{}", None)
        .unwrap();

    let event = store
        .record_settings_audit_event("storage.backup_retention", "global", "update")
        .unwrap();
    assert_eq!(event.root_path, "settings");
    assert_eq!(event.path, "storage.backup_retention");
    assert_eq!(event.operation, "update");
    assert_eq!(event.status, "approved");
    assert!(event.process_json.contains("\"scope\":\"global\""));
    assert!(!event.process_json.contains("backup_retention\":"));

    let settings_events = store.list_settings_audit_events().unwrap();
    assert_eq!(settings_events.len(), 1);
    assert_eq!(settings_events[0].id, event.id);

    // The event is still on the shared Hub audit chain alongside the
    // filesystem row, just filterable by root_path.
    assert_eq!(store.list_audit_events(false).unwrap().len(), 2);
    assert_eq!(store.verify_audit_chain().unwrap(), 2);
}

#[test]
fn promote_and_compact_short_term() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    for i in 0..5 {
        store
            .write_memory(
                MemoryTier::ShortTerm,
                MemoryScope::Global,
                Some("grok"),
                None,
                Some(&format!("note-{i}")),
                &format!("short body {i}"),
                &[],
            )
            .unwrap();
        // tiny delay so created_at ordering is stable across platforms
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    let report = store.compact_short_term(2).unwrap();
    assert_eq!(report.promoted, 3);
    assert_eq!(report.kept, 2);

    let short = store
        .list_memories(None, Some(MemoryTier::ShortTerm), None, false)
        .unwrap();
    assert_eq!(short.len(), 2);

    let episodic = store
        .list_memories(None, Some(MemoryTier::Episodic), None, false)
        .unwrap();
    assert_eq!(episodic.len(), 3);
    assert!(episodic[0].body.contains("Promoted from"));

    let one = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("claude"),
            None,
            Some("decision"),
            "Use SQLite as source of truth.",
            &[],
        )
        .unwrap();
    let semantic = store.promote_memory(&one.id, MemoryTier::Semantic).unwrap();
    assert_eq!(semantic.tier, "semantic");
    store.delete_memory(&semantic.id).unwrap();
    assert!(store.get_memory(&semantic.id).unwrap().is_none());
}

#[test]
fn memory_links_connect_related_memories_across_authors() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let claude_note = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("claude"),
            None,
            Some("checkout decision"),
            "Hosted checkout redirect, no first-party payment backend.",
            &["checkout".to_string()],
        )
        .unwrap();
    let grok_note = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("grok"),
            None,
            Some("payment options"),
            "Stripe Payment Links keep us on a static export.",
            &["payments".to_string()],
        )
        .unwrap();
    let unrelated = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("gemini"),
            None,
            Some("brand palette"),
            "Deep Slate + Clinical Cyan for the icon set.",
            &[],
        )
        .unwrap();

    // Self-links are rejected.
    assert!(store
        .link_memories(&claude_note.id, &claude_note.id, None, "human")
        .is_err());

    let link = store
        .link_memories(
            &claude_note.id,
            &grok_note.id,
            Some("agrees"),
            "human",
        )
        .unwrap();
    assert_eq!(link.relation.as_deref(), Some("agrees"));
    assert_eq!(link.created_by, "human");

    // A second, independently-drawn edge between the same pair is allowed —
    // it's signal (two observers converged), not a duplicate to reject.
    store
        .link_memories(&claude_note.id, &grok_note.id, None, "system:auto-link")
        .unwrap();

    let links = store.list_memory_links(&claude_note.id).unwrap();
    assert_eq!(links.len(), 2);
    // Listing from the *other* endpoint finds the same edges (undirected lookup).
    assert_eq!(store.list_memory_links(&grok_note.id).unwrap().len(), 2);

    let related = store.related_memories(&claude_note.id, 1).unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].id, grok_note.id);
    assert!(!related.iter().any(|m| m.id == unrelated.id));

    // depth = 0 is a deliberate no-op, not "unbounded."
    assert!(store.related_memories(&claude_note.id, 0).unwrap().is_empty());

    let by_topic = store.memories_for_topic("checkout").unwrap();
    assert!(by_topic
        .get("claude")
        .is_some_and(|v| v.iter().any(|m| m.id == claude_note.id)));
    assert!(!by_topic.contains_key("gemini"));

    store.unlink_memories(&link.id).unwrap();
    assert_eq!(store.list_memory_links(&claude_note.id).unwrap().len(), 1);
    assert!(store.unlink_memories(&link.id).is_err());
}

#[test]
fn suggest_links_scores_shared_tags_and_terms_above_unrelated_memories() {
    use crate::settings::LinkSuggestionMode;

    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let source = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("claude"),
            None,
            Some("checkout decision"),
            "Hosted checkout redirect via Stripe Payment Links, no first-party payment backend.",
            &["checkout".to_string(), "payments".to_string()],
        )
        .unwrap();
    let strong_match = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("grok"),
            None,
            Some("payment provider notes"),
            "Stripe Payment Links keep the checkout flow on a static export.",
            &["payments".to_string()],
        )
        .unwrap();
    let unrelated = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("gemini"),
            None,
            Some("brand palette"),
            "Deep Slate and Clinical Cyan for the icon set.",
            &[],
        )
        .unwrap();

    // Pure scoring: Off/Suggest/Auto all still classify strong_match ahead of unrelated.
    let suggestions = store.suggest_links_for_memory(&source.id, 10).unwrap();
    let ids: Vec<&str> = suggestions.iter().map(|s| s.candidate.id.as_str()).collect();
    assert!(ids.contains(&strong_match.id.as_str()));
    assert!(!ids.contains(&unrelated.id.as_str()));
    assert!(suggestions[0].candidate.id == strong_match.id);
    assert!(suggestions[0].score > 0.0);
    assert!(!suggestions[0].reason.is_empty());

    // Off: no scoring performed, no edges, empty result.
    let off = store
        .apply_link_suggestions(&source.id, LinkSuggestionMode::Off, 10)
        .unwrap();
    assert!(off.is_empty());
    assert!(store.list_memory_links(&source.id).unwrap().is_empty());

    // Suggest: candidates returned, but nothing written.
    let suggested = store
        .apply_link_suggestions(&source.id, LinkSuggestionMode::Suggest, 10)
        .unwrap();
    assert!(!suggested.is_empty());
    assert!(store.list_memory_links(&source.id).unwrap().is_empty());

    // Auto: strong_match's score clears AUTO_ACCEPT_THRESHOLD, so an edge gets
    // drawn automatically, attributed to the system, not either author.
    let auto = store
        .apply_link_suggestions(&source.id, LinkSuggestionMode::Auto, 10)
        .unwrap();
    assert_eq!(auto.len(), suggested.len());
    let links = store.list_memory_links(&source.id).unwrap();
    assert!(links
        .iter()
        .any(|l| l.to_memory_id == strong_match.id && l.created_by == "system:auto-link"));

    // A memory already linked (even manually) isn't re-suggested.
    let after_auto = store.suggest_links_for_memory(&source.id, 10).unwrap();
    assert!(!after_auto
        .iter()
        .any(|s| s.candidate.id == strong_match.id));
}

#[test]
fn wake_policy_and_retention() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    // Default policy forces human gate even when caller passes false.
    let wake = store
        .request_wake("claude", Some("need review"), None, false)
        .unwrap();
    assert!(wake.requires_human_gate);

    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: false,
        })
        .unwrap();
    let err = store
        .request_wake("claude", Some("auto"), None, false)
        .unwrap_err();
    assert!(err.to_string().contains("forbids auto-wake"));

    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: true,
        })
        .unwrap();
    let auto = store
        .request_wake("gemini", Some("auto ok"), None, false)
        .unwrap();
    assert!(!auto.requires_human_gate);
    store
        .set_wake_status(&auto.id, WakeStatus::Delivered)
        .unwrap();
    assert_eq!(store.list_wakes(Some("gemini"), true).unwrap().len(), 0);

    let m = store
        .write_memory(
            MemoryTier::ShortTerm,
            MemoryScope::Global,
            Some("grok"),
            None,
            Some("old"),
            "stale me",
            &[],
        )
        .unwrap();
    store.mark_memory_stale(&m.id, true).unwrap();
    assert_eq!(store.purge_stale_memories().unwrap(), 1);
    assert!(store.get_memory(&m.id).unwrap().is_none());
}

#[test]
fn m6_cross_agent_handoff_acceptance_flow() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let handoff = store
        .send_message(
            "grok",
            "claude",
            MessageKind::Handoff,
            "The shared Hub slice is ready for review.",
            Some("m6-acceptance"),
            None,
            Some("m6-acceptance"),
        )
        .unwrap();
    let memory = store
        .write_memory_with_source(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("grok"),
            Some("m6-acceptance"),
            Some("Hub handoff"),
            "Review the Hub implementation and verify wake delivery.",
            &["handoff".into(), "acceptance".into()],
            Some("m6-acceptance"),
        )
        .unwrap();
    assert_eq!(memory.source_event_id.as_deref(), Some("m6-acceptance"));

    let inbox = store.poll_messages("claude", true).unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].id, handoff.id);
    assert_eq!(inbox[0].status, "acked");

    let wake = store
        .request_wake("claude", Some("handoff ready"), Some(&handoff.id), true)
        .unwrap();
    let duplicate = store
        .request_wake("claude", Some("handoff ready"), Some(&handoff.id), true)
        .unwrap();
    assert_eq!(wake.id, duplicate.id);
    store
        .set_wake_status(&wake.id, WakeStatus::Delivered)
        .unwrap();
    assert!(store.list_wakes(Some("claude"), true).unwrap().is_empty());

    let export = store.export_markdown(None).unwrap();
    let text = fs::read_to_string(export).unwrap();
    assert!(text.contains("Hub handoff"));
    assert!(text.contains("The shared Hub slice is ready for review."));

    // CA-103: Messager-style channel communication across multiple agent
    // roles must stay isolated per channel at the data layer, since the
    // desktop MessagerPanel filters purely by `subject == "channel:<id>"`
    // over the full `list_messages` result — a leak here would be
    // invisible in the UI but would surface as one channel seeing
    // another channel's traffic.
    store
        .send_message(
            "grok",
            "claude",
            MessageKind::Message,
            "general channel: build is green",
            Some("channel:general"),
            None,
            None,
        )
        .unwrap();
    store
        .send_message(
            "gemini",
            "claude",
            MessageKind::Message,
            &format!("team-coordination channel: see memory:{}", memory.id),
            Some("channel:team-coordination"),
            None,
            None,
        )
        .unwrap();
    store
        .send_message(
            "grok",
            "human",
            MessageKind::Message,
            "DM: quick question about the Hub schema",
            None,
            None,
            None,
        )
        .unwrap();

    let all = store.list_messages(None, None).unwrap();
    let general: Vec<_> = all
        .iter()
        .filter(|m| m.subject.as_deref() == Some("channel:general"))
        .collect();
    let team_coord: Vec<_> = all
        .iter()
        .filter(|m| m.subject.as_deref() == Some("channel:team-coordination"))
        .collect();
    assert_eq!(general.len(), 1);
    assert_eq!(general[0].body, "general channel: build is green");
    assert_eq!(team_coord.len(), 1);
    assert!(team_coord[0].body.contains(&memory.id));
    assert!(general.iter().all(|m| m.body != team_coord[0].body));

    let dm: Vec<_> = all
        .iter()
        .filter(|m| m.from_agent == "grok" && m.to_agent == "human")
        .collect();
    assert_eq!(dm.len(), 1);
    assert!(dm[0].subject.is_none());

    // Memory-link retrieval: a channel message can reference a memory
    // id inline (as the desktop drawer's "attach memory" action does);
    // the linked memory must still be reachable through the normal
    // search path used by both the CLI and the Tauri hub commands.
    let linked = store
        .search_memories("Review the Hub implementation")
        .unwrap();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].id, memory.id);
    assert!(team_coord[0].body.contains(&linked[0].id));
}
