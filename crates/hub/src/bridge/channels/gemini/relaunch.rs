//! Gemini kill -> capture -> relaunch flow implementation.
#![allow(dead_code)]

use crate::bridge::gemini::{
    latest_gemini_session_id, queued, run_agy_worker, unavailable, AgyStreamOutput,
};
use crate::harness::{HarnessInjectRequest, HarnessInjectResult};
use crate::{
    HarnessSessionMode, HarnessSessionRegistration, HarnessSessionState, HubError, HubStore,
};
use std::path::Path;
use std::process::Command;

/// Returns true if process `pid` is currently running.
pub fn is_pid_running(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Terminates a running `agy` process by `pid`.
/// Sends SIGTERM, waits briefly, and sends SIGKILL if it hasn't exited.
pub fn kill_managed_agy_process(pid: u32) -> bool {
    if !is_pid_running(pid) {
        return false;
    }
    // Send SIGTERM
    let _ = Command::new("kill")
        .args(["-15", &pid.to_string()])
        .output();

    std::thread::sleep(std::time::Duration::from_millis(150));

    if is_pid_running(pid) {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    true
}

/// Parses stdout/stderr of an exiting `agy` process for resume conversation ID.
pub fn parse_agy_resume_conversation_id(output: &str) -> Option<String> {
    for line in output.lines() {
        let line = line.trim();
        if let Some(idx) = line.find("--conversation=") {
            let rest = &line[idx + "--conversation=".len()..];
            let conv = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('"')
                .trim_matches('\'');
            if !conv.is_empty() {
                return Some(conv.to_string());
            }
        }
        if let Some(idx) = line.find("--conversation ") {
            let rest = &line[idx + "--conversation ".len()..];
            let conv = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('"')
                .trim_matches('\'');
            if !conv.is_empty() {
                return Some(conv.to_string());
            }
        }
    }
    None
}

/// Resolves the effective conversation ID UUID for continuing Gemini `agy` session.
pub fn resolve_gemini_continuation_id(
    workspace: &Path,
    registration: Option<&HarnessSessionRegistration>,
    request_session_id: Option<&str>,
    captured_id: Option<&str>,
) -> Option<String> {
    if let Some(captured) = captured_id {
        if !captured.trim().is_empty() {
            return Some(captured.trim().to_string());
        }
    }
    if let Some(req_id) = request_session_id {
        if !req_id.trim().is_empty() {
            return Some(req_id.trim().to_string());
        }
    }
    if let Some(reg) = registration {
        if !reg.disk_session_id.trim().is_empty() {
            return Some(reg.disk_session_id.trim().to_string());
        }
    }
    latest_gemini_session_id(workspace)
}

/// Performs kill -> capture -> relaunch delivery for Gemini managed session tasks/wakes.
pub fn relaunch_and_deliver_gemini_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    relaunch_and_deliver_gemini_task_with(store, request, run_agy_worker)
}

pub fn relaunch_and_deliver_gemini_task_with(
    store: &HubStore,
    request: &HarnessInjectRequest,
    runner: impl FnOnce(&Path, &str, Option<&str>) -> Result<(Option<u32>, AgyStreamOutput), String>,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Gemini active-session delivery requires an absolute workspace".into(),
        ));
    }

    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let workspace_str = workspace.to_string_lossy().into_owned();

    let registration = store
        .get_harness_session("gemini", &workspace_str)?
        .or_else(|| {
            store
                .get_harness_session("agy", &workspace_str)
                .ok()
                .flatten()
        });

    let captured_id: Option<String> = None;

    // Step 1: Kill running process if managed process exists
    if let Some(ref reg) = registration {
        if let Some(pid) = reg.managed_pid {
            if is_pid_running(pid) {
                kill_managed_agy_process(pid);
            }
        }
    }

    // Step 2: Resolve conversation ID
    let conversation_id = resolve_gemini_continuation_id(
        &workspace,
        registration.as_ref(),
        request.session_id.as_deref(),
        captured_id.as_deref(),
    );

    let is_managed = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);

    if !is_managed {
        return Ok(unavailable(
            "Gemini/Antigravity active session delivery requires an app-owned managed session. Register one with hub_register_managed_harness_session. Task stays queued.",
        ));
    }

    let writer_owner = format!(
        "gemini-worker:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );

    if let Err(error) = store.acquire_harness_writer("gemini", &workspace_str, &writer_owner) {
        return Ok(queued(&format!(
            "Gemini managed worker in workspace {workspace_str} is busy; task stays queued for retry: {error}"
        )));
    }

    // Step 4: Relaunch worker with conversation_id
    let run_res = runner(&workspace, &request.body, conversation_id.as_deref());

    let (next_state, result) = match run_res {
        Ok((pid, output)) => {
            let new_pid = pid.unwrap_or_else(std::process::id);
            let final_conv = output
                .conversation_id
                .or(conversation_id)
                .unwrap_or_else(|| "general".to_string());
            let _ = store.register_managed_harness_session(
                "gemini",
                &workspace_str,
                &final_conv,
                new_pid,
            );
            let detail = format!("Gemini managed session relaunched and delivered successfully (conversation: {final_conv})");
            (
                HarnessSessionState::Ready,
                HarnessInjectResult {
                    harness: "gemini".into(),
                    pid: Some(new_pid),
                    status: "ok".into(),
                    detail,
                },
            )
        }
        Err(err) => (
            HarnessSessionState::Queued,
            HarnessInjectResult {
                harness: "gemini".into(),
                pid: None,
                status: "errored".into(),
                detail: format!("Gemini worker failed: {err}"),
            },
        ),
    };

    let _ = store.release_harness_writer("gemini", &workspace_str, &writer_owner, next_state);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_agy_resume_conversation_id_from_stdout() {
        let text = "CLI output...\nResume with -c (or command below): agy --conversation=12345-abc-6789\nDone";
        assert_eq!(
            parse_agy_resume_conversation_id(text).as_deref(),
            Some("12345-abc-6789")
        );
    }

    #[test]
    fn resolves_continuation_id_priority() {
        let dir = tempdir().unwrap();
        let id = resolve_gemini_continuation_id(dir.path(), None, Some("req-id"), Some("cap-id"));
        assert_eq!(id.as_deref(), Some("cap-id"));

        let id2 = resolve_gemini_continuation_id(dir.path(), None, Some("req-id"), None);
        assert_eq!(id2.as_deref(), Some("req-id"));
    }
}
