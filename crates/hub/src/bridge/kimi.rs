//! Moonshot Kimi Code CLI bridge — session discovery + managed delivery (C14.14, #309).
//!
//! Kimi Code CLI stores sessions under `~/.kimi-code/sessions/wd_<slug>_<hash>/session_<uuid>/`
//! (or `$KIMI_CODE_HOME/sessions/...`) with `state.json` carrying metadata and the transcript
//! at `agents/main/wire.jsonl`. It also provides a machine-readable session enumerator
//! via `kimi session list --json [--cwd <path>]`.
//!
//! Task delivery re-enters the managed session headlessly via
//! `kimi -p <task> --session <id> --output-format stream-json`, holding the single-writer
//! lease for the duration. On success the lease releases to `Ready`, arming capture polling;
//! on failure it releases back to `Queued`.

use crate::harness::{
    kimi_disk_session_id, kimi_executable, kimi_managed_spawn_args, HarnessStartResult,
};
use crate::{
    HarnessInjectRequest, HarnessInjectResult, HarnessSessionMode, HarnessSessionRegistration,
    HarnessSessionState, HubError, HubStore,
};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Kimi home directory honoring `$KIMI_CODE_HOME` or defaulting to `~/.kimi-code`.
pub fn kimi_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("KIMI_CODE_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".kimi-code")
}

/// Kimi sessions root under the home directory.
pub fn kimi_sessions_root() -> PathBuf {
    kimi_home_dir().join("sessions")
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KimiSessionSummary {
    pub id: String,
    #[serde(rename = "workDir")]
    pub work_dir: Option<String>,
    #[serde(rename = "sessionDir")]
    pub session_dir: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<u64>,
    #[serde(default)]
    pub archived: bool,
}

/// Discover active/recent sessions using `kimi session list --json`.
pub fn list_kimi_sessions(workspace: Option<&Path>) -> Result<Vec<KimiSessionSummary>, String> {
    let program = kimi_executable();
    let mut cmd = Command::new(program);
    cmd.args(["session", "list", "--json"]);
    if let Some(ws) = workspace {
        cmd.args(["--cwd", &ws.to_string_lossy()]);
    }
    let output = cmd
        .output()
        .map_err(|error| format!("could not run `kimi session list --json`: {error}"))?;
    if !output.status.success() {
        return Err(format!("`kimi session list` exited with {}", output.status));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("unexpected `kimi session list` output: {error}"))
}

/// Check if a recorded workspace path matches the wanted workspace.
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

/// Read session details from a session directory's `state.json`.
fn read_session_state(session_dir: &Path) -> Option<(String, String, u64, bool)> {
    let state_path = session_dir.join("state.json");
    let raw = fs::read(&state_path).ok()?;
    let value = serde_json::from_slice::<serde_json::Value>(&raw).ok()?;
    let id = value.get("id").and_then(|v| v.as_str())?.to_string();
    let cwd = value.get("cwd").and_then(|v| v.as_str())?.to_string();
    let updated_at = value.get("updatedAt").and_then(|v| v.as_u64()).unwrap_or(0);
    let archived = value
        .get("archived")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Some((id, cwd, updated_at, archived))
}

/// Discover latest session ID for a workspace: tries CLI first, then inspects disk.
pub fn latest_kimi_session_id(workspace: &Path) -> Option<String> {
    // 1. Try `kimi session list --cwd <ws> --json`
    if let Ok(sessions) = list_kimi_sessions(Some(workspace)) {
        for s in sessions {
            if !s.archived {
                if let Some(ref wd) = s.work_dir {
                    if workspace_matches(wd, workspace) {
                        return Some(s.id);
                    }
                } else {
                    return Some(s.id);
                }
            }
        }
    }

    // 2. Fallback to direct directory inspection
    latest_kimi_session_id_from(&kimi_sessions_root(), workspace, 200)
}

/// Most recent Kimi session id for `workspace` under `sessions_root`, newest-first.
pub fn latest_kimi_session_id_from(
    sessions_root: &Path,
    workspace: &Path,
    candidate_cap: usize,
) -> Option<String> {
    if !sessions_root.is_dir() {
        return None;
    }

    let mut matches: Vec<(u64, String)> = Vec::new();
    let Ok(wd_entries) = fs::read_dir(sessions_root) else {
        return None;
    };

    for wd_entry in wd_entries.filter_map(Result::ok) {
        let wd_path = wd_entry.path();
        if !wd_path.is_dir() {
            continue;
        }
        let Ok(session_entries) = fs::read_dir(&wd_path) else {
            continue;
        };
        for sess_entry in session_entries.filter_map(Result::ok) {
            let sess_path = sess_entry.path();
            if !sess_path.is_dir() {
                continue;
            }
            if let Some((id, cwd, updated_at, archived)) = read_session_state(&sess_path) {
                if !archived && workspace_matches(&cwd, workspace) {
                    matches.push((updated_at, id));
                }
            }
        }
    }

    matches.sort_by_key(|(updated_at, _)| *updated_at);
    matches
        .into_iter()
        .rev()
        .take(candidate_cap)
        .map(|(_, id)| id)
        .next()
}

fn run_kimi_worker(workspace: &Path, prompt: &str) -> Result<Option<u32>, String> {
    let args =
        kimi_managed_spawn_args(workspace, prompt, None, None, None).map_err(|e| e.to_string())?;
    let program = kimi_executable();
    let mut child = Command::new(program)
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn kimi worker: {e}"))?;
    let pid = child.id();
    let status = child
        .wait()
        .map_err(|e| format!("failed waiting for kimi worker: {e}"))?;
    if !status.success() {
        return Err(format!("kimi worker exited with status {status}"));
    }
    Ok(Some(pid))
}

fn run_kimi_managed_task(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Option<u32>, String> {
    let args = kimi_managed_spawn_args(workspace, prompt, session_id, model, effort)
        .map_err(|e| e.to_string())?;
    let program = kimi_executable();
    let mut child = Command::new(program)
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn kimi worker: {e}"))?;
    let pid = child.id();
    let status = child
        .wait()
        .map_err(|e| format!("failed waiting for kimi task worker: {e}"))?;
    if !status.success() {
        return Err(format!("kimi task worker exited with status {status}"));
    }
    Ok(Some(pid))
}

/// Start a fresh managed Kimi worker and register its session with the Hub store.
pub fn start_kimi_managed_harness(
    store: &HubStore,
    workspace: &Path,
    prompt: &str,
) -> Result<(HarnessStartResult, HarnessSessionRegistration), String> {
    start_kimi_managed_harness_with(store, workspace, prompt, run_kimi_worker)
}

pub fn start_kimi_managed_harness_with(
    store: &HubStore,
    workspace: &Path,
    prompt: &str,
    runner: impl FnOnce(&Path, &str) -> Result<Option<u32>, String>,
) -> Result<(HarnessStartResult, HarnessSessionRegistration), String> {
    if !workspace.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let workspace_key = workspace.to_string_lossy().into_owned();

    if let Ok(Some(existing)) = store.get_harness_session("kimi", &workspace_key) {
        if let Some(pid) = existing.managed_pid {
            crate::bridge::relaunch::kill_pid(pid);
        }
    }

    let pid = runner(&workspace, prompt)?;
    let disk_id = latest_kimi_session_id(&workspace).ok_or_else(|| {
        "kimi started but no matching session was found; managed session not registered".to_string()
    })?;

    let registration = store
        .register_managed_harness_session_with_state(
            "kimi",
            &workspace_key,
            &disk_id,
            pid,
            HarnessSessionState::Ready,
        )
        .map_err(|e| e.to_string())?;

    Ok((
        HarnessStartResult {
            harness: "kimi".into(),
            pid,
            status: "started".into(),
            detail: format!("Kimi managed session started (session {disk_id})"),
        },
        registration,
    ))
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "kimi".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

fn queued(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "kimi".into(),
        pid: None,
        status: "queued".into(),
        detail: detail.into(),
    }
}

/// Deliver a task into a managed Kimi session and arm capture polling.
pub fn deliver_kimi_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_kimi_task_with(store, request, run_kimi_managed_task)
}

pub fn deliver_kimi_task_with(
    store: &HubStore,
    request: &HarnessInjectRequest,
    runner: impl FnOnce(
        &Path,
        &str,
        Option<&str>,
        Option<&str>,
        Option<&str>,
    ) -> Result<Option<u32>, String>,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Kimi active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let workspace_str = workspace.to_string_lossy().into_owned();

    let registration = store.get_harness_session("kimi", &workspace_str)?;
    let is_managed = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);
    if !is_managed {
        return Ok(unavailable(
            "Kimi active-session delivery requires an app-owned managed session. Register one with hub_register_managed_harness_session. Task stays queued.",
        ));
    }

    let registered_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .unwrap_or_default();
    let Some(disk_session) = kimi_disk_session_id(&registered_id) else {
        return Ok(unavailable(&format!(
            "registered Kimi session id {registered_id:?} is invalid and cannot be resumed. Task stays queued."
        )));
    };

    let writer_owner = format!(
        "kimi-worker:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );
    if let Err(error) = store.acquire_harness_writer("kimi", &workspace_str, &writer_owner) {
        return Ok(queued(&format!(
            "Kimi managed worker in workspace {workspace_str} is busy; task stays queued for retry: {error}"
        )));
    }

    let run_res = runner(
        &workspace,
        &request.body,
        Some(&disk_session),
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
                    harness: "kimi".into(),
                    pid,
                    status: "delivered".into(),
                    detail: format!(
                        "forwarded to managed Kimi session {disk_session}; the run's transcript is captured on the next poll"
                    ),
                },
            )
        }
        Err(error) => (
            HarnessSessionState::Queued,
            HarnessInjectResult {
                harness: "kimi".into(),
                pid: None,
                status: "errored".into(),
                detail: format!("Kimi worker failed: {error}"),
            },
        ),
    };

    let _ = store.release_harness_writer("kimi", &workspace_str, &writer_owner, next_state);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn latest_kimi_session_id_from_disk_reads_state_json() {
        let dir = tempdir().unwrap();
        let sessions_root = dir.path().join("sessions");
        let ws_slug_dir = sessions_root.join("wd_my-project_abc123");
        let session_dir = ws_slug_dir.join("session_1111-2222");
        fs::create_dir_all(&session_dir).unwrap();

        let state = serde_json::json!({
            "id": "session_1111-2222",
            "cwd": "/path/to/my-project",
            "updatedAt": 1789126200000_u64,
            "archived": false
        });
        fs::write(
            session_dir.join("state.json"),
            serde_json::to_vec(&state).unwrap(),
        )
        .unwrap();

        let found =
            latest_kimi_session_id_from(&sessions_root, Path::new("/path/to/my-project"), 10);
        assert_eq!(found, Some("session_1111-2222".to_string()));

        let missing = latest_kimi_session_id_from(&sessions_root, Path::new("/other/project"), 10);
        assert_eq!(missing, None);
    }
}
