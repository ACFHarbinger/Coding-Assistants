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
        let sign = if ((hash >> 32) & 1) == 0 {
            1.0f32
        } else {
            -1.0f32
        };
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
#[cfg(test)]
pub fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    let (chunks, _rem) = blob.as_chunks::<4>();
    chunks.iter().map(|&arr| f32::from_le_bytes(arr)).collect()
}

/// Compute cosine similarity between two normalized vectors.
#[cfg(test)]
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
        self.conn.execute(
            "DELETE FROM memory_vectors WHERE memory_id = ?1",
            params![memory_id],
        )?;
        self.conn.execute(
            "INSERT INTO memory_vectors (embedding, memory_id) VALUES (?1, ?2)",
            params![blob, memory_id],
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
        self.search_memories_semantic_impl(query, limit, scope, tier, workspace_path, None)
    }

    fn search_memories_semantic_impl(
        &self,
        query: &str,
        limit: usize,
        scope: Option<MemoryScope>,
        tier: Option<MemoryTier>,
        workspace_path: Option<&str>,
        tool: Option<&str>,
    ) -> Result<Vec<(MemoryRecord, f32)>, HubError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(HubError::Invalid(
                "semantic search query must not be empty".into(),
            ));
        }

        let query_vector = compute_embedding(trimmed);

        let candidate_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM memory_vectors", [], |row| row.get(0))?;
        if candidate_count == 0 {
            return Ok(Vec::new());
        }
        let mut sql = String::from(
            r#"
            WITH nearest AS MATERIALIZED (
                SELECT memory_id, distance
                FROM memory_vectors
                WHERE embedding MATCH ? AND k = ?
                ORDER BY distance
            )
            SELECT m.id, m.scope, m.workspace_path, m.tier, m.agent_id, m.title, m.body,
                   m.tags_json, m.created_at, m.updated_at, m.stale, m.source_event_id, m.tool,
                   nearest.distance
            FROM nearest
            JOIN memories m ON m.id = nearest.memory_id
            WHERE m.stale = 0
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
            Box::new(vector_to_blob(&query_vector)),
            Box::new(candidate_count),
        ];

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
        if let Some(tool) = tool {
            sql.push_str(" AND m.tool = ?");
            params_vec.push(Box::new(tool.to_string()));
        }
        sql.push_str(" ORDER BY nearest.distance LIMIT ?");
        params_vec.push(Box::new(limit.max(1) as i64));

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
                tool: r.get(12)?,
            };
            let distance: f32 = r.get(13)?;
            Ok((rec, distance))
        })?;
        Ok(rows
            .filter_map(|row| match row {
                Ok((record, distance)) => {
                    let similarity = (1.0 - distance).clamp(-1.0, 1.0);
                    (similarity > 0.05).then_some(Ok((record, similarity)))
                }
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, _>>()?)
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
        self.search_memories_hybrid_impl(query, limit, scope, tier, workspace_path, None)
    }

    fn search_memories_hybrid_impl(
        &self,
        query: &str,
        limit: usize,
        scope: Option<MemoryScope>,
        tier: Option<MemoryTier>,
        workspace_path: Option<&str>,
        tool: Option<&str>,
    ) -> Result<Vec<(MemoryRecord, f32)>, HubError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(HubError::Invalid(
                "hybrid search query must not be empty".into(),
            ));
        }

        // 1. Semantic search results
        let semantic_results = self.search_memories_semantic_impl(
            trimmed,
            limit * 2,
            scope,
            tier,
            workspace_path,
            tool,
        )?;

        // 2. Lexical search results
        let lexical_candidates = self.search_memories_impl(trimmed, tool)?;
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

    pub fn search_memories_semantic_with_tool(
        &self,
        query: &str,
        limit: usize,
        scope: Option<MemoryScope>,
        tier: Option<MemoryTier>,
        workspace_path: Option<&str>,
        tool: Option<&str>,
    ) -> Result<Vec<(MemoryRecord, f32)>, HubError> {
        self.search_memories_semantic_impl(query, limit, scope, tier, workspace_path, tool)
    }

    pub fn search_memories_hybrid_with_tool(
        &self,
        query: &str,
        limit: usize,
        scope: Option<MemoryScope>,
        tier: Option<MemoryTier>,
        workspace_path: Option<&str>,
        tool: Option<&str>,
    ) -> Result<Vec<(MemoryRecord, f32)>, HubError> {
        self.search_memories_hybrid_impl(query, limit, scope, tier, workspace_path, tool)
    }

    /// Retrieves only memories safe to share with an agent working in a
    /// workspace: records scoped to that workspace plus global records.
    /// Records belonging to another workspace are deliberately excluded.
    pub fn search_memories_for_workspace_recall(
        &self,
        query: &str,
        limit: usize,
        workspace_path: &str,
    ) -> Result<Vec<(MemoryRecord, f32)>, HubError> {
        let fetch_limit = limit.max(1);
        // One fused ranking pass over every scope, then drop records owned by
        // another workspace. Merging two independent `search_memories_hybrid`
        // calls by raw score is invalid: RRF scores encode rank *within their
        // own result set*, so the top hit of the workspace call and the top
        // hit of the global call collide at the same score regardless of true
        // relevance, and a weak global memory can outrank a strong local one.
        // Over-fetch (4x, floor 32) so that after dropping other workspaces'
        // records there are still enough candidates to fill `fetch_limit`.
        let ranked = self.search_memories_hybrid(
            query,
            fetch_limit.saturating_mul(4).max(32),
            None,
            None,
            None,
        )?;
        let mut hits: Vec<(MemoryRecord, f32)> = ranked
            .into_iter()
            .filter(|(record, _)| {
                record.scope == MemoryScope::Global.as_str()
                    || (record.scope == MemoryScope::Workspace.as_str()
                        && record.workspace_path.as_deref() == Some(workspace_path))
            })
            .collect();
        hits.truncate(fetch_limit);
        Ok(hits)
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
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let count = unindexed.len();
        for (id, title, body, tags_json) in unindexed {
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            self.upsert_memory_vector(&id, title.as_deref(), &body, &tags)?;
        }

        Ok(count)
    }
}
