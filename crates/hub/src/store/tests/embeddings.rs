use super::super::models::embeddings::{
    blob_to_vector, compute_embedding, cosine_similarity, vector_to_blob, VECTOR_DIMENSIONS,
};
use super::super::*;

#[test]
fn embedding_is_deterministic_and_normalized() {
    let text = "Architectural decision on SQLite vectors and IPC";
    let v1 = compute_embedding(text);
    let v2 = compute_embedding(text);
    assert_eq!(v1.len(), VECTOR_DIMENSIONS);
    assert_eq!(v1, v2);

    let norm: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-4, "norm is {norm}");
}

#[test]
fn vector_blob_round_trips() {
    let vec = vec![0.12345f32, -0.98765f32, 1.0f32];
    assert_eq!(vec, blob_to_vector(&vector_to_blob(&vec)));
}

#[test]
fn related_texts_have_higher_cosine_similarity() {
    let query = compute_embedding("Rust vector search sqlite");
    let similar =
        compute_embedding("Vector database embedding indexing in SQLite database with Rust");
    let unrelated = compute_embedding("Watercolor painting canvas brush acrylic colors");
    assert!(cosine_similarity(&query, &similar) > cosine_similarity(&query, &unrelated));
}

#[test]
fn vec0_search_and_reindex_memory_vectors() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let embedding_model: String = store
        .conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'embedding_model'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(embedding_model, "minilm-l6-v2");
    let mem1 = store
        .write_memory(
            MemoryTier::Semantic,
            MemoryScope::Workspace,
            Some("claude"),
            Some("/repo/coding"),
            Some("SQLite Vector Indexing"),
            "Implemented vector similarity search using cosine distance and dense embeddings.",
            &["vector".into(), "sqlite".into()],
        )
        .unwrap();
    let mem2 = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Workspace,
            Some("grok"),
            Some("/repo/coding"),
            Some("Frontend Glass Theme"),
            "Redesigned the React glassmorphism navigation tabs with translucent backgrounds.",
            &["ui".into(), "css".into()],
        )
        .unwrap();

    let semantic_hits = store
        .search_memories_semantic("embeddings and vector similarity", 5, None, None, None)
        .unwrap();
    assert_eq!(semantic_hits[0].0.id, mem1.id);
    assert!(semantic_hits[0].1 > 0.1);
    let hybrid_hits = store
        .search_memories_hybrid("vector similarity search", 5, None, None, None)
        .unwrap();
    assert_eq!(hybrid_hits[0].0.id, mem1.id);

    store
        .conn
        .execute(
            "DELETE FROM memory_vectors WHERE memory_id = ?1",
            rusqlite::params![mem2.id],
        )
        .unwrap();
    assert_eq!(store.reindex_memory_vectors().unwrap(), 1);
}

#[test]
fn mixed_embedding_marker_forces_a_clean_rebuild_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .conn
        .execute(
            "UPDATE meta SET value = 'mixed' WHERE key = 'embedding_model'",
            [],
        )
        .unwrap();
    drop(store);

    let reopened = HubStore::open(dir.path()).unwrap();
    let marker: String = reopened
        .conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'embedding_model'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marker, "minilm-l6-v2");
}

#[test]
fn opening_a_legacy_blob_store_rebuilds_vec0() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("hub.db");
    let legacy = rusqlite::Connection::open(&db_path).unwrap();
    legacy
        .execute_batch(
            r#"
            CREATE TABLE memories (
                id TEXT PRIMARY KEY NOT NULL, scope TEXT NOT NULL, workspace_path TEXT,
                tier TEXT NOT NULL, agent_id TEXT, title TEXT, body TEXT NOT NULL,
                tags_json TEXT NOT NULL DEFAULT '[]', created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL, stale INTEGER NOT NULL DEFAULT 0,
                source_event_id TEXT
            );
            CREATE TABLE memory_vectors (
                memory_id TEXT PRIMARY KEY NOT NULL, dimensions INTEGER NOT NULL,
                vector_blob BLOB NOT NULL, created_at TEXT NOT NULL
            );
            INSERT INTO memories VALUES (
                'legacy-memory', 'global', NULL, 'semantic', 'human', 'Legacy vector',
                'sqlite vec migration keeps this memory searchable', '[]', '2026-01-01',
                '2026-01-01', 0, NULL
            );
            "#,
        )
        .unwrap();
    drop(legacy);

    let store = HubStore::open(dir.path()).unwrap();
    let table_sql: String = store
        .conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'memory_vectors'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(table_sql.contains("VIRTUAL TABLE"));
    assert_eq!(
        store
            .search_memories_semantic("sqlite vec migration", 1, None, None, None)
            .unwrap()[0]
            .0
            .id,
        "legacy-memory"
    );
}
