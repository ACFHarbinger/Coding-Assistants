use super::*;

#[test]
fn tauri_semantic_and_hybrid_memory_search() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-vector-test-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);

    let store = open_store().expect("open_store should create the hub dir");
    let mem1 = store
        .write_memory(
            MemoryTier::Semantic,
            MemoryScope::Workspace,
            Some("claude"),
            Some("/work/repo"),
            Some("Vector Embeddings Architecture"),
            "Dense 384-dimensional feature hashing with cosine similarity ranking.",
            &["vector".into(), "embeddings".into()],
        )
        .expect("write_memory should succeed");

    let _mem2 = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Workspace,
            Some("grok"),
            Some("/work/repo"),
            Some("Terminal Color Palette"),
            "ANSI theme switcher and ratatui UI layout styling.",
            &["tui".into(), "theme".into()],
        )
        .expect("write_memory should succeed");

    // 1. Semantic search
    let semantic_hits = store
        .search_memories_semantic("dense feature hashing embeddings", 10, None, None, None)
        .expect("semantic search should succeed");

    assert!(!semantic_hits.is_empty());
    assert_eq!(semantic_hits[0].0.id, mem1.id);
    assert!(semantic_hits[0].1 > 0.1);

    // 2. Hybrid search
    let hybrid_hits = store
        .search_memories_hybrid("vector embeddings", 10, None, None, None)
        .expect("hybrid search should succeed");

    assert!(!hybrid_hits.is_empty());
    assert_eq!(hybrid_hits[0].0.id, mem1.id);

    // 3. Reindex
    let reindexed = store
        .reindex_memory_vectors()
        .expect("reindex should succeed");
    assert_eq!(reindexed, 0);

    std::env::remove_var("CA_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}
