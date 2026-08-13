//! C12 Tauri surface for harness start / inject / capture.
//! Claude and Gemini implement their adapters in sibling files; this file
//! only dispatches through the shared `ca_hub` contract.

use crate::hub_cmds::open_store;
use ca_hub::{
    inject_harness, start_harness, HarnessInjectRequest, HarnessInjectResult, HarnessStartRequest,
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
    inject_harness(&HarnessInjectRequest {
        harness,
        workspace: PathBuf::from(workspace),
        session_id,
        message_id,
        body,
        is_task,
        is_wake,
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_capture_grok_session(
    workspace: String,
    grok_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness_grok::GrokCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness_grok::capture_grok_session(
        &store,
        &PathBuf::from(workspace),
        grok_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[tauri::command]
pub fn hub_capture_claude_session(
    workspace: String,
    session_id: Option<String>,
) -> Result<crate::harness_claude::ClaudeCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness_claude::capture_claude_session(
        &store,
        &PathBuf::from(workspace),
        session_id.as_deref(),
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
