//! Memory tier, memory-link, and link-suggestion tests (split from
//! tests/roster.rs for the 500-LoC cap, #158).

use super::super::*;
use tempfile::tempdir;

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
        .link_memories(&claude_note.id, &grok_note.id, Some("agrees"), "human")
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
    assert!(store
        .related_memories(&claude_note.id, 0)
        .unwrap()
        .is_empty());

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
    let ids: Vec<&str> = suggestions
        .iter()
        .map(|s| s.candidate.id.as_str())
        .collect();
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
    assert!(!after_auto.iter().any(|s| s.candidate.id == strong_match.id));
}
