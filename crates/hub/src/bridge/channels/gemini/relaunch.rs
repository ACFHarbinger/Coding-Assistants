//! Gemini kill -> capture -> relaunch flow implementation.
#![allow(dead_code)]

use crate::bridge::gemini::{
    latest_gemini_session_id, queued, run_agy_worker, unavailable, AgyStreamOutput,
};
use crate::harness::{HarnessInjectRequest, HarnessInjectResult};
use crate::{
    HarnessSessionMode, HarnessSessionRegistration, HarnessSessionState, HubError, HubStore,
    MessageRecord,
};
use std::path::Path;
use std::process::Command;
use uuid::Uuid;

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
            let worker_texts = output.assistant_texts.join("\n\n");
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
            // Publish the worker's own output back into the Hub work
            // session so a Gemini task result surfaces in Chat & Memory
            // like a Claude Channel `reply` does, instead of being visible
            // only as an on-disk diff.
            let _ = record_gemini_worker_reply(store, request.message_id.as_deref(), &worker_texts);
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

/// Resolves the Hub work-session id and the original sender from a
/// task-inject message id, using the same `channel:session:<id>:...`
/// subject convention the Claude Channel reply path relies on.
fn hub_session_and_sender(store: &HubStore, message_id: Option<&str>) -> Option<(String, String)> {
    let message = store.get_message(message_id?).ok()??;
    let session_id = message
        .subject
        .as_deref()
        .and_then(|subject| subject.strip_prefix("channel:session:"))
        .and_then(|rest| rest.split(':').next())
        .filter(|id| !id.is_empty())?
        .to_string();
    Some((session_id, message.from_agent))
}

/// Posts a managed `agy` worker's own output back into the originating Hub
/// work session as a `gemini` message, so a delivered task's result shows
/// up in Chat & Memory. No-op (returns `Ok(None)`) when the body is empty
/// or the originating message can't be tied to a session.
pub fn record_gemini_worker_reply(
    store: &HubStore,
    message_id: Option<&str>,
    body: &str,
) -> Result<Option<MessageRecord>, HubError> {
    let body = body.trim();
    if body.is_empty() {
        return Ok(None);
    }
    let Some((session_id, recipient)) = hub_session_and_sender(store, message_id) else {
        return Ok(None);
    };
    let subject = format!("channel:session:{session_id}:reply:{}", Uuid::new_v4());
    let sent = store.send_session_message(
        "gemini",
        &session_id,
        &[recipient],
        body,
        Some(&subject),
        None,
        None,
    )?;
    Ok(sent.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageKind;
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

    #[test]
    fn worker_reply_lands_in_the_session_addressed_to_the_task_sender() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let session = store.create_work_session("poc").unwrap();
        let origin = store
            .send_message(
                "human",
                "gemini",
                MessageKind::Message,
                "[TASK] do the thing",
                Some(&format!("channel:session:{}:abcd:kind:task", session.id)),
                None,
                None,
            )
            .unwrap();

        let reply =
            record_gemini_worker_reply(&store, Some(&origin.id), "  done: added the guard  ")
                .unwrap()
                .expect("a reply message");
        assert_eq!(reply.from_agent, "gemini");
        assert_eq!(reply.to_agent, "human");
        assert_eq!(reply.body, "done: added the guard");
        assert_eq!(
            reply
                .subject
                .as_deref()
                .unwrap()
                .split(':')
                .take(3)
                .collect::<Vec<_>>(),
            vec!["channel", "session", session.id.as_str()]
        );
    }

    #[test]
    fn worker_reply_is_a_noop_without_a_session_subject_or_with_an_empty_body() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let plain = store
            .send_message(
                "human",
                "gemini",
                MessageKind::Message,
                "hi",
                None,
                None,
                None,
            )
            .unwrap();
        assert!(record_gemini_worker_reply(&store, Some(&plain.id), "text")
            .unwrap()
            .is_none());
        assert!(record_gemini_worker_reply(&store, None, "text")
            .unwrap()
            .is_none());
        assert!(record_gemini_worker_reply(&store, Some(&plain.id), "   ")
            .unwrap()
            .is_none());
    }
}
