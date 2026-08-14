//! Claude-specific "Start managed": kill-prior, then a Channel-connected
//! terminal. Lives next to [`super::relaunch`] so that file stays generic
//! across harnesses and under 500 LoC.

use super::relaunch::{kill_pid, latest_session_id};
use crate::harness::{HarnessId, HarnessStartResult};
use crate::{
    is_channel_session_live, launch_claude_channel_session, HarnessSessionRegistration,
    HarnessSessionState, HubStore,
};
use std::path::Path;
use std::time::Duration;

const CLAUDE_CHANNEL_WAIT_ATTEMPTS: u32 = 16;
const CLAUDE_CHANNEL_WAIT_MS: u64 = 400;

fn kill_prior_managed_pid(store: &HubStore, harness: &str, workspace_key: &str) {
    if let Ok(Some(existing)) = store.get_harness_session(harness, workspace_key) {
        if let Some(pid) = existing.managed_pid {
            if pid != std::process::id() {
                kill_pid(pid);
            }
        }
    }
}

fn wait_for_claude_channel(workspace: &Path) -> bool {
    for _ in 0..CLAUDE_CHANNEL_WAIT_ATTEMPTS {
        if is_channel_session_live(workspace).unwrap_or(false) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(CLAUDE_CHANNEL_WAIT_MS));
    }
    is_channel_session_live(workspace).unwrap_or(false)
}

fn claude_live_pid(workspace: &Path) -> Option<u32> {
    let sessions = crate::bridge::claude::list_active_claude_sessions().ok()?;
    crate::bridge::claude::find_active_claude_session(&sessions, workspace).and_then(|s| s.pid)
}

fn register_claude_channel(
    store: &HubStore,
    workspace: &Path,
    workspace_key: &str,
    live: bool,
    fallback_pid: Option<u32>,
) -> Result<HarnessSessionRegistration, String> {
    let disk_session_id = latest_session_id(HarnessId::Claude, workspace)
        .unwrap_or_else(|| format!("channel:{}", chrono::Utc::now().timestamp()));
    let pid = claude_live_pid(workspace).or(fallback_pid);
    let state = if live {
        HarnessSessionState::Ready
    } else {
        HarnessSessionState::Queued
    };
    store
        .register_managed_harness_session_with_state(
            "claude",
            workspace_key,
            &disk_session_id,
            pid,
            state,
        )
        .map_err(|error| error.to_string())
}

/// Kill any prior registered Claude pid, then open a Channel-connected
/// `claude` terminal (same invocation as Channels → Connect). Readiness
/// is [`is_channel_session_live`], not the terminal-emulator pid — Claude
/// has no durable headless worker to track the way Codex/Gemini do.
pub fn start_managed_claude_channel(
    store: &HubStore,
    workspace: &Path,
) -> Result<(HarnessStartResult, HarnessSessionRegistration), String> {
    if !workspace.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    let workspace_key = workspace.to_string_lossy().to_string();

    if is_channel_session_live(workspace).unwrap_or(false) {
        let registration = register_claude_channel(store, workspace, &workspace_key, true, None)?;
        return Ok((
            HarnessStartResult {
                harness: "claude".into(),
                pid: registration.managed_pid,
                status: "started".into(),
                detail: "Channel-connected Claude Code session is already live for this workspace"
                    .into(),
            },
            registration,
        ));
    }

    kill_prior_managed_pid(store, "claude", &workspace_key);
    if let Some(pid) = claude_live_pid(workspace) {
        if pid != std::process::id() {
            kill_pid(pid);
            std::thread::sleep(Duration::from_millis(300));
        }
    }

    let terminal_pid = launch_claude_channel_session(workspace)?;
    let live = wait_for_claude_channel(workspace);
    let registration =
        register_claude_channel(store, workspace, &workspace_key, live, Some(terminal_pid))?;
    let detail = if live {
        "Opened a Channel-connected Claude Code terminal; Channel session is live".into()
    } else {
        "Opened a Channel-connected Claude Code terminal; waiting for the Channel bridge to come up"
            .into()
    };
    Ok((
        HarnessStartResult {
            harness: "claude".into(),
            pid: registration.managed_pid,
            status: if live { "started" } else { "queued" }.into(),
            detail,
        },
        registration,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::relaunch::is_pid_running;
    use super::*;
    use std::process::Command;

    #[test]
    fn start_managed_claude_channel_rejects_a_relative_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err = start_managed_claude_channel(&store, Path::new("relative/path")).unwrap_err();
        assert!(err.contains("absolute"), "{err}");
    }

    #[test]
    fn kill_prior_managed_pid_kills_a_registered_pid_and_skips_self() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
        let prior_pid = sleeper.id();
        store
            .register_managed_harness_session("claude", "/abs/repo", "old-channel", prior_pid)
            .unwrap();
        assert!(is_pid_running(prior_pid));
        kill_prior_managed_pid(&store, "claude", "/abs/repo");
        assert!(!is_pid_running(prior_pid));
        let _ = sleeper.kill();
        let _ = sleeper.wait();

        store
            .register_managed_harness_session("claude", "/abs/other", "self", std::process::id())
            .unwrap();
        kill_prior_managed_pid(&store, "claude", "/abs/other");
        assert!(is_pid_running(std::process::id()));
    }
}
