//! C12-GEMINI-BRIDGE: deliver a queued Hub task into an already-running Gemini /
//! Antigravity CLI session through the active bridge path.
//!
//! Antigravity CLI (`agy`) sessions are tracked in
//! `~/.gemini/antigravity-cli/brain/<conversation-id>/`. If a registered
//! session socket or active session bridge exists (default socket `~/.gemini/antigravity-cli/bridge.sock`),
//! tasks are delivered to the active conversation. If the bridge socket is absent,
//! delivery is `unavailable` and the task stays queued.

use crate::{HarnessInjectRequest, HarnessInjectResult, HubError, HubStore};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn default_gemini_bridge_socket() -> PathBuf {
    if let Ok(path) = std::env::var("GEMINI_BRIDGE_SOCKET") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".gemini")
        .join("antigravity-cli")
        .join("bridge.sock")
}

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

pub fn gemini_bridge_socket_available(path: &Path) -> bool {
    path.exists()
}

/// Deliver a task into a registered Gemini / Antigravity session. Never writes a TTY/PTY and
/// never starts a replacement interactive CLI process.
pub fn deliver_gemini_task(
    store: &HubStore,
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
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let registration = store.get_harness_session("gemini", &workspace.to_string_lossy())?;
    let session_id = request
        .session_id
        .clone()
        .or_else(|| registration.as_ref().map(|row| row.disk_session_id.clone()))
        .or_else(|| latest_gemini_session_id(&workspace));

    let Some(session_id) = session_id else {
        return Ok(unavailable(
            "no registered or on-disk Gemini/Antigravity session for this workspace; register one with hub_register_harness_session",
        ));
    };

    let socket = registration
        .as_ref()
        .and_then(|row| row.leader_socket.as_deref())
        .map(PathBuf::from)
        .unwrap_or_else(default_gemini_bridge_socket);

    if !gemini_bridge_socket_available(&socket) {
        return Ok(unavailable(&format!(
            "Gemini/Antigravity bridge socket is not available at {} — start the active session bridge. Task stays queued.",
            socket.display()
        )));
    }

    match run_gemini_prompt(&socket, &workspace, &session_id, &request.body) {
        Ok(reply) => {
            if let Some(message_id) = request.message_id.as_deref() {
                let _ = store.set_message_status(message_id, crate::MessageStatus::Acked);
            }
            let _ = store.record_harness_capture(
                "gemini",
                "gemini",
                request.session_id.as_deref(),
                &reply,
                Some(&workspace.to_string_lossy()),
            );
            Ok(HarnessInjectResult {
                harness: "gemini".into(),
                pid: None,
                status: "delivered".into(),
                detail: format!(
                    "forwarded to registered Gemini/Antigravity session {session_id} via bridge {}",
                    socket.display()
                ),
            })
        }
        Err(error) => Ok(unavailable(&error)),
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "gemini".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

fn run_gemini_prompt(
    socket: &Path,
    workspace: &Path,
    session_id: &str,
    text: &str,
) -> Result<String, String> {
    let mut child = Command::new("agy")
        .args([
            "agent",
            "--bridge-socket",
            &socket.to_string_lossy(),
            "--session",
            session_id,
        ])
        .current_dir(workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not start Gemini bridge client: {error}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Gemini bridge stdin unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Gemini bridge stdout unavailable".to_string())?;

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "session/prompt",
        "params": {
            "conversation_id": session_id,
            "prompt": text
        }
    });

    writeln!(stdin, "{payload}").map_err(|e| e.to_string())?;
    stdin.flush().map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).map_err(|e| e.to_string())?;
    let _ = child.kill();
    let _ = child.wait();

    if bytes == 0 {
        return Err("Gemini bridge returned empty response".into());
    }

    let Ok(val) = serde_json::from_str::<Value>(line.trim()) else {
        return Err("invalid JSON response from Gemini bridge".into());
    };

    let reply = val
        .pointer("/result/content")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    if reply.trim().is_empty() {
        return Err("Gemini prompt returned no content".into());
    }

    Ok(reply.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;
    use tempfile::tempdir;

    #[test]
    fn missing_bridge_is_unavailable_and_does_not_spawn_replacement() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session(
                "gemini",
                "/tmp/ca-gemini-bridge",
                "conv-uuid-12345",
                Some("/tmp/missing-ca-gemini-bridge.sock"),
            )
            .unwrap();
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
        assert!(result.detail.contains("bridge socket"));
        assert_eq!(result.pid, None);
    }

    #[test]
    fn registration_is_required_or_inferred() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_gemini_task(
            &store,
            &HarnessInjectRequest {
                harness: "gemini".into(),
                workspace: PathBuf::from("/tmp/nonexistent-gemini-workspace-xyz"),
                session_id: None,
                message_id: None,
                body: "hello".into(),
                is_task: true,
                is_wake: false,
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert!(result.detail.contains("no registered") || result.detail.contains("bridge socket"));
    }
}
