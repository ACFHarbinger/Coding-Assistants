//! C14.4 Gemini / Antigravity (`agy`) managed worker bridge (#151).
//!
//! Antigravity CLI (`agy`) sessions are tracked on disk under `~/.gemini`.
//! App-managed non-interactive worker processes run using documented flags:
//! `agy --print --output-format stream-json --prompt <prompt>` (and `--conversation <id>`
//! on continuation). Working directory is the child process working directory (`workspace`).
//!
//! One exclusive writer lease is claimed per managed session. Concurrent writers are queued.
//! Observed C12 sessions remain capture-only and return `unavailable` without mutating state.

use crate::{
    gemini_managed_spawn_args, HarnessInjectRequest, HarnessInjectResult, HarnessSessionMode,
    HarnessSessionState, HubError, HubStore,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn gemini_brain_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".gemini")
        .join("antigravity-cli")
        .join("brain")
}

pub fn latest_gemini_session_id(_workspace: &Path) -> Option<String> {
    let brain_dir = gemini_brain_dir();
    let entries = std::fs::read_dir(&brain_dir).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter_map(|dir| {
            let log_file = dir
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");
            if log_file.is_file() {
                let modified = std::fs::metadata(&log_file).ok()?.modified().ok()?;
                let conv_id = dir.file_name()?.to_string_lossy().into_owned();
                Some((modified, conv_id))
            } else {
                None
            }
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, conv_id)| conv_id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum AgyWorkerStatus {
    Idle,
    Running,
    Completed,
    Cancelled,
    Errored(String),
}

#[derive(Debug, Clone, Default)]
pub struct AgyStreamOutput {
    pub conversation_id: Option<String>,
    pub assistant_texts: Vec<String>,
}

pub fn parse_agy_stream_line(line: &str) -> Option<AgyStreamOutput> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(line).ok()?;
    let mut out = AgyStreamOutput::default();

    if let Some(conv) = value
        .get("conversation_id")
        .or_else(|| value.get("conversationId"))
        .and_then(Value::as_str)
    {
        out.conversation_id = Some(conv.to_string());
    }

    let source = value.get("source").and_then(Value::as_str);
    let msg_type = value.get("type").and_then(Value::as_str);

    // Skip tool execution steps (VIEW_FILE, RUN_COMMAND, etc.) and intermediate tool_calls
    let is_tool_step = matches!(
        msg_type,
        Some(
            "VIEW_FILE"
                | "LIST_DIR"
                | "RUN_COMMAND"
                | "REPLACE_FILE_CONTENT"
                | "WRITE_TO_FILE"
                | "MULTI_REPLACE_FILE_CONTENT"
                | "READ_URL_CONTENT"
                | "SEARCH_WEB"
                | "ASK_QUESTION"
                | "DEFINE_SUBAGENT"
                | "INVOKE_SUBAGENT"
                | "MANAGE_SUBAGENTS"
                | "MANAGE_TASK"
                | "SCHEDULE"
                | "SEND_MESSAGE"
                | "GENERATE_IMAGE"
                | "GREP_SEARCH"
        )
    );
    let has_tool_calls = value
        .get("tool_calls")
        .is_some_and(|tc| tc.as_array().is_some_and(|arr| !arr.is_empty()));

    if (source == Some("MODEL") || msg_type == Some("PLANNER_RESPONSE"))
        && !is_tool_step
        && !has_tool_calls
    {
        if let Some(content) = value.get("content").and_then(Value::as_str) {
            let trimmed = content.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with("Created At:")
                && !trimmed.starts_with("Completed At:")
                && !trimmed.starts_with("File Path:")
                && !trimmed.starts_with("<USER_REQUEST>")
                && !trimmed.starts_with("{{ CHECKPOINT")
                && !trimmed.starts_with("```json")
                && !trimmed.contains("The following code has been modified")
            {
                out.assistant_texts.push(trimmed.to_string());
            }
        }
    } else if let Some(text) = value.get("text").and_then(Value::as_str) {
        let trimmed = text.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with("Created At:")
            && !trimmed.starts_with("Completed At:")
            && !trimmed.starts_with("File Path:")
            && !trimmed.starts_with("<USER_REQUEST>")
            && !trimmed.starts_with("{{ CHECKPOINT")
            && !trimmed.starts_with("```json")
        {
            out.assistant_texts.push(trimmed.to_string());
        }
    }

    if out.conversation_id.is_some() || !out.assistant_texts.is_empty() {
        Some(out)
    } else {
        None
    }
}

pub fn deliver_gemini_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_gemini_task_with(store, request, run_agy_worker)
}

pub fn deliver_gemini_task_with(
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
        .or_else(|| store.get_harness_session("agy", &workspace_str).ok().flatten());

    let is_managed = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);

    if !is_managed {
        return Ok(unavailable(
            "Gemini/Antigravity active session delivery requires an app-owned managed session. Register one with hub_register_managed_harness_session. Task stays queued.",
        ));
    }

    let conversation_id = request
        .session_id
        .clone()
        .or_else(|| registration.as_ref().map(|row| row.disk_session_id.clone()))
        .or_else(|| latest_gemini_session_id(&workspace));

    let writer_owner = format!(
        "gemini-worker:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );

    if let Err(error) = store.acquire_harness_writer("gemini", &workspace_str, &writer_owner) {
        return Ok(queued(&format!(
            "Gemini managed worker in workspace {workspace_str} is busy; task stays queued for retry: {error}"
        )));
    }

    let run_res = runner(&workspace, &request.body, conversation_id.as_deref());

    let (next_state, result) = match run_res {
        Ok((pid, output)) => {
            if let Some(ref new_conv) = output.conversation_id {
                let _ = store.register_managed_harness_session(
                    "gemini",
                    &workspace_str,
                    new_conv,
                    pid.unwrap_or_else(std::process::id),
                );
            }
            let detail = if let Some(ref conv) = output.conversation_id {
                format!("Gemini worker completed successfully (conversation: {conv})")
            } else {
                "Gemini worker completed successfully".into()
            };
            (
                HarnessSessionState::Ready,
                HarnessInjectResult {
                    harness: "gemini".into(),
                    pid,
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

fn run_agy_worker(
    workspace: &Path,
    prompt: &str,
    conversation_id: Option<&str>,
) -> Result<(Option<u32>, AgyStreamOutput), String> {
    let args = gemini_managed_spawn_args(workspace, prompt, conversation_id)
        .map_err(|e| format!("invalid spawn args: {e}"))?;

    let mut child = Command::new("agy")
        .args(&args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn agy: {e}"))?;

    let pid = Some(child.id());
    let mut stream_out = AgyStreamOutput::default();

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(evt) = parse_agy_stream_line(&line) {
                if evt.conversation_id.is_some() {
                    stream_out.conversation_id = evt.conversation_id;
                }
                stream_out.assistant_texts.extend(evt.assistant_texts);
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| format!("error waiting for agy: {e}"))?;

    if status.success() {
        Ok((pid, stream_out))
    } else {
        Err(format!("agy exited with code {:?}", status.code()))
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "gemini".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

fn queued(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "gemini".into(),
        pid: None,
        status: "queued".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;
    use tempfile::tempdir;

    #[test]
    fn unmanaged_gemini_delivery_returns_unavailable() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_gemini_task(
            &store,
            &HarnessInjectRequest {
                harness: "gemini".into(),
                workspace: dir.path().to_path_buf(),
                session_id: None,
                message_id: Some("msg-1".into()),
                body: "hello gemini".into(),
                is_task: true,
                is_wake: false,
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert!(result.detail.contains("managed session"));
    }

    #[test]
    fn parse_agy_stream_json_line() {
        let line = r#"{"source":"MODEL","type":"PLANNER_RESPONSE","content":"Analyzed codebase","conversation_id":"conv-123"}"#;
        let parsed = parse_agy_stream_line(line).unwrap();
        assert_eq!(parsed.conversation_id.as_deref(), Some("conv-123"));
        assert_eq!(
            parsed.assistant_texts,
            vec!["Analyzed codebase".to_string()]
        );
    }

    #[test]
    fn managed_gemini_delivery_acquires_and_releases_writer_lease() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path();
        let ws_str = workspace.to_string_lossy().into_owned();

        store
            .register_managed_harness_session("gemini", &ws_str, "conv-owned-1", 1234)
            .unwrap();

        let req = HarnessInjectRequest {
            harness: "gemini".into(),
            workspace: workspace.to_path_buf(),
            session_id: None,
            message_id: Some("msg-test-1".into()),
            body: "build worker component".into(),
            is_task: true,
            is_wake: false,
        };

        let result = deliver_gemini_task_with(&store, &req, |_ws, _prompt, conv_id| {
            assert_eq!(conv_id, Some("conv-owned-1"));
            Ok((
                Some(1234),
                AgyStreamOutput {
                    conversation_id: Some("conv-owned-1".into()),
                    assistant_texts: vec!["Done".into()],
                },
            ))
        })
        .unwrap();

        assert_eq!(result.status, "ok");
        assert_eq!(result.pid, Some(1234));

        let sess = store
            .get_harness_session("gemini", &ws_str)
            .unwrap()
            .unwrap();
        assert_eq!(sess.state, HarnessSessionState::Ready);
        assert!(sess.writer_owner.is_none());
    }

    #[test]
    fn managed_gemini_delivery_returns_queued_when_writer_is_busy() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path();
        let ws_str = workspace.to_string_lossy().into_owned();

        store
            .register_managed_harness_session("gemini", &ws_str, "conv-owned-1", 1234)
            .unwrap();

        store
            .acquire_harness_writer("gemini", &ws_str, "existing-writer")
            .unwrap();

        let req = HarnessInjectRequest {
            harness: "gemini".into(),
            workspace: workspace.to_path_buf(),
            session_id: None,
            message_id: Some("msg-test-2".into()),
            body: "second task".into(),
            is_task: true,
            is_wake: false,
        };

        let result = deliver_gemini_task_with(&store, &req, |_ws, _prompt, _conv| {
            panic!("runner should not execute when busy");
        })
        .unwrap();

        assert_eq!(result.status, "queued");
        assert!(result.detail.contains("busy"));
    }

    #[test]
    fn relative_workspace_is_rejected() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_gemini_task(
            &store,
            &HarnessInjectRequest {
                harness: "gemini".into(),
                workspace: PathBuf::from("relative-workspace"),
                session_id: None,
                message_id: None,
                body: "hello".into(),
                is_task: true,
                is_wake: false,
            },
        );
        assert!(result.is_err());
    }
}
