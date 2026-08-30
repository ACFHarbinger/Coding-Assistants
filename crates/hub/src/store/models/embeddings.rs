//! Semantic / Vector embedding and retrieval for shared memory (M1).
//!
//! Provides deterministic 384-dimensional dense semantic feature hashing,
//! $L_2$ normalized vector storage in SQLite (`memory_vectors`),
//! cosine-similarity semantic retrieval, hybrid (Reciprocal Rank Fusion)
//! search, and backfill reindexing.

use super::super::*;
use std::collections::HashMap;

pub const VECTOR_DIMENSIONS: usize = 384;

/// Computes a deterministic 384-dimensional dense semantic embedding vector
/// using token unigram, token bigram, and subword character n-gram feature hashing.
pub fn compute_embedding(text: &str) -> Vec<f32> {
    let mut vec = vec![0.0f32; VECTOR_DIMENSIONS];
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return vec;
    }

    let mut feature_counts: HashMap<String, f32> = HashMap::new();

    // 1. Tokenize into lowercase words and code identifiers
    let words: Vec<String> = cleaned
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect();

    // 2. Token unigrams and bigrams
    for (i, word) in words.iter().enumerate() {
        *feature_counts.entry(format!("w:{}", word)).or_insert(0.0) += 1.0;

        // Sub-word character 3-grams and 4-grams (captures stems, typos, compounding)
        let chars: Vec<char> = word.chars().collect();
        if chars.len() >= 3 {
            for window in chars.windows(3) {
                let gram: String = window.iter().collect();
                *feature_counts.entry(format!("c3:{}", gram)).or_insert(0.0) += 0.5;
            }
        }
        if chars.len() >= 4 {
            for window in chars.windows(4) {
                let gram: String = window.iter().collect();
                *feature_counts.entry(format!("c4:{}", gram)).or_insert(0.0) += 0.75;
            }
        }

        // Bigram
        if i + 1 < words.len() {
            let bigram = format!("b:{}_{}", word, words[i + 1]);
            *feature_counts.entry(bigram).or_insert(0.0) += 1.25;
        }
    }

    // 3. Hash features into dense vector with sublinear frequency scaling
    for (feature, count) in feature_counts {
        let weight = (1.0 + count.ln()).max(0.1);
        let hash = fnv1a_hash(feature.as_bytes());
        let dim = (hash as usize) % VECTOR_DIMENSIONS;
        let sign = if ((hash >> 32) & 1) == 0 { 1.0f32 } else { -1.0f32 };
        vec[dim] += sign * weight;
    }

    // 4. L2 Normalization
    let sum_sq: f32 = vec.iter().map(|&x| x * x).sum();
    let norm = sum_sq.sqrt();
    if norm > 1e-6 {
        for val in vec.iter_mut() {
            *val /= norm;
        }
    }

    vec
}

/// 64-bit FNV-1a hash function for fast, deterministic feature hashing.
fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3u64);
    }
    hash
}

/// Serialize float slice to byte blob (little-endian).
pub fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for &val in vector {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

/// Deserialize byte blob to float vector.
pub fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
            f32::from_le_bytes(arr)
        })
        .collect()
}

/// Compute cosine similarity between two normalized vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum();
    dot.clamp(-1.0, 1.0)
}

impl HubStore {
    /// Upserts an embedding vector for a memory.
    pub fn upsert_memory_vector(
        &self,
        memory_id: &str,
        title: Option<&str>,
        body: &str,
        tags: &[String],
    ) -> Result<(), HubError> {
        let mut text = String::new();
        if let Some(t) = title {
            text.push_str(t);
            text.push(' ');
        }
        text.push_str(body);
        if !tags.is_empty() {
            text.push(' ');
            text.push_str(&tags.join(" "));
        }

        let vector = compute_embedding(&text);
        let blob = vector_to_blob(&vector);
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO memory_vectors (memory_id, dimensions, vector_blob, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(memory_id) DO UPDATE SET
                dimensions = excluded.dimensions,
                vector_blob = excluded.vector_blob,
                created_at = excluded.created_at
            "#,
            params![memory_id, VECTOR_DIMENSIONS as i64, blob, now],
        )?;

        Ok(())
    }

    /// Performs semantic vector search on memories using cosine similarity.
    pub fn search_memories_semantic(
        &self,
        query: &str,
        limit: usize,
        scope: Option<MemoryScope>,
        tier: Option<MemoryTier>,
        workspace_path: Option<&str>,
    ) -> Result<Vec<(MemoryRecord, f32)>, HubError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(HubError::Invalid("semantic search query must not be empty".into()));
        }

        let query_vector = compute_embedding(trimmed);

        let mut sql = String::from(
            r#"
            SELECT m.id, m.scope, m.workspace_path, m.tier, m.agent_id, m.title, m.body,
                   m.tags_json, m.created_at, m.updated_at, m.stale, m.source_event_id,
                   v.vector_blob
            FROM memories m
            JOIN memory_vectors v ON m.id = v.memory_id
            WHERE m.stale = 0
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(s) = scope {
            sql.push_str(" AND m.scope = ?");
            params_vec.push(Box::new(s.as_str().to_string()));
        }
        if let Some(t) = tier {
            sql.push_str(" AND m.tier = ?");
            params_vec.push(Box::new(t.as_str().to_string()));
        }
        if let Some(ws) = workspace_path {
            sql.push_str(" AND m.workspace_path = ?");
            params_vec.push(Box::new(ws.to_string()));
        }

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            let rec = MemoryRecord {
                id: r.get(0)?,
                scope: r.get(1)?,
                workspace_path: r.get(2)?,
                tier: r.get(3)?,
                agent_id: r.get(4)?,
                title: r.get(5)?,
                body: r.get(6)?,
                tags_json: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
                stale: r.get::<_, i64>(10)? != 0,
                source_event_id: r.get(11)?,
            };
            let blob: Vec<u8> = r.get(12)?;
            Ok((rec, blob))
        })?;

        let mut scored: Vec<(MemoryRecord, f32)> = Vec::new();
        for item in rows {
            let (rec, blob) = item?;
            let vec = blob_to_vector(&blob);
            let sim = cosine_similarity(&query_vector, &vec);
            if sim > 0.05 {
                scored.push((rec, sim));
            }
        }

        // Sort descending by cosine similarity score
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit.max(1));

        Ok(scored)
    }

    /// Performs hybrid search combining lexical (text match) and semantic (vector)
    /// retrieval via Reciprocal Rank Fusion (RRF).
    pub fn search_memories_hybrid(
        &self,
        query: &str,
        limit: usize,
        scope: Option<MemoryScope>,
        tier: Option<MemoryTier>,
        workspace_path: Option<&str>,
    ) -> Result<Vec<(MemoryRecord, f32)>, HubError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(HubError::Invalid("hybrid search query must not be empty".into()));
        }

        // 1. Semantic search results
        let semantic_results = self.search_memories_semantic(
            trimmed,
            limit * 2,
            scope,
            tier,
            workspace_path,
        )?;

        // 2. Lexical search results
        let lexical_candidates = self.search_memories(trimmed)?;
        let filtered_lexical: Vec<MemoryRecord> = lexical_candidates
            .into_iter()
            .filter(|m| {
                if let Some(s) = scope {
                    if m.scope != s.as_str() {
                        return false;
                    }
                }
                if let Some(t) = tier {
                    if m.tier != t.as_str() {
                        return false;
                    }
                }
                if let Some(ws) = workspace_path {
                    if m.workspace_path.as_deref() != Some(ws) {
                        return false;
                    }
                }
                true
            })
            .take(limit * 2)
            .collect();

        // 3. Reciprocal Rank Fusion
        const RRF_K: f32 = 60.0;
        let mut rrf_scores: HashMap<String, (MemoryRecord, f32)> = HashMap::new();

        for (rank, rec) in filtered_lexical.into_iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f32 + 1.0);
            rrf_scores.insert(rec.id.clone(), (rec, score));
        }

        for (rank, (rec, _sim)) in semantic_results.into_iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f32 + 1.0);
            if let Some(entry) = rrf_scores.get_mut(&rec.id) {
                entry.1 += score;
            } else {
                rrf_scores.insert(rec.id.clone(), (rec, score));
            }
        }

        let mut fused: Vec<(MemoryRecord, f32)> = rrf_scores.into_values().collect();
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        fused.truncate(limit.max(1));

        Ok(fused)
    }

    /// Backfills missing memory embeddings in `memory_vectors`.
    pub fn reindex_memory_vectors(&self) -> Result<usize, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT m.id, m.title, m.body, m.tags_json
            FROM memories m
            LEFT JOIN memory_vectors v ON m.id = v.memory_id
            WHERE v.memory_id IS NULL
            "#,
        )?;

        let unindexed: Vec<(String, Option<String>, String, String)> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let count = unindexed.len();
        for (id, title, body, tags_json) in unindexed {
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            self.upsert_memory_vector(&id, title.as_deref(), &body, &tags)?;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let blob = vector_to_blob(&vec);
        let restored = blob_to_vector(&blob);
        assert_eq!(vec, restored);
    }

    #[test]
    fn related_texts_have_higher_cosine_similarity() {
        let query = compute_embedding("Rust vector search sqlite");
        let similar = compute_embedding("Vector database embedding indexing in SQLite database with Rust");
        let unrelated = compute_embedding("Watercolor painting canvas brush acrylic colors");

        let sim_related = cosine_similarity(&query, &similar);
        let sim_unrelated = cosine_similarity(&query, &unrelated);

        assert!(
            sim_related > sim_unrelated,
            "sim_related ({sim_related}) should exceed sim_unrelated ({sim_unrelated})"
        );
    }

    #[test]
    fn store_semantic_and_hybrid_search_and_reindex() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

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

        // 1. Semantic search
        let semantic_hits = store
            .search_memories_semantic("embeddings and vector similarity", 5, None, None, None)
            .unwrap();
        assert!(!semantic_hits.is_empty());
        assert_eq!(semantic_hits[0].0.id, mem1.id);
        assert!(semantic_hits[0].1 > 0.1);

        // 2. Hybrid search
        let hybrid_hits = store
            .search_memories_hybrid("vector similarity search", 5, None, None, None)
            .unwrap();
        assert!(!hybrid_hits.is_empty());
        assert_eq!(hybrid_hits[0].0.id, mem1.id);

        // 3. Reindex
        // Delete a vector directly to test backfill
        store
            .conn
            .execute("DELETE FROM memory_vectors WHERE memory_id = ?1", rusqlite::params![mem2.id])
            .unwrap();
        let reindexed = store.reindex_memory_vectors().unwrap();
        assert_eq!(reindexed, 1);
    }
}
