use super::super::*;
use tempfile::tempdir;

#[test]
fn tool_scoped_memory_filters_preserve_legacy_rows() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let legacy = store
        .write_memory(
            MemoryTier::Semantic,
            MemoryScope::Global,
            Some("human"),
            None,
            Some("Legacy note"),
            "shared rendering workflow",
            &[],
        )
        .unwrap();
    let blender = store
        .write_memory_with_tool(
            MemoryTier::Semantic,
            MemoryScope::Global,
            Some("gemini"),
            None,
            Some("Blender workflow"),
            "shared rendering workflow",
            &[],
            Some("blender"),
        )
        .unwrap();
    let krita = store
        .write_memory_with_tool(
            MemoryTier::Semantic,
            MemoryScope::Global,
            Some("gemini"),
            None,
            Some("Krita workflow"),
            "shared rendering workflow",
            &[],
            Some("krita"),
        )
        .unwrap();

    assert_eq!(store.get_memory(&legacy.id).unwrap().unwrap().tool, None);
    assert_eq!(
        store
            .get_memory(&blender.id)
            .unwrap()
            .unwrap()
            .tool
            .as_deref(),
        Some("blender")
    );
    let exact = store
        .search_memories_with_tool("rendering workflow", Some("blender"))
        .unwrap();
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].id, blender.id);
    for hits in [
        store
            .search_memories_semantic_with_tool(
                "rendering workflow",
                10,
                None,
                None,
                None,
                Some("blender"),
            )
            .unwrap(),
        store
            .search_memories_hybrid_with_tool(
                "rendering workflow",
                10,
                None,
                None,
                None,
                Some("blender"),
            )
            .unwrap(),
    ] {
        assert!(hits
            .iter()
            .all(|(memory, _)| memory.tool.as_deref() == Some("blender")));
        assert!(hits.iter().any(|(memory, _)| memory.id == blender.id));
        assert!(!hits.iter().any(|(memory, _)| memory.id == krita.id));
    }
}
