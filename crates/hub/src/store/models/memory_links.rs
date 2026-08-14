use super::super::*;
use crate::settings::LinkSuggestionMode;
use std::collections::{BTreeMap, HashSet};

/// Grouping key `memories_for_topic` uses for memories with no `agent_id`
/// (`MemoryRecord::agent_id` is nullable). A plain string, not `""`, so it
/// sorts predictably and reads unambiguously in a raw JSON dump.
pub const UNATTRIBUTED_AUTHOR: &str = "unattributed";

/// A scored candidate produced by `suggest_links_for_memory`. Not a
/// `memory_links` row — nothing is written until a caller (or `Auto` mode)
/// turns a suggestion into a real `link_memories` call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkSuggestion {
    pub candidate: MemoryRecord,
    /// 0.0..=1.0. Not calibrated against any ground truth — a first-pass
    /// heuristic weight, tuned to be conservative rather than aggressive.
    /// Treat the exact numbers as ordering, not probability.
    pub score: f64,
    /// Human-readable explanation ("shared tags: checkout, payments"),
    /// not a machine-parsed field — this exists so a `Suggest`-mode UI can
    /// show *why* without the caller re-deriving it.
    pub reason: String,
}

/// A suggestion at or above this score is created automatically under
/// `LinkSuggestionMode::Auto`. False *positives* here are much worse than
/// false negatives, since an unwanted auto-drawn edge undermines the "who
/// actually noticed this" property the whole feature exists for — so this
/// stays conservative, but calibrated against real scores, not a guessed
/// round number: a smoke test of two obviously-related short memories
/// (shared tag, four shared meaningful terms, same technical decision)
/// scored 0.39-0.42 with this scorer. An initial guess of 0.55 would have
/// silently never fired on real data. Re-calibrate here, with a fresh real
/// example, if the scorer's weights ever change.
const AUTO_ACCEPT_THRESHOLD: f64 = 0.35;

/// Below this score, a candidate isn't worth returning even as a `Suggest`.
const MIN_SUGGEST_SCORE: f64 = 0.15;

const TAG_WEIGHT: f64 = 0.6;
const TOKEN_WEIGHT: f64 = 0.4;

/// Tokens too common to carry topical signal. Small and deliberately
/// English-only for a first pass — this is a heuristic prefilter, not a
/// claim of linguistic correctness.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "was", "were", "that", "this", "with", "from", "into", "onto",
    "have", "has", "had", "not", "but", "you", "your", "our", "their", "its", "it's", "will",
    "would", "should", "could", "can", "may", "might", "must", "than", "then", "when", "what",
    "which", "who", "whom", "these", "those", "there", "here", "over", "under", "about", "also",
];

/// Lowercased, stopword-and-short-token-filtered word set for Jaccard
/// comparison. Not a real tokenizer (no stemming, no multi-word phrases) —
/// intentionally simple; swap for embeddings later without changing
/// `suggest_links_for_memory`'s signature (same contract `memories_for_topic`
/// already documents for its own LIKE-based matching).
fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3 && !STOPWORDS.contains(w))
        .map(str::to_string)
        .collect()
}

fn jaccard<T: std::hash::Hash + Eq>(a: &HashSet<T>, b: &HashSet<T>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn tags_from_json(tags_json: &str) -> HashSet<String> {
    serde_json::from_str::<Vec<String>>(tags_json)
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.to_lowercase())
        .collect()
}

impl HubStore {
    /// Draw a directed edge between two existing memories. `relation` is
    /// freeform (see `MemoryLinkRecord` doc comment) — pass `None` for a
    /// plain "related" edge. Multiple edges between the same pair (even with
    /// the same `relation`) are allowed on purpose: two different observers
    /// independently drawing the same connection is signal, not a duplicate.
    pub fn link_memories(
        &self,
        from_memory_id: &str,
        to_memory_id: &str,
        relation: Option<&str>,
        created_by: &str,
    ) -> Result<MemoryLinkRecord, HubError> {
        if from_memory_id == to_memory_id {
            return Err(HubError::Invalid(
                "a memory cannot be linked to itself".into(),
            ));
        }
        if self.get_memory(from_memory_id)?.is_none() {
            return Err(HubError::NotFound(from_memory_id.to_string()));
        }
        if self.get_memory(to_memory_id)?.is_none() {
            return Err(HubError::NotFound(to_memory_id.to_string()));
        }
        if created_by.trim().is_empty() {
            return Err(HubError::Invalid("created_by must not be empty".into()));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO memory_links(id, from_memory_id, to_memory_id, relation, created_by, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![id, from_memory_id, to_memory_id, relation, created_by, now],
        )?;

        self.get_memory_link(&id)?
            .ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_memory_link(&self, id: &str) -> Result<Option<MemoryLinkRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, from_memory_id, to_memory_id, relation, created_by, created_at
            FROM memory_links WHERE id = ?1
            "#,
        )?;
        let row = stmt
            .query_row(params![id], Self::row_to_memory_link)
            .optional()?;
        Ok(row)
    }

    pub fn unlink_memories(&self, link_id: &str) -> Result<(), HubError> {
        let n = self
            .conn
            .execute("DELETE FROM memory_links WHERE id = ?1", params![link_id])?;
        if n == 0 {
            return Err(HubError::NotFound(link_id.to_string()));
        }
        Ok(())
    }

    /// All links touching a memory, in either direction.
    pub fn list_memory_links(&self, memory_id: &str) -> Result<Vec<MemoryLinkRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, from_memory_id, to_memory_id, relation, created_by, created_at
            FROM memory_links
            WHERE from_memory_id = ?1 OR to_memory_id = ?1
            ORDER BY created_at DESC
            "#,
        )?;
        let rows = stmt.query_map(params![memory_id], Self::row_to_memory_link)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Walk the link graph outward from one memory, up to `depth` hops
    /// (`depth = 1` returns only directly-linked memories). Direction is
    /// ignored during the walk — `memory_links` rows are directed for
    /// `relation` semantics, but "is this related" is symmetric for
    /// traversal purposes. Stale memories are excluded, matching
    /// `list_memories`'s default. Cycles can't cause infinite recursion:
    /// `UNION` (not `UNION ALL`) dedupes visited ids as part of the walk.
    pub fn related_memories(
        &self,
        memory_id: &str,
        depth: u8,
    ) -> Result<Vec<MemoryRecord>, HubError> {
        if depth == 0 {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            r#"
            WITH RECURSIVE walk(memory_id, hops) AS (
                SELECT ?1, 0
                UNION
                SELECT
                    CASE WHEN l.from_memory_id = w.memory_id
                         THEN l.to_memory_id ELSE l.from_memory_id END,
                    w.hops + 1
                FROM memory_links l
                JOIN walk w
                  ON l.from_memory_id = w.memory_id OR l.to_memory_id = w.memory_id
                WHERE w.hops < ?2
            )
            SELECT DISTINCT
                m.id, m.scope, m.workspace_path, m.tier, m.agent_id, m.title, m.body,
                m.tags_json, m.created_at, m.updated_at, m.stale, m.source_event_id
            FROM memories m
            JOIN walk w ON w.memory_id = m.id
            WHERE w.memory_id != ?1 AND m.stale = 0
            ORDER BY m.created_at DESC
            "#,
        )?;
        let rows = stmt.query_map(params![memory_id, depth as i64], Self::row_to_memory)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Memories matching a tag/text search, grouped by `agent_id` — the
    /// "everyone's view of this topic" browse query. Memories with no author
    /// (`agent_id` is nullable on `MemoryRecord`) are grouped under
    /// [`UNATTRIBUTED_AUTHOR`] rather than as a map key of `None`: serde_json
    /// can only serialize string-keyed maps, and `BTreeMap<Option<String>, _>`
    /// compiles but panics at `to_string_pretty` time with "key must be a
    /// string" — every JSON-facing consumer (CLI, Tauri IPC) would otherwise
    /// have had to work around that independently. Reuses `search_memories`'s
    /// LIKE-based matching rather than duplicating it; swap the inner call
    /// out once semantic search lands without changing this method's
    /// signature.
    pub fn memories_for_topic(
        &self,
        query: &str,
    ) -> Result<BTreeMap<String, Vec<MemoryRecord>>, HubError> {
        let mut grouped: BTreeMap<String, Vec<MemoryRecord>> = BTreeMap::new();
        for memory in self.search_memories(query)? {
            let key = memory
                .agent_id
                .clone()
                .unwrap_or_else(|| UNATTRIBUTED_AUTHOR.to_string());
            grouped
                .entry(key)
                .or_default()
                .push(memory);
        }
        Ok(grouped)
    }

    /// Score every other non-stale memory against `memory_id` on tag overlap
    /// (Jaccard over `tags_json`) and title+body token overlap (Jaccard over
    /// a stopword-filtered word set), and return the top `limit` candidates
    /// scoring at or above [`MIN_SUGGEST_SCORE`], highest first. Pure and
    /// read-only — creates no `memory_links` rows. Candidate pool is bounded
    /// by `list_memories`'s existing `LIMIT 200`, same as every other
    /// browse-style query in this store; this is a personal-scale tool, not
    /// a corpus search engine, and re-scanning is O(pool), not O(table).
    pub fn suggest_links_for_memory(
        &self,
        memory_id: &str,
        limit: usize,
    ) -> Result<Vec<LinkSuggestion>, HubError> {
        let source = self
            .get_memory(memory_id)?
            .ok_or_else(|| HubError::NotFound(memory_id.to_string()))?;
        let source_tags = tags_from_json(&source.tags_json);
        let source_tokens = tokenize(&format!(
            "{} {}",
            source.title.as_deref().unwrap_or(""),
            source.body
        ));

        let already_linked: HashSet<String> = self
            .list_memory_links(memory_id)?
            .into_iter()
            .flat_map(|l| [l.from_memory_id, l.to_memory_id])
            .collect();

        let mut scored: Vec<LinkSuggestion> = self
            .list_memories(None, None, None, false)?
            .into_iter()
            .filter(|m| m.id != memory_id && !already_linked.contains(&m.id))
            .filter_map(|candidate| {
                let candidate_tags = tags_from_json(&candidate.tags_json);
                let tag_score = jaccard(&source_tags, &candidate_tags);
                let candidate_tokens = tokenize(&format!(
                    "{} {}",
                    candidate.title.as_deref().unwrap_or(""),
                    candidate.body
                ));
                let token_score = jaccard(&source_tokens, &candidate_tokens);
                let score = TAG_WEIGHT * tag_score + TOKEN_WEIGHT * token_score;
                if score < MIN_SUGGEST_SCORE {
                    return None;
                }

                let mut reasons = Vec::new();
                if tag_score > 0.0 {
                    let shared: Vec<&str> = source_tags
                        .intersection(&candidate_tags)
                        .map(String::as_str)
                        .collect();
                    reasons.push(format!("shared tags: {}", shared.join(", ")));
                }
                if token_score > 0.0 {
                    let mut shared: Vec<&str> = source_tokens
                        .intersection(&candidate_tokens)
                        .map(String::as_str)
                        .collect();
                    shared.sort_unstable();
                    shared.truncate(5);
                    reasons.push(format!("shared terms: {}", shared.join(", ")));
                }

                Some(LinkSuggestion {
                    candidate,
                    score,
                    reason: reasons.join("; "),
                })
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    /// Apply [`LinkSuggestionMode`]'s policy for a memory that was just
    /// written: `Off` does nothing and returns an empty list without even
    /// scoring (cheapest path for the common case); `Suggest` scores and
    /// returns candidates but creates no edges, leaving the decision to
    /// whoever's holding the candidate list; `Auto` scores, creates a real
    /// `link_memories` edge (`created_by = "system:auto-link"`, never the
    /// triggering memory's own author — see `MemoryLinkRecord`'s doc comment)
    /// for every candidate at or above [`AUTO_ACCEPT_THRESHOLD`], and still
    /// returns the *full* candidate list (including sub-threshold ones) so
    /// the caller can show what was auto-linked versus merely considered.
    ///
    /// `HubStore` doesn't read `SettingsStore` itself — deliberately, to
    /// keep the SQLite store and the TOML settings store decoupled, matching
    /// how every other store method takes typed parameters instead of
    /// reaching for global config. The caller (CLI/IPC layer) is the one
    /// that knows the current effective `LinkSuggestionMode` and passes it in.
    pub fn apply_link_suggestions(
        &self,
        memory_id: &str,
        mode: LinkSuggestionMode,
        limit: usize,
    ) -> Result<Vec<LinkSuggestion>, HubError> {
        if mode == LinkSuggestionMode::Off {
            return Ok(Vec::new());
        }
        let suggestions = self.suggest_links_for_memory(memory_id, limit)?;
        if mode == LinkSuggestionMode::Auto {
            for suggestion in &suggestions {
                if suggestion.score >= AUTO_ACCEPT_THRESHOLD {
                    self.link_memories(memory_id, &suggestion.candidate.id, None, "system:auto-link")?;
                }
            }
        }
        Ok(suggestions)
    }

    fn row_to_memory_link(r: &rusqlite::Row) -> rusqlite::Result<MemoryLinkRecord> {
        Ok(MemoryLinkRecord {
            id: r.get(0)?,
            from_memory_id: r.get(1)?,
            to_memory_id: r.get(2)?,
            relation: r.get(3)?,
            created_by: r.get(4)?,
            created_at: r.get(5)?,
        })
    }

    fn row_to_memory(r: &rusqlite::Row) -> rusqlite::Result<MemoryRecord> {
        Ok(MemoryRecord {
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
        })
    }
}
