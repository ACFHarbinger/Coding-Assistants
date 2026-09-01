//! Durable memory commands.
use super::store::open_store;
use hub::{
    CompactReport, LinkSuggestion, LinkSuggestionMode, MemoryLinkRecord, MemoryRecord, MemoryScope,
    MemoryTier,
};
#[derive(serde::Deserialize)]
pub struct WriteMemoryArgs {
    pub tier: String,
    pub scope: String,
    pub agent: Option<String>,
    pub workspace: Option<String>,
    pub title: Option<String>,
    pub body: String,
    pub tags: Option<Vec<String>>,
}

#[tauri::command]
pub fn hub_write_memory(args: WriteMemoryArgs) -> Result<MemoryRecord, String> {
    let store = open_store()?;
    let tier = MemoryTier::parse(&args.tier).map_err(|e| e.to_string())?;
    let scope = MemoryScope::parse(&args.scope).map_err(|e| e.to_string())?;
    let tags = args.tags.unwrap_or_default();
    store
        .write_memory(
            tier,
            scope,
            args.agent.as_deref(),
            args.workspace.as_deref(),
            args.title.as_deref(),
            &args.body,
            &tags,
        )
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct UpdateMemoryArgs {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub tags: Option<Vec<String>>,
}

#[tauri::command]
pub fn hub_update_memory(args: UpdateMemoryArgs) -> Result<MemoryRecord, String> {
    let store = open_store()?;
    let tags = args.tags.as_deref();
    store
        .update_memory(&args.id, args.title.as_deref(), &args.body, tags)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_memories(
    scope: Option<String>,
    tier: Option<String>,
    workspace: Option<String>,
    include_stale: Option<bool>,
) -> Result<Vec<MemoryRecord>, String> {
    let store = open_store()?;
    let scope = scope
        .as_deref()
        .map(MemoryScope::parse)
        .transpose()
        .map_err(|e| e.to_string())?;
    let tier = tier
        .as_deref()
        .map(MemoryTier::parse)
        .transpose()
        .map_err(|e| e.to_string())?;
    store
        .list_memories(
            scope,
            tier,
            workspace.as_deref(),
            include_stale.unwrap_or(false),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_search_memories(query: String) -> Result<Vec<MemoryRecord>, String> {
    open_store()?
        .search_memories(&query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_mark_memory_stale(id: String, stale: bool) -> Result<(), String> {
    open_store()?
        .mark_memory_stale(&id, stale)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_delete_memory(id: String) -> Result<(), String> {
    open_store()?.delete_memory(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_promote_memory(id: String, to_tier: String) -> Result<MemoryRecord, String> {
    let store = open_store()?;
    let to = MemoryTier::parse(&to_tier).map_err(|e| e.to_string())?;
    store.promote_memory(&id, to).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_compact_short_term(keep_newest: Option<usize>) -> Result<CompactReport, String> {
    open_store()?
        .compact_short_term(keep_newest.unwrap_or(50))
        .map_err(|e| e.to_string())
}

// ── memory_links ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct LinkMemoriesArgs {
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub relation: Option<String>,
    pub created_by: String,
}

#[tauri::command]
pub fn hub_link_memories(args: LinkMemoriesArgs) -> Result<MemoryLinkRecord, String> {
    open_store()?
        .link_memories(
            &args.from_memory_id,
            &args.to_memory_id,
            args.relation.as_deref(),
            &args.created_by,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_unlink_memories(link_id: String) -> Result<(), String> {
    open_store()?
        .unlink_memories(&link_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_memory_links(memory_id: String) -> Result<Vec<MemoryLinkRecord>, String> {
    open_store()?
        .list_memory_links(&memory_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_related_memories(
    memory_id: String,
    depth: Option<u8>,
) -> Result<Vec<MemoryRecord>, String> {
    open_store()?
        .related_memories(&memory_id, depth.unwrap_or(1))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_memories_for_topic(
    query: String,
) -> Result<std::collections::BTreeMap<String, Vec<MemoryRecord>>, String> {
    open_store()?
        .memories_for_topic(&query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_suggest_links_for_memory(
    memory_id: String,
    limit: Option<usize>,
) -> Result<Vec<LinkSuggestion>, String> {
    open_store()?
        .suggest_links_for_memory(&memory_id, limit.unwrap_or(10))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_apply_link_suggestions(
    memory_id: String,
    mode: String,
    limit: Option<usize>,
) -> Result<Vec<LinkSuggestion>, String> {
    let mode = LinkSuggestionMode::parse(&mode)
        .ok_or_else(|| format!("unknown link_suggestion_mode: {mode}"))?;
    open_store()?
        .apply_link_suggestions(&memory_id, mode, limit.unwrap_or(10))
        .map_err(|e| e.to_string())
}

// ── Vector / Semantic Retrieval (M1) ──────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredMemoryRecord {
    #[serde(flatten)]
    pub record: MemoryRecord,
    pub score: f32,
}

#[tauri::command]
pub async fn hub_search_memories_semantic(
    query: String,
    limit: Option<usize>,
    scope: Option<String>,
    tier: Option<String>,
    workspace: Option<String>,
) -> Result<Vec<ScoredMemoryRecord>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = open_store()?;
        let scope = scope
            .as_deref()
            .map(MemoryScope::parse)
            .transpose()
            .map_err(|e| e.to_string())?;
        let tier = tier
            .as_deref()
            .map(MemoryTier::parse)
            .transpose()
            .map_err(|e| e.to_string())?;
        let hits = store
            .search_memories_semantic(
                &query,
                limit.unwrap_or(20),
                scope,
                tier,
                workspace.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        Ok(hits
            .into_iter()
            .map(|(record, score)| ScoredMemoryRecord { record, score })
            .collect())
    })
    .await
    .map_err(|e| format!("search task panicked: {e}"))?
}

#[tauri::command]
pub async fn hub_search_memories_hybrid(
    query: String,
    limit: Option<usize>,
    scope: Option<String>,
    tier: Option<String>,
    workspace: Option<String>,
) -> Result<Vec<ScoredMemoryRecord>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = open_store()?;
        let scope = scope
            .as_deref()
            .map(MemoryScope::parse)
            .transpose()
            .map_err(|e| e.to_string())?;
        let tier = tier
            .as_deref()
            .map(MemoryTier::parse)
            .transpose()
            .map_err(|e| e.to_string())?;
        let hits = store
            .search_memories_hybrid(
                &query,
                limit.unwrap_or(20),
                scope,
                tier,
                workspace.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        Ok(hits
            .into_iter()
            .map(|(record, score)| ScoredMemoryRecord { record, score })
            .collect())
    })
    .await
    .map_err(|e| format!("hybrid search task panicked: {e}"))?
}

#[tauri::command]
pub async fn hub_reindex_memory_vectors() -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        open_store()?
            .reindex_memory_vectors()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("reindex task panicked: {e}"))?
}
