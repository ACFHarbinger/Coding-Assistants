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
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let workspace_key = workspace.to_string_lossy().to_string();
    let existing = store
        .get_harness_session("cursor", &workspace_key)
        .map_err(|error| error.to_string())?;
    if let Some(existing) = existing.as_ref() {
        if let Some(pid) = existing.managed_pid {
            crate::bridge::relaunch::kill_pid(pid);
        }
    }

    if !existing.is_some_and(|row| row.mode == HarnessSessionMode::Managed) {
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

    let outcome = (|| {
        store
            .update_managed_harness_disk_session_id(
                "cursor",
                &workspace_key,
                CURSOR_PENDING_DISK_SESSION_ID,
            )
            .map_err(|error| error.to_string())?;
        let (_pid, output) = runner(&workspace, prompt, None, None)?;
        let chat_id = output.session_id.ok_or_else(|| {
                "Cursor managed start completed but stream-json did not include a session_id; cannot register this workspace".to_string()
            })?;
        store
            .update_managed_harness_disk_session_id("cursor", &workspace_key, &chat_id)
            .map_err(|error| error.to_string())?;
        let started = crate::HarnessStartResult {
                harness: "cursor".into(),
                pid: None,
                status: "started".into(),
                detail: format!(
                    "Cursor managed session registered (chat id: {chat_id}); awaiting its first task before capture starts."
                ),
            };
        Ok(started)
    })();

    let release = store
        .release_harness_writer(
            "cursor",
            &workspace_key,
            CURSOR_MANAGED_START_WRITER,
            crate::HarnessSessionState::Queued,
        )
        .map_err(|error| error.to_string());

    match (outcome, release) {
        (Err(error), Err(release_error)) => Err(format!(
            "{error}; additionally failed to release Cursor managed-start writer: {release_error}"
        )),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(format!(
            "Cursor managed start completed but failed to release its writer: {error}"
        )),
        (Ok(started), Ok(())) => {
            let registration = store
                .get_harness_session("cursor", &workspace_key)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "Cursor managed registration disappeared".to_string())?;
            Ok((started, registration))
        }
    }
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
    latest_cursor_session_id_from(&cursor_agent_transcripts_dir(workspace))
}

fn latest_cursor_session_id_from(root: &Path) -> Option<String> {
    let entries = fs::read_dir(root).ok()?;
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

    store.release_harness_writer("cursor", &workspace_str, &writer_owner, next_state)?;
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
#[path = "cursor_tests.rs"]
mod tests;
