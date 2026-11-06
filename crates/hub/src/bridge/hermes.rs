//! Nous Research Hermes Agent CLI bridge — session discovery + managed delivery (C14.16, #322).
//!
//! Hermes keeps sessions in SQLite (`~/.hermes/`) but ships first-party
//! machine-readable surfaces, so nothing is scraped: the free per-call
//! `--usage-file` JSON reports carry `session_id` (discover-after-spawn,
//! mirroring Vibe's `meta.json.session_id`), and
//! `hermes sessions export --format jsonl --cwd <dir>` enumerates a
//! workspace's sessions as one full session object per line.
//!
//! Task delivery re-enters the managed session headlessly via
//! `hermes --resume <id> --in <dir> -z <task>` (final-text-only stdout),
//! holding the single-writer lease for the duration. One-shot runs exit
//! immediately, so managed registrations store no pid (a dead pid would
//! grey the presence dot) and stay `Ready`, arming capture polling.

use crate::harness::{
    hermes_disk_session_id, hermes_managed_spawn_args, hermes_usage_dir, resolve_hermes_path,
    HarnessStartResult,
};
use crate::{
    HarnessInjectRequest, HarnessInjectResult, HarnessSessionMode, HarnessSessionRegistration,
    HarnessSessionState, HubError, HubStore,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

/// Hermes home directory (`~/.hermes`): config, SQLite store, sessions.
pub fn hermes_home_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hermes")
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "hermes".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

fn queued(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "hermes".into(),
        pid: None,
        status: "queued".into(),
        detail: detail.into(),
    }
}

/// Parse one `sessions export --format jsonl` line into
/// `(session_id, last_activity)`. `last_activity_at` is the recency signal;
/// missing/zero falls back to `started_at`, and a line with no id is skipped.
fn parse_export_line(line: &serde_json::Value) -> Option<(String, i64)> {
    let id = line.get("id").and_then(|v| v.as_str())?;
    if id.trim().is_empty() {
        return None;
    }
    let activity = line
        .get("last_activity_at")
        .and_then(|v| v.as_str())
        .and_then(parse_hermes_time)
        .filter(|t| *t > 0)
        .or_else(|| {
            line.get("started_at")
                .and_then(|v| v.as_str())
                .and_then(parse_hermes_time)
        })
        .unwrap_or(0);
    Some((id.to_string(), activity))
}

/// Hermes timestamps are `YYYYMMDD_HHMMSS_microfrac` (`20260912_232548_916eae`)
/// or epoch seconds (`1789251948.6968367`); both verified live.
fn parse_hermes_time(raw: &str) -> Option<i64> {
    if let Ok(epoch) = raw.parse::<f64>() {
        if epoch.is_finite() && epoch > 0.0 {
            return Some(epoch as i64);
        }
    }
    // Lexicographic order matches chronological order for the fixed-width
    // `YYYYMMDD_HHMMSS_*` shape, so hash it into an order-preserving rank.
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 14 {
        digits[..14].parse::<i64>().ok()
    } else {
        None
    }
}

/// Newest session id in an export payload for `workspace` (matched by `cwd`),
/// or the newest overall when no workspace matches.
pub fn latest_session_from_export(stdout: &str, workspace: Option<&Path>) -> Option<String> {
    let wanted = workspace.map(|ws| ws.to_string_lossy().into_owned());
    let mut best: Option<(String, i64)> = None;
    let mut fallback: Option<(String, i64)> = None;
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some((id, activity)) = parse_export_line(&value) else {
            continue;
        };
        let entry = Some((id, activity));
        let cwd = value
            .get("cwd")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        match (&wanted, cwd) {
            (Some(want), got) if got == want => {
                if best.as_ref().map(|(_, t)| activity > *t).unwrap_or(true) {
                    best = entry;
                }
            }
            _ => {
                if fallback
                    .as_ref()
                    .map(|(_, t)| activity > *t)
                    .unwrap_or(true)
                {
                    fallback = entry;
                }
            }
        }
    }
    best.or(fallback).map(|(id, _)| id)
}

/// Run `hermes sessions export --format jsonl` scoped to `workspace` into a
/// temp file and return its stdout text. Export prints a human summary line
/// to stdout alongside writing the file, so callers must read the file.
fn export_workspace_sessions(workspace: &Path) -> Result<String, String> {
    let out_path = std::env::temp_dir().join(format!(
        "hermes-export-{}.jsonl",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let status = Command::new(resolve_hermes_path())
        .args([
            "sessions",
            "export",
            "--format",
            "jsonl",
            "--cwd",
            &workspace.to_string_lossy(),
        ])
        .arg(&out_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("could not run `hermes sessions export`: {e}"))?;
    if !status.success() {
        return Err(format!("`hermes sessions export` exited with {status}"));
    }
    let text =
        fs::read_to_string(&out_path).map_err(|e| format!("cannot read hermes export: {e}"))?;
    let _ = fs::remove_file(&out_path);
    Ok(text)
}

/// Newest Hermes session id for `workspace` via the first-party export.
pub fn latest_hermes_session_id(workspace: &Path) -> Option<String> {
    let text = export_workspace_sessions(workspace).ok()?;
    latest_session_from_export(&text, Some(workspace))
}

/// `session_id` from the newest `--usage-file` report written after `since` —
/// the exact discover-after-spawn signal for managed one-shot runs.
pub fn latest_usage_session_id(since: SystemTime) -> Option<String> {
    let dir = hermes_usage_dir();
    let entries = fs::read_dir(&dir).ok()?;
    let mut newest: Option<(SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let modified = entry.metadata().and_then(|m| m.modified()).ok()?;
        if modified < since {
            continue;
        }
        if newest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
            newest = Some((modified, path));
        }
    }
    let (_, path) = newest?;
    let raw = fs::read(&path).ok()?;
    let report: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    let id = report.get("session_id").and_then(|v| v.as_str())?;
    hermes_disk_session_id(id)
}

fn run_hermes_worker(workspace: &Path, prompt: &str) -> Result<Option<u32>, String> {
    let args = hermes_managed_spawn_args(workspace, prompt, None, None, None)
        .map_err(|e| e.to_string())?;
    let mut child = Command::new(resolve_hermes_path())
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn hermes worker: {e}"))?;
    let pid = child.id();
    let status = child
        .wait()
        .map_err(|e| format!("failed waiting for hermes worker: {e}"))?;
    if !status.success() {
        return Err(format!("hermes worker exited with status {status}"));
    }
    Ok(Some(pid))
}

fn run_hermes_managed_task(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Option<u32>, String> {
    let args = hermes_managed_spawn_args(workspace, prompt, session_id, model, effort)
        .map_err(|e| e.to_string())?;
    let mut child = Command::new(resolve_hermes_path())
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn hermes worker: {e}"))?;
    let pid = child.id();
    let status = child
        .wait()
        .map_err(|e| format!("failed waiting for hermes task worker: {e}"))?;
    if !status.success() {
        return Err(format!("hermes task worker exited with status {status}"));
    }
    Ok(Some(pid))
}

pub fn start_hermes_managed_harness(
    store: &HubStore,
    workspace: &Path,
    prompt: &str,
) -> Result<(HarnessStartResult, HarnessSessionRegistration), String> {
    start_hermes_managed_harness_with(store, workspace, prompt, run_hermes_worker)
}

pub fn start_hermes_managed_harness_with(
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

    if let Ok(Some(existing)) = store.get_harness_session("hermes", &workspace_key) {
        if let Some(pid) = existing.managed_pid {
            crate::bridge::relaunch::kill_pid(pid);
        }
    }

    let started_at = SystemTime::now();
    let _pid = runner(&workspace, prompt)?;
    // Discover-then-register: the one-shot has already exited, so read the
    // exact session id back from its `--usage-file` report (written even on
    // failure) rather than guessing from recency.
    let disk_id = latest_usage_session_id(started_at).ok_or_else(|| {
        "hermes started but no usage report was found; managed session not registered".to_string()
    })?;

    // The one-shot pid is dead by design; storing it would grey the presence
    // dot, so register with none and stay Ready (same rationale as Kimi).
    let registration = store
        .register_managed_harness_session_with_state(
            "hermes",
            &workspace_key,
            &disk_id,
            None,
            HarnessSessionState::Ready,
        )
        .map_err(|e| e.to_string())?;

    Ok((
        HarnessStartResult {
            harness: "hermes".into(),
            pid: None,
            status: "started".into(),
            detail: format!("managed Hermes session {disk_id} ready"),
        },
        registration,
    ))
}

pub fn deliver_hermes_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_hermes_task_with(store, request, run_hermes_managed_task)
}

pub fn deliver_hermes_task_with(
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
            "Hermes active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let workspace_str = workspace.to_string_lossy().into_owned();

    let registration = store.get_harness_session("hermes", &workspace_str)?;
    let is_managed = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);
    if !is_managed {
        return Ok(unavailable(
            "Hermes active-session delivery requires an app-owned managed session. Register one with hub_register_managed_harness_session. Task stays queued.",
        ));
    }

    let registered_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .unwrap_or_default();
    let Some(disk_session) = hermes_disk_session_id(&registered_id) else {
        return Ok(unavailable(&format!(
            "registered Hermes session id {registered_id:?} is invalid and cannot be resumed. Task stays queued."
        )));
    };

    let writer_owner = format!(
        "hermes-worker:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );
    if let Err(error) = store.acquire_harness_writer("hermes", &workspace_str, &writer_owner) {
        return Ok(queued(&format!(
            "Hermes managed worker in workspace {workspace_str} is busy; task stays queued for retry: {error}"
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
                    harness: "hermes".into(),
                    pid,
                    status: "delivered".into(),
                    detail: format!(
                        "forwarded to managed Hermes session {disk_session}; the run's transcript is captured on the next poll"
                    ),
                },
            )
        }
        Err(error) => (
            HarnessSessionState::Queued,
            HarnessInjectResult {
                harness: "hermes".into(),
                pid: None,
                status: "errored".into(),
                detail: format!("Hermes worker failed: {error}"),
            },
        ),
    };

    let _ = store.release_harness_writer("hermes", &workspace_str, &writer_owner, next_state);
    Ok(result)
}

#[cfg(test)]
#[path = "hermes_tests.rs"]
mod tests;
