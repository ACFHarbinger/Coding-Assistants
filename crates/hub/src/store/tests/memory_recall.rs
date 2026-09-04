use super::super::*;
use tempfile::tempdir;

#[test]
fn workspace_recall_includes_global_but_excludes_other_workspaces() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Workspace,
            Some("claude"),
            Some("/repo/a"),
            Some("Workspace release decision"),
            "release decision uses a signed artifact",
            &[],
        )
        .unwrap();
    let global = store
        .write_memory(
            MemoryTier::Semantic,
            MemoryScope::Global,
            Some("human"),
            None,
            Some("Global release policy"),
            "release policy requires approval",
            &[],
        )
        .unwrap();
    let other_workspace = store
        .write_memory(
            MemoryTier::ShortTerm,
            MemoryScope::Workspace,
            Some("claude"),
            Some("/repo/b"),
            Some("Other workspace release note"),
            "release decision for an unrelated workspace",
            &[],
        )
        .unwrap();

    let hits = store
        .search_memories_for_workspace_recall("release decision", 5, "/repo/a")
        .unwrap();
    let ids = hits
        .iter()
        .map(|(record, _)| record.id.as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&workspace.id.as_str()));
    assert!(ids.contains(&global.id.as_str()));
    assert!(!ids.contains(&other_workspace.id.as_str()));
}
