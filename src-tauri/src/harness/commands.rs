//! C12 Tauri surface for harness start / inject / capture.
//! Claude and Gemini implement their adapters in sibling files; this file
//! only dispatches through the shared `hub` contract.

use crate::hub::commands::store::open_store;
use hub::{
    default_leader_socket, inject_harness_with_store, latest_grok_session_id, start_harness,
    HarnessInjectRequest, HarnessInjectResult, HarnessSessionRegistration, HarnessStartRequest,
    HarnessStartResult, MessageRecord,
};
use std::path::PathBuf;

#[tauri::command]
pub fn hub_start_harness(
    harness: String,
    workspace: String,
    session_id: Option<String>,
    prompt: String,
) -> Result<HarnessStartResult, String> {
    start_harness(&HarnessStartRequest {
        harness,
        workspace: PathBuf::from(workspace),
        session_id,
        prompt,
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_inject_harness(
    harness: String,
    workspace: String,
    session_id: Option<String>,
    message_id: Option<String>,
    body: String,
    is_task: bool,
    is_wake: bool,
) -> Result<HarnessInjectResult, String> {
    inject_harness_with_store(
        &open_store()?,
        &HarnessInjectRequest {
            harness,
            workspace: PathBuf::from(workspace),
            session_id,
            message_id,
            body,
            is_task,
            is_wake,
        },
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_register_harness_session(
    harness: String,
    workspace: String,
    disk_session_id: Option<String>,
    leader_socket: Option<String>,
) -> Result<HarnessSessionRegistration, String> {
    let workspace_path = PathBuf::from(&workspace);
    let session_id = match disk_session_id.filter(|id| !id.trim().is_empty()) {
        Some(id) => id,
        None if harness == "grok" => latest_grok_session_id(&workspace_path).ok_or_else(|| {
            "no on-disk Grok session for this workspace; pass diskSessionId".to_string()
        })?,
        None => {
            return Err("diskSessionId is required unless Grok can infer the latest session".into())
        }
    };
    let socket = leader_socket.or_else(|| {
        let path = default_leader_socket();
        path.exists().then(|| path.display().to_string())
    });
    open_store()?
        .register_harness_session(&harness, &workspace, &session_id, socket.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_harness_sessions() -> Result<Vec<HarnessSessionRegistration>, String> {
    open_store()?
        .list_harness_sessions()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_capture_grok_session(
    workspace: String,
    grok_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::grok::GrokCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness::grok::capture_grok_session(
        &store,
        &PathBuf::from(workspace),
        grok_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[tauri::command]
pub fn hub_capture_claude_session(
    workspace: String,
    claude_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::claude::ClaudeCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness::claude::capture_claude_session(
        &store,
        &PathBuf::from(workspace),
        claude_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[tauri::command]
pub fn hub_capture_codex_session(
    workspace: String,
    codex_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::codex::CodexCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness::codex::capture_codex_session(
        &store,
        &PathBuf::from(workspace),
        codex_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[tauri::command]
pub fn hub_capture_gemini_session(
    workspace: String,
    gemini_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::gemini::GeminiCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness::gemini::capture_gemini_session(
        &store,
        &PathBuf::from(workspace),
        gemini_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[tauri::command]
pub fn hub_record_harness_capture(
    harness: String,
    agent_id: String,
    session_id: Option<String>,
    body: String,
    workspace: Option<String>,
) -> Result<Option<MessageRecord>, String> {
    open_store()?
        .record_harness_capture(
            &harness,
            &agent_id,
            session_id.as_deref(),
            &body,
            workspace.as_deref(),
        )
        .map_err(|error| error.to_string())
}
