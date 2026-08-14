//! Kill the honestly-live process for a harness in one workspace.
//!
//! Does not relaunch. Liveness matches [`super::presence`]: Claude is the
//! Channel session (never the terminal-emulator `managed_pid`); Chat /
//! Gemini are a still-running managed pid; Grok is that pid and/or the
//! workspace `--leader` TUI. A dead Hub row is not something to kill.

use super::channels::claude::channel_bridge_pids;
use super::channels::gemini::kill_managed_agy_process;
use super::claude::{find_active_claude_session, list_active_claude_sessions};
use super::grok::active_grok_session_for;
use super::relaunch::kill_pid;
use crate::harness::HarnessId;
use crate::{HarnessSessionRegistration, HubStore};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StopManagedOutcome {
    pub harness: String,
    pub killed_pids: Vec<u32>,
    pub detail: String,
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn same_workspace(registered: &str, requested: &Path) -> bool {
    canonical(Path::new(registered)) == canonical(requested)
}

fn aliases(harness: HarnessId) -> &'static [&'static str] {
    match harness {
        HarnessId::Chat => &["chat", "codex"],
        HarnessId::Gemini => &["gemini", "agy"],
        HarnessId::Grok => &["grok"],
        HarnessId::Claude => &["claude"],
        HarnessId::OpenCode => &["opencode"],
        HarnessId::Vibe => &["vibe"],
    }
}

fn matching_rows<'a>(
    sessions: &'a [HarnessSessionRegistration],
    harness: HarnessId,
    workspace: &Path,
) -> Vec<&'a HarnessSessionRegistration> {
    let names = aliases(harness);
    sessions
        .iter()
        .filter(|row| {
            names.contains(&row.harness.as_str()) && same_workspace(&row.workspace, workspace)
        })
        .collect()
}

fn terminate(harness: HarnessId, pid: u32) -> bool {
    if pid == std::process::id() {
        return false;
    }
    if harness == HarnessId::Gemini {
        kill_managed_agy_process(pid)
    } else {
        kill_pid(pid)
    }
}

fn mark_rows_stopped(store: &HubStore, rows: &[&HarnessSessionRegistration]) {
    for row in rows {
        let _ = store.mark_harness_session_stopped(&row.harness, &row.workspace);
    }
}

fn stop_claude(store: &HubStore, workspace: &Path) -> Result<StopManagedOutcome, String> {
    let listed = store
        .list_harness_sessions()
        .map_err(|error| error.to_string())?;
    let rows = matching_rows(&listed, HarnessId::Claude, workspace);
    let mut killed = Vec::new();

    if let Ok(sessions) = list_active_claude_sessions() {
        // find_active_claude_session only ever returns an interactive
        // entry with a real pid, but the field itself stays Option<u32>
        // (background/Task-tool entries in the same roster have none).
        if let Some(pid) = find_active_claude_session(&sessions, workspace).and_then(|s| s.pid) {
            if terminate(HarnessId::Claude, pid) {
                killed.push(pid);
            }
        }
    }
    if let Ok(bridges) = channel_bridge_pids(workspace) {
        for pid in bridges {
            if terminate(HarnessId::Claude, pid) {
                killed.push(pid);
            }
        }
    }

    if killed.is_empty() {
        return Err(
            "nothing honestly live to kill for claude in this workspace (Channel session is the signal; the stored terminal-emulator pid is not)"
                .into(),
        );
    }
    mark_rows_stopped(store, &rows);
    Ok(StopManagedOutcome {
        harness: "claude".into(),
        killed_pids: killed,
        detail: "Stopped the live Claude Channel session for this workspace".into(),
    })
}

fn stop_pid_backed(
    store: &HubStore,
    harness: HarnessId,
    workspace: &Path,
) -> Result<StopManagedOutcome, String> {
    let listed = store
        .list_harness_sessions()
        .map_err(|error| error.to_string())?;
    let rows = matching_rows(&listed, harness, workspace);
    let mut killed = Vec::new();
    for row in &rows {
        if let Some(pid) = row.managed_pid {
            if terminate(harness, pid) {
                killed.push(pid);
            }
        }
    }
    if harness == HarnessId::Grok {
        if let Some(live) = active_grok_session_for(workspace) {
            if !killed.contains(&live.pid) && terminate(harness, live.pid) {
                killed.push(live.pid);
            }
        }
    }
    if killed.is_empty() {
        return Err(format!(
            "nothing honestly live to kill for {} in this workspace",
            harness.as_str()
        ));
    }
    mark_rows_stopped(store, &rows);
    Ok(StopManagedOutcome {
        harness: harness.as_str().into(),
        killed_pids: killed,
        detail: format!(
            "Stopped the live {} session for this workspace",
            harness.as_str()
        ),
    })
}

/// Kill the process that actually backs liveness for `harness` in
/// `workspace`. Errors when nothing live is there to kill.
pub fn stop_managed_harness(
    store: &HubStore,
    harness_id: &str,
    workspace: &Path,
) -> Result<StopManagedOutcome, String> {
    let harness = HarnessId::parse(harness_id).map_err(|error| error.to_string())?;
    if !workspace.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    match harness {
        HarnessId::Claude => stop_claude(store, workspace),
        other => stop_pid_backed(store, other, workspace),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::relaunch::is_pid_running;
    use crate::{HarnessSessionMode, HarnessSessionState};
    use std::process::Command;

    #[test]
    fn stop_rejects_a_relative_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err = stop_managed_harness(&store, "chat", Path::new("relative/path")).unwrap_err();
        assert!(err.contains("absolute"), "{err}");
    }

    #[test]
    fn stop_rejects_an_unknown_harness() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err =
            stop_managed_harness(&store, "not-a-harness", Path::new("/abs/repo")).unwrap_err();
        assert!(err.contains("unknown harness"), "{err}");
    }

    #[test]
    fn stop_chat_kills_a_registered_managed_pid() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = sleeper.id();
        store
            .register_managed_harness_session("chat", "/abs/repo", "thread-1", pid)
            .unwrap();
        let outcome = stop_managed_harness(&store, "chat", Path::new("/abs/repo")).unwrap();
        assert_eq!(outcome.killed_pids, vec![pid]);
        assert!(!is_pid_running(pid));
        let row = store
            .get_harness_session("chat", "/abs/repo")
            .unwrap()
            .unwrap();
        assert_eq!(row.state, HarnessSessionState::Stopped);
        assert!(row.managed_pid.is_none());
        assert_eq!(row.mode, HarnessSessionMode::Managed);
        let _ = sleeper.kill();
        let _ = sleeper.wait();
    }

    #[test]
    fn stop_claude_does_not_kill_a_terminal_emulator_managed_pid() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = sleeper.id();
        store
            .register_managed_harness_session("claude", "/abs/repo", "channel:1", pid)
            .unwrap();
        let err = stop_managed_harness(&store, "claude", Path::new("/abs/repo")).unwrap_err();
        assert!(err.contains("terminal-emulator"), "{err}");
        assert!(
            is_pid_running(pid),
            "stored emulator pid must be left alone"
        );
        let row = store
            .get_harness_session("claude", "/abs/repo")
            .unwrap()
            .unwrap();
        assert_eq!(row.managed_pid, Some(pid));
        let _ = sleeper.kill();
        let _ = sleeper.wait();
    }
}
