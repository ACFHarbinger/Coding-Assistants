//! C12 capture-poll Tauri commands — one thin re-dispatcher per harness's
//! own on-disk-transcript capture module. Split out of `commands.rs` to
//! keep that file under this repo's 500-LoC-per-file convention.

use crate::commands::commands::store::open_store;
use std::path::PathBuf;

async fn capture_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn hub_capture_grok_session(
    workspace: String,
    grok_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::grok::GrokCaptureOutcome, String> {
    capture_blocking(move || {
        let store = open_store()?;
        crate::harness::grok::capture_grok_session(
            &store,
            &PathBuf::from(workspace),
            grok_session_id.as_deref(),
            hub_session_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn hub_capture_claude_session(
    workspace: String,
    claude_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::claude::ClaudeCaptureOutcome, String> {
    capture_blocking(move || {
        let store = open_store()?;
        crate::harness::claude::capture_claude_session(
            &store,
            &PathBuf::from(workspace),
            claude_session_id.as_deref(),
            hub_session_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn hub_capture_codex_session(
    workspace: String,
    codex_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::codex::CodexCaptureOutcome, String> {
    capture_blocking(move || {
        let store = open_store()?;
        crate::harness::codex::capture_codex_session(
            &store,
            &PathBuf::from(workspace),
            codex_session_id.as_deref(),
            hub_session_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn hub_capture_gemini_session(
    workspace: String,
    gemini_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::gemini::GeminiCaptureOutcome, String> {
    capture_blocking(move || {
        let store = open_store()?;
        crate::harness::gemini::capture_gemini_session(
            &store,
            &PathBuf::from(workspace),
            gemini_session_id.as_deref(),
            hub_session_id.as_deref(),
        )
    })
    .await
}
