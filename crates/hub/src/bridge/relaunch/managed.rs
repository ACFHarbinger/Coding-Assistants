//! Headless managed-worker lifecycle for `start_managed_harness`: spawn the
//! harness's documented one-shot worker, register it with the Hub, and kill
//! any prior managed pid first so a second "Start managed" never orphans the
//! process the Hub still tracks.
//!
//! Split out of `bridge::relaunch` to keep every source unit within the
//! 500-line cap (#158). The interactive kill/resume helpers stay in the
//! parent module.

use crate::harness::{start_harness_owned, HarnessId, HarnessStartRequest, HarnessStartResult};
use crate::{HarnessSessionRegistration, HubStore};
use std::path::Path;
use std::process::{Child, ExitStatus};
use std::time::Duration;
use uuid::Uuid;

use super::kill_pid;

const IMMEDIATE_EXIT_GRACE: Duration = Duration::from_millis(750);

fn fresh_managed_session_id() -> String {
    format!("managed-{}", Uuid::new_v4())
}

fn exit_detail(harness: &str, status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!(
            "{harness} managed worker exited immediately (exit code {code}). Retry Start managed."
        ),
        None => format!(
            "{harness} managed worker exited immediately after a signal. Retry Start managed."
        ),
    }
}

fn finish_child(store: &HubStore, harness: &str, workspace: &str, pid: u32, status: ExitStatus) {
    let _ = store.finish_managed_harness_process(harness, workspace, pid, status.success());
}

fn reap_managed_child(
    child: Child,
    data_dir: std::path::PathBuf,
    harness: String,
    workspace: String,
    pid: u32,
) {
    std::thread::spawn(move || {
        let Ok(output) = child.wait_with_output() else {
            return;
        };
        let Ok(store) = HubStore::open(data_dir) else {
            return;
        };
        finish_child(&store, &harness, &workspace, pid, output.status);
    });
}

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
    // A caller-provided id may name a global, pre-existing provider session.
    // Never register it for a new managed worker: doing so arms the capture
    // poller against someone else's transcript. Gemini accepts this UUID as
    // its new conversation id; other one-shot CLIs create their own session,
    // which remains unarmed until a provider bridge can identify it.
    let _requested_disk_session_id = disk_session_id;
    let disk_session_id = fresh_managed_session_id();
    let workspace_key = workspace.to_string_lossy().to_string();

    if let Ok(Some(existing)) = store.get_harness_session(harness.as_str(), &workspace_key) {
        if let Some(pid) = existing.managed_pid {
            kill_pid(pid);
        }
    }

    let (mut started, mut child) = start_harness_owned(&HarnessStartRequest {
        harness: harness.as_str().into(),
        workspace: workspace.to_path_buf(),
        session_id: Some(disk_session_id.clone()),
        prompt: prompt.into(),
        ..Default::default()
    })
    .map_err(|error| error.to_string())?;

    let Some(pid) = started.pid else {
        return Err(started.detail);
    };

    let registration = store
        .register_managed_harness_session_with_state(
            harness.as_str(),
            &workspace_key,
            &disk_session_id,
            Some(pid),
            crate::HarnessSessionState::Queued,
        )
        .map_err(|error| error.to_string())?;

    // A bad argv, missing TTY, or rejected startup commonly exits before the
    // first liveness poll. Wait briefly while we still own the Child so the
    // UI receives a truthful, retryable error and the process is reaped.
    std::thread::sleep(IMMEDIATE_EXIT_GRACE);
    if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
        started.pid = None;
        if status.success() {
            started.status = "started".into();
            started.detail = format!(
                "{} managed startup completed; session is awaiting its first task before capture starts.",
                harness.as_str()
            );
        } else {
            finish_child(store, harness.as_str(), &workspace_key, pid, status);
            started.status = "unavailable".into();
            started.detail = exit_detail(harness.as_str(), status);
        }
        let registration = store
            .get_harness_session(harness.as_str(), &workspace_key)
            .map_err(|error| error.to_string())?
            .unwrap_or(registration);
        return Ok((started, registration));
    }

    reap_managed_child(
        child,
        store.data_dir().to_path_buf(),
        harness.as_str().into(),
        workspace_key,
        pid,
    );

    Ok((started, registration))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::relaunch::is_pid_running;
    use std::process::Command;
    use std::time::Instant;

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
    fn managed_start_id_is_fresh_and_never_uses_a_requested_provider_id() {
        let first = fresh_managed_session_id();
        let second = fresh_managed_session_id();
        assert!(first.starts_with("managed-"));
        assert_ne!(first, second);
        assert_ne!(first, "global-provider-session");
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

    #[test]
    fn managed_reaper_waits_for_exit_and_marks_only_its_pid_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let child = Command::new("sh")
            .args(["-c", "exit 23"])
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let pid = child.id();
        store
            .register_managed_harness_session("chat", "/abs/repo", "thread-1", pid)
            .unwrap();

        reap_managed_child(
            child,
            dir.path().to_path_buf(),
            "chat".into(),
            "/abs/repo".into(),
            pid,
        );

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let row = store
                .get_harness_session("chat", "/abs/repo")
                .unwrap()
                .unwrap();
            if row.state == crate::HarnessSessionState::Unavailable {
                assert_eq!(row.managed_pid, None);
                break;
            }
            assert!(
                Instant::now() < deadline,
                "managed child was not reaped in time"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !is_pid_running(pid),
            "completed managed child must be reaped"
        );
    }
}
