//! Headless managed-worker lifecycle for `start_managed_harness`: spawn the
//! harness's documented one-shot worker, register it with the Hub, and kill
//! any prior managed pid first so a second "Start managed" never orphans the
//! process the Hub still tracks.
//!
//! Split out of `bridge::relaunch` to keep every source unit within the
//! 500-line cap (#158). The interactive kill/resume helpers stay in the
//! parent module.

use crate::harness::{HarnessId, HarnessStartRequest, HarnessStartResult};
use crate::{HarnessSessionRegistration, HubStore};
use std::path::Path;

use super::kill_pid;

/// Starts a headless managed harness worker (`harness::start_harness`) and
/// registers it, first killing any prior managed pid already registered for
/// `(harness, workspace)`.
///
/// Without this, "Start managed" orphaned the previous process: each
/// registration unconditionally overwrites `managed_pid` on the existing
/// Hub row (`register_harness_session`'s documented conflict behavior), so
/// a second click swapped which pid the Hub tracked without anything ever
/// killing the one it stopped tracking. Same kill primitive as
/// `relaunch_harness_in_terminal`, just for the headless one-shot spawn
/// instead of an interactive terminal.
pub fn start_managed_harness(
    store: &HubStore,
    harness_id: &str,
    workspace: &Path,
    disk_session_id: &str,
    prompt: &str,
) -> Result<(HarnessStartResult, HarnessSessionRegistration), String> {
    let harness = HarnessId::parse(harness_id).map_err(|error| error.to_string())?;
    if harness == HarnessId::Claude {
        return Err(
            "Claude has no headless managed worker; Start managed must open a Channel-connected terminal"
                .into(),
        );
    }
    if !workspace.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    let disk_session_id = disk_session_id.trim();
    if disk_session_id.is_empty() {
        return Err(
            "start_managed_harness requires a real disk/thread/conversation id, not a placeholder"
                .into(),
        );
    }
    let workspace_key = workspace.to_string_lossy().to_string();

    if let Ok(Some(existing)) = store.get_harness_session(harness.as_str(), &workspace_key) {
        if let Some(pid) = existing.managed_pid {
            kill_pid(pid);
        }
    }

    let started = crate::harness::start_harness(&HarnessStartRequest {
        harness: harness.as_str().into(),
        workspace: workspace.to_path_buf(),
        session_id: None,
        prompt: prompt.into(),
    })
    .map_err(|error| error.to_string())?;

    let Some(pid) = started.pid else {
        return Err(started.detail);
    };

    let registration = store
        .register_managed_harness_session(harness.as_str(), &workspace_key, disk_session_id, pid)
        .map_err(|error| error.to_string())?;

    Ok((started, registration))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::relaunch::is_pid_running;
    use std::process::Command;

    #[test]
    fn start_managed_harness_rejects_a_relative_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err =
            start_managed_harness(&store, "grok", Path::new("relative/path"), "thread-1", "hi")
                .unwrap_err();
        assert!(err.contains("absolute"), "{err}");
    }

    #[test]
    fn start_managed_harness_rejects_an_empty_disk_session_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err =
            start_managed_harness(&store, "grok", Path::new("/abs/repo"), "   ", "hi").unwrap_err();
        assert!(err.contains("real disk"), "{err}");
    }

    #[test]
    fn start_managed_harness_rejects_claude_headless_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err =
            start_managed_harness(&store, "claude", Path::new("/abs/repo"), "session-1", "hi")
                .unwrap_err();
        assert!(err.contains("Channel-connected"), "{err}");
    }

    #[test]
    fn start_managed_harness_rejects_an_unknown_harness() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err = start_managed_harness(
            &store,
            "not-a-harness",
            Path::new("/abs/repo"),
            "thread-1",
            "hi",
        )
        .unwrap_err();
        assert!(err.contains("unknown harness"), "{err}");
    }

    #[test]
    fn start_managed_harness_kills_a_prior_registered_managed_pid_before_replacing_it() {
        // A real, controllable long-lived process this test owns end-to-end
        // (not a harness CLI, which may not be installed wherever this runs).
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
        let prior_pid = sleeper.id();
        store
            .register_managed_harness_session("grok", "/abs/repo", "old-thread", prior_pid)
            .unwrap();
        assert!(
            is_pid_running(prior_pid),
            "precondition: sleeper must be alive"
        );

        // grok itself may not be installed in every environment this runs
        // in; either outcome (spawn succeeds and re-registers, or grok is
        // unavailable and this returns Err) is fine — the property under
        // test is only that the *prior* pid was killed either way, since
        // the kill happens before the new spawn is attempted.
        let _ = start_managed_harness(&store, "grok", Path::new("/abs/repo"), "new-thread", "hi");

        assert!(
            !is_pid_running(prior_pid),
            "the previously registered managed pid must be killed, not orphaned"
        );
        let _ = sleeper.kill();
        let _ = sleeper.wait();
    }
}
