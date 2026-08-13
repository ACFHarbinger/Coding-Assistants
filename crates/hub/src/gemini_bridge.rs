//! C12-GEMINI-BRIDGE: deliver a queued Hub task into an already-running Gemini /
//! Antigravity CLI session through the active bridge path.
//!
//! Antigravity CLI (`agy`) sessions are tracked on disk and can be resumed at
//! startup. Its published CLI currently exposes no supported IPC, RPC, or
//! active-session attach transport. The bridge therefore reports the task as
//! unavailable and leaves it queued rather than inventing a socket protocol or
//! writing to the TUI's terminal.

use crate::{HarnessInjectRequest, HarnessInjectResult, HubError, HubStore};
use std::path::{Path, PathBuf};

pub fn gemini_brain_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".gemini")
        .join("antigravity-cli")
        .join("brain")
}

pub fn latest_gemini_session_id(_workspace: &Path) -> Option<String> {
    let brain_dir = gemini_brain_dir();
    let entries = std::fs::read_dir(&brain_dir).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter_map(|dir| {
            let log_file = dir
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");
            if log_file.is_file() {
                let modified = std::fs::metadata(&log_file).ok()?.modified().ok()?;
                let conv_id = dir.file_name()?.to_string_lossy().into_owned();
                Some((modified, conv_id))
            } else {
                None
            }
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, conv_id)| conv_id)
}

/// Preserve a task for a Gemini / Antigravity session until the provider
/// offers a documented active-session transport. Never writes a TTY/PTY and
/// never starts a replacement interactive CLI process.
pub fn deliver_gemini_task(
    _store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Gemini active-session delivery requires an absolute workspace".into(),
        ));
    }
    Ok(unavailable(
        "Gemini/Antigravity exposes no documented active-session IPC or RPC transport. The task remains queued; use an explicit wake or deliver it from the active conversation until provider support exists.",
    ))
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "gemini".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;
    use tempfile::tempdir;

    #[test]
    fn active_delivery_is_unavailable_without_a_documented_provider_transport() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_gemini_task(
            &store,
            &HarnessInjectRequest {
                harness: "gemini".into(),
                workspace: PathBuf::from("/tmp/ca-gemini-bridge"),
                session_id: None,
                message_id: Some("msg-gemini-1".into()),
                body: "review antigravity code".into(),
                is_task: true,
                is_wake: false,
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert!(result.detail.contains("no documented active-session"));
        assert_eq!(result.pid, None);
    }

    #[test]
    fn relative_workspace_is_rejected() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_gemini_task(
            &store,
            &HarnessInjectRequest {
                harness: "gemini".into(),
                workspace: PathBuf::from("relative-workspace"),
                session_id: None,
                message_id: None,
                body: "hello".into(),
                is_task: true,
                is_wake: false,
            },
        );
        assert!(result.is_err());
    }
}
