//! C14.13 Alibaba Qwen Code bridge (#308).
//!
//! Headless runs use `qwen --session-id <uuid> --chat-recording -y
//! --output-format stream-json <prompt>` with cwd set to the workspace
//! (see `harness::qwen_spawn_args`). `--session-id` pre-assigns the
//! transcript id, so managed capture is Muse-style: register the id the
//! app passed, no discover-then-register.
//!
//! Transcripts are Claude-Code JSONL at
//! `~/.qwen/projects/<sanitised-cwd>/chats/<sessionId>.jsonl`. `QWEN_CODE_HOME`
//! relocates config but detaches auth (a run under it 401s) — this bridge
//! always reads `~/.qwen` and tests stub the projects dir directly.
//!
//! Task delivery re-enters the managed session headlessly under the writer
//! lease via `--resume <uuid> --chat-recording -y` (a second `--session-id`
//! is rejected once the transcript exists). Observed sessions stay capture-only.

use crate::{
    HarnessInjectRequest, HarnessInjectResult, HarnessSessionMode, HarnessSessionState, HubError,
    HubStore,
};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `~/.qwen/projects` — the live CLI's transcript root (qwen 0.23.2).
pub fn qwen_projects_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".qwen").join("projects")
}

/// Mirrors Qwen Code's project-directory naming: absolute workspace path
/// with every `/` replaced by `-` (same sanitiser as Claude Code).
pub fn encode_workspace_dir_name(workspace: &Path) -> String {
    workspace.to_string_lossy().replace('/', "-")
}

fn chats_dir(projects_dir: &Path, workspace: &Path) -> PathBuf {
    projects_dir
        .join(encode_workspace_dir_name(workspace))
        .join("chats")
}

/// Locate `<session-id>.jsonl` under the workspace's `chats/` directory.
/// Hub `managed-<uuid>` ids strip to the UUID the CLI wrote.
pub fn qwen_session_log_path(
    projects_dir: &Path,
    workspace: &Path,
    session_id: &str,
) -> Option<PathBuf> {
    let disk_id = crate::harness::qwen_disk_session_id(session_id)
        .unwrap_or_else(|| session_id.trim().to_string());
    if disk_id.is_empty() {
        return None;
    }
    let log = chats_dir(projects_dir, workspace).join(format!("{disk_id}.jsonl"));
    log.is_file().then_some(log)
}

/// Most recent Qwen session id for `workspace`, newest `chats/*.jsonl` first.
/// Used by "Resume in terminal" — never guessed from process output.
pub fn latest_qwen_session_id(workspace: &Path) -> Option<String> {
    latest_qwen_session_id_from(&qwen_projects_dir(), workspace)
}

pub fn latest_qwen_session_id_from(projects_dir: &Path, workspace: &Path) -> Option<String> {
    let dir = chats_dir(projects_dir, workspace);
    let entries = fs::read_dir(&dir).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|path| {
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            let id = path.file_stem()?.to_str()?.to_string();
            (!id.is_empty()).then_some((modified, id))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, id)| id)
}

/// Deliver a task into a managed Qwen session and arm its capture.
pub fn deliver_qwen_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_qwen_task_with(store, request, run_qwen_worker)
}

#[allow(dead_code)]
pub fn deliver_qwen_task_with(
    store: &HubStore,
    request: &HarnessInjectRequest,
    runner: impl FnOnce(&Path, &str, &str, Option<&str>, Option<&str>) -> Result<Option<u32>, String>,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Qwen active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let workspace_str = workspace.to_string_lossy().into_owned();

    let registration = store.get_harness_session("qwen", &workspace_str)?;
    let is_managed = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);
    if !is_managed {
        return Ok(unavailable(
            "Qwen active-session delivery requires an app-owned managed session. Register one with hub_register_managed_harness_session. Task stays queued.",
        ));
    }
    let registered_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .unwrap_or_default();
    let Some(disk_uuid) = crate::harness::qwen_disk_session_id(&registered_id) else {
        return Ok(unavailable(&format!(
            "registered Qwen session id {registered_id:?} is not a UUID and cannot be passed to `qwen --session-id`. Task stays queued."
        )));
    };

    let writer_owner = format!(
        "qwen-worker:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );
    if let Err(error) = store.acquire_harness_writer("qwen", &workspace_str, &writer_owner) {
        return Ok(queued(&format!(
            "Qwen managed worker in workspace {workspace_str} is busy; task stays queued for retry: {error}"
        )));
    }

    let run_res = runner(
        &workspace,
        &request.body,
        &disk_uuid,
        request.model.as_deref(),
        request.effort.as_deref(),
    );
    let (next_state, result) = match run_res {
        Ok(pid) => {
            if let Some(message_id) = request.message_id.as_deref() {
                let _ = store.set_message_status(message_id, crate::MessageStatus::Acked);
            }
            (
                HarnessSessionState::Ready,
                HarnessInjectResult {
                    harness: "qwen".into(),
                    pid,
                    status: "delivered".into(),
                    detail: format!(
                        "forwarded to managed Qwen session {disk_uuid}; the run's transcript is captured on the next poll"
                    ),
                },
            )
        }
        Err(error) => (
            HarnessSessionState::Queued,
            HarnessInjectResult {
                harness: "qwen".into(),
                pid: None,
                status: "errored".into(),
                detail: format!("Qwen worker failed: {error}"),
            },
        ),
    };
    let _ = store.release_harness_writer("qwen", &workspace_str, &writer_owner, next_state);
    Ok(result)
}

fn run_qwen_worker(
    workspace: &Path,
    prompt: &str,
    disk_uuid: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Option<u32>, String> {
    let args = crate::harness::qwen_resume_spawn_args(workspace, prompt, disk_uuid, model, effort)
        .map_err(|error| format!("invalid Qwen spawn args: {error}"))?;
    let mut child = Command::new("qwen")
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn qwen: {error}"))?;
    let pid = Some(child.id());
    let stderr_join = child.stderr.take().map(|pipe| {
        std::thread::spawn(move || {
            BufReader::new(pipe)
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<_>>()
                .join("\n")
        })
    });
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = line;
        }
    }
    let status = child
        .wait()
        .map_err(|error| format!("error waiting for qwen: {error}"))?;
    let stderr = stderr_join
        .and_then(|join| join.join().ok())
        .unwrap_or_default();
    if status.success() {
        Ok(pid)
    } else {
        let hint = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .chars()
            .take(240)
            .collect::<String>();
        Err(format!(
            "qwen exited with code {:?} ({hint})",
            status.code()
        ))
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "qwen".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

fn queued(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "qwen".into(),
        pid: None,
        status: "queued".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
#[path = "qwen_tests.rs"]
mod qwen_tests;
