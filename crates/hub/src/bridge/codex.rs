//! C12 Codex / Chat bridge: deliver a queued Hub task into a persisted
//! Codex thread through the documented app-server JSON-RPC protocol
//! (`initialize`, `thread/resume`, `turn/start`).
//!
//! This never writes another process's PTY, never invents a control socket,
//! and never starts a replacement `codex exec` process for a task-only
//! inject. Without a registered or on-disk thread id the task stays
//! `unavailable` / queued.

use crate::{HarnessInjectRequest, HarnessInjectResult, HubError, HubStore};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn latest_codex_thread_id(workspace: &Path) -> Option<String> {
    latest_codex_thread_id_from(&codex_sessions_dir(), workspace)
}

fn codex_sessions_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".codex").join("sessions")
}

fn latest_codex_thread_id_from(sessions_root: &Path, workspace: &Path) -> Option<String> {
    let workspace = workspace.to_string_lossy();
    let mut best: Option<(std::time::SystemTime, String)> = None;
    let years = std::fs::read_dir(sessions_root).ok()?;
    for year in years.filter_map(Result::ok) {
        let months = std::fs::read_dir(year.path()).ok().into_iter().flatten();
        for month in months.filter_map(Result::ok) {
            let days = std::fs::read_dir(month.path()).ok().into_iter().flatten();
            for day in days.filter_map(Result::ok) {
                let files = std::fs::read_dir(day.path()).ok().into_iter().flatten();
                for file in files.filter_map(Result::ok) {
                    let path = file.path();
                    if path.extension().is_none_or(|ext| ext != "jsonl") {
                        continue;
                    }
                    let Some((cwd, session_id)) = session_meta(&path) else {
                        continue;
                    };
                    if cwd != workspace {
                        continue;
                    }
                    let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
                    if best.as_ref().is_none_or(|(stamp, _)| modified > *stamp) {
                        best = Some((modified, session_id));
                    }
                }
            }
        }
    }
    best.map(|(_, id)| id)
}

fn session_meta(path: &Path) -> Option<(String, String)> {
    let raw = std::fs::read_to_string(path).ok()?;
    raw.lines().take(16).find_map(|line| {
        let value = serde_json::from_str::<Value>(line).ok()?;
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            return None;
        }
        let payload = value.get("payload")?;
        Some((
            payload.get("cwd")?.as_str()?.to_string(),
            payload.get("session_id")?.as_str()?.to_string(),
        ))
    })
}

pub fn deliver_codex_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_codex_task_with(store, request, send_codex_turn)
}

fn deliver_codex_task_with(
    store: &HubStore,
    request: &HarnessInjectRequest,
    send_turn: impl FnOnce(&str, &str) -> Result<String, String>,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Codex active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let registration = store.get_harness_session("chat", &workspace.to_string_lossy())?;
    let thread_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .or_else(|| latest_codex_thread_id(&workspace));
    let Some(thread_id) = thread_id else {
        return Ok(unavailable(
            "no registered or on-disk Codex thread for this workspace; register one with hub_register_harness_session (diskSessionId = thread id). Task stays queued.",
        ));
    };

    match send_turn(&thread_id, &request.body) {
        Ok(turn) => {
            if let Some(message_id) = request.message_id.as_deref() {
                let _ = store.set_message_status(message_id, crate::MessageStatus::Acked);
            }
            Ok(HarnessInjectResult {
                harness: "chat".into(),
                pid: None,
                status: "delivered".into(),
                detail: format!("forwarded to Codex thread {thread_id} via app-server ({turn})"),
            })
        }
        Err(error) => Ok(unavailable(&error)),
    }
}

fn send_codex_turn(thread_id: &str, body: &str) -> Result<String, String> {
    let mut child = Command::new("codex")
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("codex app-server unavailable: {error}"))?;
    let result = (|| {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "codex app-server stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "codex app-server stdout unavailable".to_string())?;
        let mut stdout = BufReader::new(stdout);
        write_rpc(
            &mut stdin,
            1,
            "initialize",
            json!({ "clientInfo": { "name": "coding-assistants", "version": "0.1.0" } }),
        )?;
        writeln!(
            stdin,
            "{}",
            json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} })
        )
        .and_then(|_| stdin.flush())
        .map_err(|error| error.to_string())?;
        read_rpc_result(&mut stdout, 1)?;
        write_rpc(
            &mut stdin,
            2,
            "thread/resume",
            json!({ "threadId": thread_id }),
        )?;
        read_rpc_result(&mut stdout, 2)?;
        write_rpc(
            &mut stdin,
            3,
            "turn/start",
            json!({
                "threadId": thread_id,
                "input": [{ "type": "text", "text": body }]
            }),
        )?;
        let started = read_rpc_result(&mut stdout, 3)?;
        let turn_id = started
            .get("turn")
            .and_then(|turn| turn.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("started")
            .to_string();
        Ok(turn_id)
    })();
    let _ = child.kill();
    result
}

fn write_rpc(stdin: &mut impl Write, id: i64, method: &str, params: Value) -> Result<(), String> {
    writeln!(
        stdin,
        "{}",
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    )
    .and_then(|_| stdin.flush())
    .map_err(|error| error.to_string())
}

fn read_rpc_result(stdout: &mut impl BufRead, id: i64) -> Result<Value, String> {
    for line in stdout.lines().take(200) {
        let line = line.map_err(|error| error.to_string())?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("id") != Some(&Value::from(id)) {
            continue;
        }
        if let Some(error) = value.get("error") {
            return Err(format!("codex app-server error: {error}"));
        }
        return Ok(value.get("result").cloned().unwrap_or(json!({})));
    }
    Err("codex app-server returned no matching response".into())
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "chat".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;
    use tempfile::tempdir;

    fn request(workspace: &Path) -> HarnessInjectRequest {
        HarnessInjectRequest {
            harness: "chat".into(),
            workspace: workspace.to_path_buf(),
            session_id: Some("hub-session".into()),
            message_id: Some("msg-1".into()),
            body: "review the adapter".into(),
            is_task: true,
            is_wake: false,
        }
    }

    #[test]
    fn missing_thread_is_unavailable_and_does_not_spawn() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_codex_task_with(
            &store,
            &request(Path::new("/tmp/c12-codex")),
            |_thread, _body| {
                panic!("must not contact app-server without a thread");
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.pid, None);
        assert!(result.detail.contains("queued") || result.detail.contains("register"));
    }

    #[test]
    fn registered_thread_is_delivered_through_injected_app_server() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session("chat", "/tmp/c12-codex", "thread-abc", None)
            .unwrap();
        let result = deliver_codex_task_with(
            &store,
            &request(Path::new("/tmp/c12-codex")),
            |thread, body| {
                assert_eq!(thread, "thread-abc");
                assert_eq!(body, "review the adapter");
                Ok("turn-9".into())
            },
        )
        .unwrap();
        assert_eq!(result.status, "delivered");
        assert_eq!(result.pid, None);
        assert!(result.detail.contains("thread-abc"));
    }

    #[test]
    fn empty_body_and_relative_workspace_are_rejected() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let mut empty = request(Path::new("/tmp/c12-codex"));
        empty.body = "  ".into();
        assert!(deliver_codex_task_with(&store, &empty, |_, _| Ok("x".into())).is_err());
        let relative = request(Path::new("relative"));
        assert!(deliver_codex_task_with(&store, &relative, |_, _| Ok("x".into())).is_err());
    }
}
