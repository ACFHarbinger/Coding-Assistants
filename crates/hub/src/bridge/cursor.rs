//! C14.12 Cursor `agent` managed worker bridge (#275).
//!
//! Headless runs use `agent -p <prompt> [--model <id>] --output-format stream-json`
//! with cwd set to the workspace. The chat id is parsed from stream-json and
//! persisted — `agent ls` is not machine-readable. Observed external TUI
//! sessions stay capture-only and return `unavailable` without mutating state.

use crate::{
    cursor_managed_spawn_args, HarnessInjectRequest, HarnessInjectResult, HarnessSessionMode,
    HarnessSessionState, HubError, HubStore,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::harness::cursor_executable;

const CURSOR_MANAGED_START_WRITER: &str = "cursor-managed-start";
const CURSOR_PENDING_DISK_SESSION_ID: &str = "pending";

pub fn start_cursor_managed_harness(
    store: &HubStore,
    workspace: &Path,
    prompt: &str,
) -> Result<(crate::HarnessStartResult, crate::HarnessSessionRegistration), String> {
    start_cursor_managed_harness_with(store, workspace, prompt, run_cursor_worker)
}

pub fn start_cursor_managed_harness_with(
    store: &HubStore,
    workspace: &Path,
    prompt: &str,
    runner: impl FnOnce(
        &Path,
        &str,
        Option<&str>,
        Option<&str>,
    ) -> Result<(Option<u32>, CursorStreamOutput), String>,
) -> Result<(crate::HarnessStartResult, crate::HarnessSessionRegistration), String> {
    if !workspace.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    let workspace_key = workspace.to_string_lossy().to_string();
    if let Ok(Some(existing)) = store.get_harness_session("cursor", &workspace_key) {
        if let Some(pid) = existing.managed_pid {
            crate::bridge::relaunch::kill_pid(pid);
        }
    }

    if store
        .get_harness_session("cursor", &workspace_key)
        .map_err(|error| error.to_string())?
        .is_none()
    {
        store
            .register_managed_harness_session_with_state(
                "cursor",
                &workspace_key,
                CURSOR_PENDING_DISK_SESSION_ID,
                None,
                crate::HarnessSessionState::Queued,
            )
            .map_err(|error| error.to_string())?;
    }

    store
        .acquire_harness_writer("cursor", &workspace_key, CURSOR_MANAGED_START_WRITER)
        .map_err(|error| format!("Cursor managed start is busy: {error}"))?;

    let run_result = runner(workspace, prompt, None, None);
    let (started, registration) = match run_result {
        Ok((_pid, output)) => {
            let chat_id = output.session_id.ok_or_else(|| {
                "Cursor managed start completed but stream-json did not include a session_id; cannot register this workspace".to_string()
            })?;
            store
                .update_managed_harness_disk_session_id("cursor", &workspace_key, &chat_id)
                .map_err(|error| error.to_string())?;
            let registration = store
                .get_harness_session("cursor", &workspace_key)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "Cursor managed registration disappeared".to_string())?;
            let started = crate::HarnessStartResult {
                harness: "cursor".into(),
                pid: None,
                status: "started".into(),
                detail: format!(
                    "Cursor managed session registered (chat id: {chat_id}); awaiting its first task before capture starts."
                ),
            };
            (started, registration)
        }
        Err(error) => {
            let _ = store.release_harness_writer(
                "cursor",
                &workspace_key,
                CURSOR_MANAGED_START_WRITER,
                crate::HarnessSessionState::Queued,
            );
            return Err(error);
        }
    };

    store
        .release_harness_writer(
            "cursor",
            &workspace_key,
            CURSOR_MANAGED_START_WRITER,
            crate::HarnessSessionState::Queued,
        )
        .map_err(|error| error.to_string())?;

    Ok((started, registration))
}

pub fn cursor_projects_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cursor").join("projects")
}

/// Mirrors Cursor's project slug: absolute workspace with leading `/` stripped
/// and remaining `/` → `-` (confirmed against `~/.cursor/projects/` on disk).
pub fn encode_workspace_dir_name(workspace: &Path) -> String {
    let raw = workspace.to_string_lossy();
    let stripped = raw.strip_prefix('/').unwrap_or(&raw);
    stripped.replace('/', "-")
}

pub fn cursor_agent_transcripts_dir(workspace: &Path) -> PathBuf {
    cursor_projects_dir()
        .join(encode_workspace_dir_name(workspace))
        .join("agent-transcripts")
}

/// Most recent Cursor chat id under `~/.cursor/projects/.../agent-transcripts/`.
pub fn latest_cursor_session_id(workspace: &Path) -> Option<String> {
    let root = cursor_agent_transcripts_dir(workspace);
    let entries = fs::read_dir(&root).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter_map(|dir| {
            if !dir.is_dir() {
                return None;
            }
            let session_id = dir.file_name()?.to_string_lossy().into_owned();
            let transcript = dir.join(format!("{session_id}.jsonl"));
            if !transcript.is_file() {
                return None;
            }
            let modified = fs::metadata(&transcript).ok()?.modified().ok()?;
            Some((modified, session_id))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, session_id)| session_id)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorStreamOutput {
    pub session_id: Option<String>,
    pub assistant_texts: Vec<String>,
}

pub fn parse_cursor_stream_line(line: &str) -> Option<CursorStreamOutput> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(line).ok()?;
    let mut out = CursorStreamOutput::default();

    if let Some(session_id) = value.get("session_id").and_then(Value::as_str) {
        out.session_id = Some(session_id.to_string());
    }

    if value.get("type").and_then(Value::as_str) == Some("assistant") {
        if let Some(text) = assistant_text_from_stream_value(&value) {
            out.assistant_texts.push(text);
        }
    }

    if out.session_id.is_some() || !out.assistant_texts.is_empty() {
        Some(out)
    } else {
        None
    }
}

fn assistant_text_from_stream_value(value: &Value) -> Option<String> {
    let content = value.get("message")?.get("content")?.as_array()?;
    let mut parts = Vec::new();
    for item in content {
        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Real Cursor chat ids come from stream-json at managed start or task delivery.
/// Hub `managed-*` / `pending` placeholders must never reach `--resume`.
/// `request.session_id` is Hub work-session routing metadata, not a Cursor chat id.
pub(crate) fn persisted_cursor_chat_id(
    registration: Option<&crate::HarnessSessionRegistration>,
) -> Option<String> {
    registration.and_then(|row| {
        let id = row.disk_session_id.trim();
        if id.is_empty() || id.starts_with("managed-") || id == "pending" {
            None
        } else {
            Some(id.to_string())
        }
    })
}

pub fn deliver_cursor_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_cursor_task_with(store, request, run_cursor_worker)
}

pub fn deliver_cursor_task_with(
    store: &HubStore,
    request: &HarnessInjectRequest,
    runner: impl FnOnce(
        &Path,
        &str,
        Option<&str>,
        Option<&str>,
    ) -> Result<(Option<u32>, CursorStreamOutput), String>,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Cursor active-session delivery requires an absolute workspace".into(),
        ));
    }

    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let workspace_str = workspace.to_string_lossy().into_owned();

    let registration = store.get_harness_session("cursor", &workspace_str)?;
    let is_managed = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);

    if !is_managed {
        return Ok(unavailable(
            "Cursor active session delivery requires an app-owned managed session. Register one with hub_register_managed_harness_session. Task stays queued.",
        ));
    }

    let chat_id = persisted_cursor_chat_id(registration.as_ref());

    if is_managed && chat_id.is_none() {
        return Ok(unavailable(
            "Cursor managed session has no persisted chat id yet; use Start managed so stream-json can register one.",
        ));
    }

    let writer_owner = format!(
        "cursor-worker:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );

    if let Err(error) = store.acquire_harness_writer("cursor", &workspace_str, &writer_owner) {
        return Ok(queued(&format!(
            "Cursor managed worker in workspace {workspace_str} is busy; task stays queued for retry: {error}"
        )));
    }

    let run_res = runner(
        &workspace,
        &request.body,
        chat_id.as_deref(),
        request.model.as_deref(),
    );

    let (next_state, result) = match run_res {
        Ok((_pid, output)) => {
            if let Some(ref new_chat) = output.session_id {
                let _ = store.update_managed_harness_disk_session_id(
                    "cursor",
                    &workspace_str,
                    new_chat,
                );
            }
            let detail = if let Some(ref chat) = output.session_id {
                format!("Cursor worker completed successfully (session: {chat})")
            } else {
                "Cursor worker completed successfully".into()
            };
            (
                HarnessSessionState::Ready,
                HarnessInjectResult {
                    harness: "cursor".into(),
                    pid: None,
                    status: "ok".into(),
                    detail,
                },
            )
        }
        Err(err) => (
            HarnessSessionState::Queued,
            HarnessInjectResult {
                harness: "cursor".into(),
                pid: None,
                status: "errored".into(),
                detail: format!("Cursor worker failed: {err}"),
            },
        ),
    };

    let _ = store.release_harness_writer("cursor", &workspace_str, &writer_owner, next_state);
    Ok(result)
}

pub(crate) fn run_cursor_worker(
    workspace: &Path,
    prompt: &str,
    chat_id: Option<&str>,
    model: Option<&str>,
) -> Result<(Option<u32>, CursorStreamOutput), String> {
    let args = cursor_managed_spawn_args(workspace, prompt, chat_id, model, None)
        .map_err(|error| format!("invalid spawn args: {error}"))?;

    let mut child = Command::new(cursor_executable())
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn {}: {error}", cursor_executable()))?;

    let pid = Some(child.id());
    let mut stream_out = CursorStreamOutput::default();

    let stderr_drain = child.stderr.take().map(|stderr| {
        std::thread::spawn(move || {
            let _ = std::io::read_to_string(stderr);
        })
    });

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(evt) = parse_cursor_stream_line(&line) {
                if evt.session_id.is_some() {
                    stream_out.session_id = evt.session_id;
                }
                stream_out.assistant_texts.extend(evt.assistant_texts);
            }
        }
    }

    if let Some(handle) = stderr_drain {
        let _ = handle.join();
    }

    let status = child
        .wait()
        .map_err(|error| format!("error waiting for {}: {error}", cursor_executable()))?;

    if status.success() {
        Ok((pid, stream_out))
    } else {
        Err(format!(
            "{} exited with code {:?}",
            cursor_executable(),
            status.code()
        ))
    }
}

pub(crate) fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "cursor".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

pub(crate) fn queued(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "cursor".into(),
        pid: None,
        status: "queued".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;
    use tempfile::tempdir;

    #[test]
    fn parse_cursor_stream_json_line() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Done"}]},"session_id":"chat-123"}"#;
        let parsed = parse_cursor_stream_line(line).unwrap();
        assert_eq!(parsed.session_id.as_deref(), Some("chat-123"));
        assert_eq!(parsed.assistant_texts, vec!["Done".to_string()]);
    }

    #[test]
    fn unmanaged_cursor_delivery_returns_unavailable() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_cursor_task(
            &store,
            &HarnessInjectRequest {
                harness: "cursor".into(),
                workspace: dir.path().to_path_buf(),
                session_id: None,
                message_id: Some("msg-1".into()),
                body: "hello cursor".into(),
                is_task: true,
                is_wake: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert!(result.detail.contains("managed session"));
    }

    #[test]
    fn managed_cursor_delivery_acquires_and_releases_writer_lease() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path();
        let ws_str = workspace.to_string_lossy().into_owned();

        store
            .register_managed_harness_session("cursor", &ws_str, "chat-owned-1", 1234)
            .unwrap();

        let req = HarnessInjectRequest {
            harness: "cursor".into(),
            workspace: workspace.to_path_buf(),
            session_id: None,
            message_id: Some("msg-test-1".into()),
            body: "build worker component".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        };

        let result = deliver_cursor_task_with(&store, &req, |_ws, _prompt, chat_id, _model| {
            assert_eq!(chat_id, Some("chat-owned-1"));
            Ok((
                Some(1234),
                CursorStreamOutput {
                    session_id: Some("chat-owned-1".into()),
                    assistant_texts: vec!["Done".into()],
                },
            ))
        })
        .unwrap();

        assert_eq!(result.status, "ok");
        assert_eq!(result.pid, None);

        let sess = store
            .get_harness_session("cursor", &ws_str)
            .unwrap()
            .unwrap();
        assert_eq!(sess.state, HarnessSessionState::Ready);
        assert!(sess.writer_owner.is_none());
    }

    #[test]
    fn managed_cursor_delivery_without_persisted_chat_id_is_unavailable() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path();
        let ws_str = workspace.to_string_lossy().into_owned();
        store
            .register_managed_harness_session("cursor", &ws_str, "managed-fresh", 1234)
            .unwrap();

        let result = deliver_cursor_task(
            &store,
            &HarnessInjectRequest {
                harness: "cursor".into(),
                workspace: workspace.to_path_buf(),
                session_id: None,
                message_id: Some("msg-1".into()),
                body: "hello".into(),
                is_task: true,
                is_wake: false,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(result.status, "unavailable");
        assert!(result.detail.contains("persisted chat id"));
    }

    #[test]
    fn persisted_chat_id_rejects_managed_placeholder() {
        let registration = crate::HarnessSessionRegistration {
            harness: "cursor".into(),
            workspace: "/tmp/ws".into(),
            disk_session_id: "managed-abc".into(),
            leader_socket: None,
            registered_at: "0".into(),
            mode: crate::HarnessSessionMode::Managed,
            state: crate::HarnessSessionState::Queued,
            managed_pid: None,
            writer_owner: None,
            writer_acquired_at: None,
        };
        assert!(persisted_cursor_chat_id(Some(&registration)).is_none());
    }

    #[test]
    fn delivery_ignores_hub_session_routing_metadata_for_resume() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path();
        let ws_str = workspace.to_string_lossy().into_owned();
        store
            .register_managed_harness_session("cursor", &ws_str, "cursor-chat-real", 1234)
            .unwrap();

        let req = HarnessInjectRequest {
            harness: "cursor".into(),
            workspace: workspace.to_path_buf(),
            session_id: Some("hub-work-session-routing-id".into()),
            message_id: Some("msg-test-2".into()),
            body: "continue task".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        };

        let result = deliver_cursor_task_with(&store, &req, |_ws, _prompt, chat_id, _model| {
            assert_eq!(chat_id, Some("cursor-chat-real"));
            Ok((
                None,
                CursorStreamOutput {
                    session_id: Some("cursor-chat-real".into()),
                    assistant_texts: vec![],
                },
            ))
        })
        .unwrap();
        assert_eq!(result.status, "ok");
    }

    #[test]
    fn managed_start_persists_stream_session_id_as_disk_session_id() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().canonicalize().unwrap();
        let ws_str = workspace.to_string_lossy().into_owned();

        let (started, registration) = start_cursor_managed_harness_with(
            &store,
            &workspace,
            "Coding-Assistants managed session",
            |_ws, _prompt, _chat_id, _model| {
                Ok((
                    None,
                    CursorStreamOutput {
                        session_id: Some("stream-chat-42".into()),
                        assistant_texts: vec![],
                    },
                ))
            },
        )
        .unwrap();

        assert_eq!(started.status, "started");
        assert_eq!(registration.disk_session_id, "stream-chat-42");
        let row = store
            .get_harness_session("cursor", &ws_str)
            .unwrap()
            .unwrap();
        assert_eq!(row.disk_session_id, "stream-chat-42");
        assert!(row.writer_owner.is_none());
        assert_eq!(row.managed_pid, None);
    }

    #[test]
    fn latest_cursor_session_id_finds_the_newest_transcript_dir() {
        static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _lock = HOME_LOCK.lock().unwrap();
        let saved_home = std::env::var("HOME").ok();
        let dir = tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let workspace = PathBuf::from("/tmp/c14-cursor-latest");
        let transcripts = cursor_agent_transcripts_dir(&workspace);
        for (session, marker) in [("older-chat", "a"), ("newer-chat", "b")] {
            let session_dir = transcripts.join(session);
            fs::create_dir_all(&session_dir).unwrap();
            fs::write(session_dir.join(format!("{session}.jsonl")), marker).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            latest_cursor_session_id(&workspace).as_deref(),
            Some("newer-chat")
        );
        match saved_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
    }
}
