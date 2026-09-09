//! S6 Mistral Vibe bridge — session discovery + managed delivery.
//!
//! Vibe stores sessions in a flat layout under `$VIBE_HOME/logs/session`
//! (default `~/.vibe/logs/session`):
//!
//! ```text
//! session_<YYYYMMDD>_<HHMMSS>_<short-id>/
//!   ├── messages.jsonl
//!   └── meta.json
//! ```
//!
//! `meta.json` carries `session_id` (the UUID accepted by `vibe --resume`),
//! `start_time`, and `environment.working_directory`. This bridge discovers
//! existing sessions by workspace match and returns the `meta.json.session_id`
//! UUID — the only valid `--resume` token.
//!
//! Task delivery re-enters the managed session headlessly: `vibe --resume
//! <uuid> -p <task> --workdir <workspace> --trust --output streaming
//! --auto-approve` appends a run to the same transcript the capture poller
//! reads, so delivery both performs the task and arms capture via the
//! writer-lease release. Observed external sessions stay capture-only.

use crate::{
    HarnessInjectRequest, HarnessInjectResult, HarnessSessionMode, HarnessSessionState, HubError,
    HubStore,
};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Vibe sessions root honoring `${VIBE_HOME:-$HOME/.vibe}/logs/session`.
pub fn vibe_logs_root() -> PathBuf {
    let vibe_home = std::env::var("VIBE_HOME")
        .ok()
        .and_then(|dir| {
            let dir = dir.trim();
            if dir.is_empty() {
                None
            } else {
                Some(PathBuf::from(dir))
            }
        })
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".vibe")
        });
    vibe_home.join("logs").join("session")
}

/// Bound on directory entries visited while locating one session log.
const MAX_VISITED_DIRS: usize = 2000;

/// Directory names that are never session logs: caches and internal state.
fn is_session_tree_excluded(name: &str) -> bool {
    name.starts_with('.') || name == "tool-outputs" || name == "subagent"
}

/// Locate `<session-dir>/messages.jsonl` under the vibe sessions root,
/// expecting the flat layout: `session_<YYYYMMDD>_<HHMMSS>_<short-id>/messages.jsonl`.
pub fn vibe_session_log_path(sessions_root: &Path, session_id: &str) -> Option<PathBuf> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return None;
    }
    let session_dir = sessions_root.join(session_id);
    let log = session_dir.join("messages.jsonl");
    if log.is_file() {
        return Some(log);
    }
    None
}

/// Read `environment.working_directory` from a session directory's
/// `meta.json`. Only the meta file is read — identity is written at session
/// open, so a bounded read suffices.
pub fn vibe_session_workspace(session_dir: &Path) -> Option<String> {
    let meta = read_meta_json(session_dir)?;
    meta.pointer("/environment/working_directory")
        .and_then(|item| item.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read `session_id` from a session directory's `meta.json`. This UUID is
/// the only valid `--resume` token — the session directory name is not.
pub fn vibe_session_id(session_dir: &Path) -> Option<String> {
    let meta = read_meta_json(session_dir)?;
    meta.get("session_id")
        .and_then(|item| item.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_meta_json(session_dir: &Path) -> Option<serde_json::Value> {
    let meta_path = session_dir.join("meta.json");
    let raw = fs::read(&meta_path).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// Copied verbatim from muse.rs: check if recorded workspace matches the
/// given workspace path.
pub fn workspace_matches(recorded: &str, workspace: &Path) -> bool {
    if recorded == workspace.to_string_lossy() {
        return true;
    }
    let wanted = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    if recorded == wanted.to_string_lossy() {
        return true;
    }
    Path::new(recorded).canonicalize().ok().as_ref() == Some(&wanted)
}

/// Most recent Vibe session id for `workspace`, newest messages.jsonl first.
/// Returns the `meta.json.session_id` UUID — the valid `--resume` token.
pub fn latest_vibe_session_id(workspace: &Path) -> Option<String> {
    latest_vibe_session_id_from(&vibe_logs_root(), workspace, 200)
}

/// Most recent Vibe session id for `workspace` under `sessions_root`,
/// newest-first. Returns the `meta.json.session_id` UUID, not the
/// directory name. Bounded to `candidate_cap` sessions.
pub fn latest_vibe_session_id_from(
    sessions_root: &Path,
    workspace: &Path,
    candidate_cap: usize,
) -> Option<String> {
    let mut sessions: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    let mut stack = vec![sessions_root.to_path_buf()];
    let mut visited = 0;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > MAX_VISITED_DIRS {
            break;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut subdirs: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if is_session_tree_excluded(name) {
                continue;
            }
            let messages_path = path.join("messages.jsonl");
            if messages_path.is_file() {
                let modified = fs::metadata(&messages_path)
                    .and_then(|meta| meta.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                sessions.push((modified, path));
            } else {
                let modified = fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                subdirs.push((modified, path));
            }
        }
        subdirs.sort_by_key(|(modified, _)| *modified);
        stack.extend(subdirs.into_iter().map(|(_, path)| path).rev());
    }
    sessions.sort_by_key(|(modified, _)| *modified);
    sessions
        .iter()
        .rev()
        .take(candidate_cap)
        .filter_map(|(_, session_dir)| {
            let recorded = vibe_session_workspace(session_dir)?;
            workspace_matches(&recorded, workspace).then_some(())?;
            vibe_session_id(session_dir)
        })
        .next()
}

/// Deliver a task into a managed Vibe session and arm its capture.
///
/// Runs `vibe --resume <uuid> -p <task> --workdir <workspace> --trust
/// --output streaming --auto-approve` headlessly against the registered
/// managed session, holding the single-writer lease for the duration. On
/// success the lease releases to `Ready`, which arms the capture poller;
/// on failure it releases back to `Queued` and the task stays queued.
///
/// Managed-only: an observed external TUI stays capture-only.
pub fn deliver_vibe_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_vibe_task_with(store, request, run_vibe_worker)
}

#[allow(dead_code)]
pub fn deliver_vibe_task_with(
    store: &HubStore,
    request: &HarnessInjectRequest,
    runner: impl FnOnce(&Path, &str, &str, Option<&str>, Option<&str>) -> Result<Option<u32>, String>,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Vibe active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let workspace_str = workspace.to_string_lossy().into_owned();

    let registration = store.get_harness_session("vibe", &workspace_str)?;
    let is_managed = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);
    if !is_managed {
        return Ok(unavailable(
            "Vibe active-session delivery requires an app-owned managed session. Register one with hub_register_managed_harness_session. Task stays queued.",
        ));
    }
    let registered_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .unwrap_or_default();
    let Some(disk_uuid) = crate::harness::vibe_disk_session_id(&registered_id) else {
        return Ok(unavailable(&format!(
            "registered Vibe session id {registered_id:?} is not a UUID and cannot be resumed with `vibe --resume`. Task stays queued."
        )));
    };

    let writer_owner = format!(
        "vibe-worker:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );
    if let Err(error) = store.acquire_harness_writer("vibe", &workspace_str, &writer_owner) {
        return Ok(queued(&format!(
            "Vibe managed worker in workspace {workspace_str} is busy; task stays queued for retry: {error}"
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
                    harness: "vibe".into(),
                    pid,
                    status: "delivered".into(),
                    detail: format!(
                        "forwarded to managed Vibe session {disk_uuid}; the run's transcript is captured on the next poll"
                    ),
                },
            )
        }
        Err(error) => (
            HarnessSessionState::Queued,
            HarnessInjectResult {
                harness: "vibe".into(),
                pid: None,
                status: "errored".into(),
                detail: format!("Vibe worker failed: {error}"),
            },
        ),
    };
    let _ = store.release_harness_writer("vibe", &workspace_str, &writer_owner, next_state);
    Ok(result)
}

/// Run one headless `vibe --resume <uuid> -p <prompt> --workdir <workspace>
/// --trust --output streaming --auto-approve` task turn and wait for it.
/// Stdout is drained and discarded — `messages.jsonl` is the capture source
/// of truth, so recording stdout risks double-captures under different
/// content hashes. Stderr is dropped for the same pipe-stall reason.
fn run_vibe_worker(
    workspace: &Path,
    prompt: &str,
    disk_uuid: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Option<u32>, String> {
    let args =
        crate::harness::vibe_managed_spawn_args(workspace, prompt, Some(disk_uuid), model, effort)
            .map_err(|error| format!("invalid Vibe spawn args: {error}"))?;
    let mut child = Command::new("vibe")
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to spawn vibe: {error}"))?;
    let pid = Some(child.id());
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = line;
        }
    }
    let status = child
        .wait()
        .map_err(|error| format!("error waiting for vibe: {error}"))?;
    if status.success() {
        Ok(pid)
    } else {
        Err(format!("vibe exited with code {:?}", status.code()))
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "vibe".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

fn queued(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "vibe".into(),
        pid: None,
        status: "queued".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
#[path = "vibe_tests.rs"]
mod vibe_tests;
