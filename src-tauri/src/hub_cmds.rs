//! Tauri commands that expose `ca_hub::HubStore` to the desktop UI.
//! Same data directory as the `ca` CLI (`$CA_HOME` or `~/.coding-assistants`).

use ca_hub::{
    CompactReport, GitExportOutcome, HubStore, MemoryRecord, MemoryScope, MemoryTier, MessageKind,
    MessageRecord, MessageStatus, WakePolicy, WakeRecord, WakeStatus,
};
use std::path::PathBuf;

fn default_home() -> PathBuf {
    if let Ok(home) = std::env::var("CA_HOME") {
        return PathBuf::from(home);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".coding-assistants")
}

fn open_store() -> Result<HubStore, String> {
    HubStore::open(default_home()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_init() -> Result<String, String> {
    let store = open_store()?;
    Ok(store.data_dir().display().to_string())
}

#[tauri::command]
pub fn hub_list_agents() -> Result<Vec<ca_hub::AgentRecord>, String> {
    open_store()?.list_agents().map_err(|e| e.to_string())
}

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

#[derive(serde::Deserialize)]
pub struct SendMessageArgs {
    pub from: String,
    pub to: String,
    pub kind: Option<String>,
    pub subject: Option<String>,
    pub workspace: Option<String>,
    pub task: Option<String>,
    pub body: String,
}

#[tauri::command]
pub fn hub_send_message(args: SendMessageArgs) -> Result<MessageRecord, String> {
    let store = open_store()?;
    let kind =
        MessageKind::parse(args.kind.as_deref().unwrap_or("message")).map_err(|e| e.to_string())?;
    store
        .send_message(
            &args.from,
            &args.to,
            kind,
            &args.body,
            args.subject.as_deref(),
            args.workspace.as_deref(),
            args.task.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_poll_messages(
    to: String,
    mark_acked: Option<bool>,
) -> Result<Vec<MessageRecord>, String> {
    open_store()?
        .poll_messages(&to, mark_acked.unwrap_or(true))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_messages(
    to: Option<String>,
    status: Option<String>,
) -> Result<Vec<MessageRecord>, String> {
    let store = open_store()?;
    let status = status
        .as_deref()
        .map(MessageStatus::parse)
        .transpose()
        .map_err(|e| e.to_string())?;
    store
        .list_messages(to.as_deref(), status)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_request_wake(
    target: String,
    reason: Option<String>,
    message_id: Option<String>,
    human_gate: Option<bool>,
) -> Result<WakeRecord, String> {
    open_store()?
        .request_wake(
            &target,
            reason.as_deref(),
            message_id.as_deref(),
            human_gate.unwrap_or(false),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_wakes(
    target: Option<String>,
    pending_only: Option<bool>,
) -> Result<Vec<WakeRecord>, String> {
    open_store()?
        .list_wakes(target.as_deref(), pending_only.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_export_markdown() -> Result<String, String> {
    let path = open_store()?
        .export_markdown(None)
        .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

/// Export + `git add`/`git commit` if the markdown dir is inside a work tree
/// (M3). Never fails solely because there's no repo there — see `detail`.
#[tauri::command]
pub fn hub_export_markdown_git(message: Option<String>) -> Result<GitExportOutcome, String> {
    open_store()?
        .export_markdown_git(None, message.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_append_journal(agent: String, entry: String) -> Result<String, String> {
    let path = open_store()?
        .append_private_journal(&agent, &entry)
        .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn hub_data_dir() -> Result<String, String> {
    Ok(open_store()?.data_dir().display().to_string())
}

#[tauri::command]
pub fn hub_purge_stale_memories() -> Result<usize, String> {
    open_store()?
        .purge_stale_memories()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_age_out_short_term(hours: Option<i64>) -> Result<usize, String> {
    open_store()?
        .mark_short_term_stale_older_than(hours.unwrap_or(72))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_set_message_status(id: String, status: String) -> Result<MessageRecord, String> {
    let st = MessageStatus::parse(&status).map_err(|e| e.to_string())?;
    open_store()?
        .set_message_status(&id, st)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_resolve_wake(id: String, status: String) -> Result<(), String> {
    let st = match status.as_str() {
        "delivered" => WakeStatus::Delivered,
        "cancelled" => WakeStatus::Cancelled,
        "pending" => WakeStatus::Pending,
        other => return Err(format!("unknown wake status: {other}")),
    };
    open_store()?
        .set_wake_status(&id, st)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_get_wake_policy() -> Result<WakePolicy, String> {
    open_store()?.get_wake_policy().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_set_wake_policy(policy: WakePolicy) -> Result<WakePolicy, String> {
    let store = open_store()?;
    store.set_wake_policy(&policy).map_err(|e| e.to_string())?;
    Ok(policy)
}
