//! Chat, work-session, wake, journal, and audit commands.
use super::store::open_store;
use hub::{
    AuditEvent, ChannelRecord, GitExportOutcome, HubStore, MemoryRecord, MessageKind,
    MessageRecord, MessageStatus, ReadMarker, SettingsStore, WakePolicy, WakeRecord, WakeStatus,
};

/// S5 / #131: exports are gated by Settings' global `export_enabled` policy.
/// There is no per-workspace export scope today, so this resolves the
/// global default only.
fn export_enabled() -> bool {
    SettingsStore::open(hub::default_hub_home())
        .effective(None)
        .orchestration
        .export_enabled
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
    if kind.requires_tagged_send() {
        return Err(
            "wake messages must use hub_send_tagged_message so enrollment and policy are recorded"
                .into(),
        );
    }
    if args.to == "team" {
        return store
            .send_message_to_team(
                &args.from,
                kind,
                &args.body,
                args.subject.as_deref(),
                args.workspace.as_deref(),
                args.task.as_deref(),
            )
            .map_err(|error| error.to_string())?
            .into_iter()
            .next()
            .ok_or_else(|| "team message produced no recipient records".to_string());
    }
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTaggedMessageArgs {
    pub from: String,
    pub to: Vec<String>,
    pub is_task: bool,
    pub is_wake: bool,
    pub subject: Option<String>,
    pub workspace: Option<String>,
    pub task: Option<String>,
    pub session_id: Option<String>,
    pub body: String,
}

/// C11: same task/wake enforcement for the human UI and agents alike — this
/// command is the one typed boundary both call, so neither can bypass the
/// other's rules. Routes through the role-permission gate
/// (`send_tagged_message_gated`): a sender over its role's daily ungated
/// quota or broadcast-recipient limit gets a durable pending approval
/// instead of immediate delivery, rather than the raw, ungated
/// `send_tagged_message`.
///
/// Async + spawn_blocking: wake enrollment and SQLite writes can stall the
/// webview when run as a sync command (#163). Chat's Send button already
/// shows "Sending…"; this keeps window drag/tab switches responsive too.
#[tauri::command]
pub async fn hub_send_tagged_message(
    args: SendTaggedMessageArgs,
) -> Result<Vec<hub::SendOutcome>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        open_store()?
            .send_tagged_message_gated(
                &args.from,
                &args.to,
                args.is_task,
                args.is_wake,
                &args.body,
                args.subject.as_deref(),
                args.workspace.as_deref(),
                args.task.as_deref(),
                args.session_id.as_deref(),
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|error| format!("hub_send_tagged_message task panicked: {error}"))?
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSessionMessageArgs {
    pub from: String,
    pub session_id: String,
    pub to: Vec<String>,
    pub subject: Option<String>,
    pub workspace: Option<String>,
    pub task: Option<String>,
    pub body: String,
}

#[tauri::command]
pub fn hub_send_session_message(
    args: SendSessionMessageArgs,
) -> Result<Vec<MessageRecord>, String> {
    open_store()?
        .send_session_message(
            &args.from,
            &args.session_id,
            &args.to,
            &args.body,
            args.subject.as_deref(),
            args.workspace.as_deref(),
            args.task.as_deref(),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_tagged_send_outcomes(subject: String) -> Result<Vec<hub::SendOutcome>, String> {
    open_store()?
        .list_tagged_send_outcomes(&subject)
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

/// Records that `agent` has read `scope` (a channel id, work session id, or
/// `dm-<agent>` pairing) as of now. Never regresses an existing, more
/// recent marker — see `hub::HubStore::mark_read`.
#[tauri::command]
pub fn hub_mark_read(agent: String, scope: String) -> Result<ReadMarker, String> {
    open_store()?
        .mark_read(&agent, &scope, None)
        .map_err(|e| e.to_string())
}

/// Every team member's read marker for `scope`, for the chat UI to render
/// "read by" against each message.
#[tauri::command]
pub fn hub_list_read_markers(scope: String) -> Result<Vec<ReadMarker>, String> {
    open_store()?
        .list_read_markers(&scope)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_channels() -> Result<Vec<ChannelRecord>, String> {
    open_store()?
        .list_channels()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_create_channel(name: String, topic: Option<String>) -> Result<ChannelRecord, String> {
    open_store()?
        .create_channel(&name, topic.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_delete_channel(id: String) -> Result<(), String> {
    open_store()?
        .delete_channel(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_channel_messages(
    channel: String,
    limit: Option<usize>,
) -> Result<Vec<MessageRecord>, String> {
    open_store()?
        .list_channel_messages(&channel, limit.unwrap_or(100))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_message_memories(message_id: String) -> Result<Vec<MemoryRecord>, String> {
    open_store()?
        .list_message_memories(&message_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_team_members() -> Result<Vec<hub::AgentRecord>, String> {
    open_store()?.list_team_members().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_set_team_member(id: String, enrolled: bool) -> Result<hub::AgentRecord, String> {
    open_store()?
        .set_team_member(&id, enrolled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_create_work_session(name: String) -> Result<hub::WorkSessionRecord, String> {
    open_store()?
        .create_work_session(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_work_sessions() -> Result<Vec<hub::WorkSessionRecord>, String> {
    open_store()?
        .list_work_sessions()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_add_work_session_member(
    session_id: String,
    agent_id: String,
) -> Result<hub::WorkSessionRecord, String> {
    open_store()?
        .add_work_session_member(&session_id, &agent_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_request_team_wakes(
    from: String,
    reason: Option<String>,
    message_id: Option<String>,
    human_gate: Option<bool>,
) -> Result<Vec<WakeRecord>, String> {
    open_store()?
        .request_team_wakes(
            &from,
            reason.as_deref(),
            message_id.as_deref(),
            human_gate.unwrap_or(false),
        )
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
    if !export_enabled() {
        return Err("export is disabled by orchestration policy".to_string());
    }
    let path = open_store()?
        .export_markdown(None)
        .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

/// Export + `git add`/`git commit` if the markdown dir is inside a work tree
/// (M3). Never fails solely because there's no repo there — see `detail`.
#[tauri::command]
pub fn hub_export_markdown_git(message: Option<String>) -> Result<GitExportOutcome, String> {
    if !export_enabled() {
        return Err("export is disabled by orchestration policy".to_string());
    }
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

/// CA-106: only Harbinger may edit/delete a Messager chat post in v1 — an agent
/// must not be able to silently rewrite another agent's line. Team/channel
/// broadcasts are N SQLite rows (one per recipient) sharing a subject, so
/// both commands update/cancel every sibling copy via `hub`'s broadcast
/// grouping, not just the row the caller happened to have in view.
fn require_human_authored(store: &HubStore, message_id: &str) -> Result<(), String> {
    let message = store
        .get_message(message_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("message not found: {message_id}"))?;
    if message.from_agent != "human" {
        return Err("only Harbinger may edit or delete a chat message".into());
    }
    Ok(())
}

#[tauri::command]
pub fn hub_update_message(id: String, body: String) -> Result<Vec<MessageRecord>, String> {
    let store = open_store()?;
    require_human_authored(&store, &id)?;
    store
        .update_broadcast(&id, &body)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_delete_message(id: String) -> Result<usize, String> {
    let store = open_store()?;
    require_human_authored(&store, &id)?;
    store.delete_broadcast(&id).map_err(|e| e.to_string())
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

/// CA-111: pending audit events surfaced when the desktop Journal/Audit tab
/// opens (`hub::HubStore::list_audit_events`, already implemented — this
/// just exposes it, plus approve/quarantine, to the Tauri IPC boundary).
#[tauri::command]
pub fn hub_list_audit_events(pending_only: Option<bool>) -> Result<Vec<AuditEvent>, String> {
    open_store()?
        .list_audit_events(pending_only.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_approve_audit(id: String) -> Result<(), String> {
    open_store()?
        .set_audit_status(&id, "approved")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_quarantine_audit(id: String) -> Result<(), String> {
    open_store()?
        .set_audit_status(&id, "quarantined")
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
