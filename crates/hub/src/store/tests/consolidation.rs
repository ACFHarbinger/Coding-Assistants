use super::super::*;
use tempfile::tempdir;

#[test]
fn consolidation_writes_summary_links_sources_and_marks_them_stale() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let first = store
        .write_memory(
            MemoryTier::ShortTerm,
            MemoryScope::Workspace,
            Some("claude"),
            Some("/repo"),
            Some("Release decision"),
            "Use signed release artifacts for the next release.",
            &["release".into(), "security".into()],
        )
        .unwrap();
    let second = store
        .write_memory(
            MemoryTier::ShortTerm,
            MemoryScope::Workspace,
            Some("grok"),
            Some("/repo"),
            Some("Release security"),
            "The release needs signed artifacts and approval.",
            &["release".into(), "security".into()],
        )
        .unwrap();
    let other = store
        .write_memory(
            MemoryTier::ShortTerm,
            MemoryScope::Workspace,
            Some("claude"),
            Some("/other"),
            Some("Release security"),
            "Unrelated workspace release note.",
            &["release".into(), "security".into()],
        )
        .unwrap();

    let clusters = store.consolidation_clusters().unwrap();
    assert_eq!(clusters.len(), 1);
    let summary = store
        .apply_consolidation(&clusters[0], "Signed artifacts require approval.")
        .unwrap();
    assert_eq!(summary.tier, "episodic");
    assert!(!summary.stale);
    assert!(store.get_memory(&first.id).unwrap().unwrap().stale);
    assert!(store.get_memory(&second.id).unwrap().unwrap().stale);
    assert!(!store.get_memory(&other.id).unwrap().unwrap().stale);
    assert_eq!(
        store.list_memory_links(&first.id).unwrap()[0]
            .relation
            .as_deref(),
        Some("consolidated_into")
    );
    assert_eq!(
        store.list_memory_links(&second.id).unwrap()[0].to_memory_id,
        summary.id
    );
}
