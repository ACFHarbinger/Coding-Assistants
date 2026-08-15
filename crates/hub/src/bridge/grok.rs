//! C12-GROK-BRIDGE: discover an already-running Grok session and deliver a
//! queued Hub task through the documented ACP + leader path.
//!
//! Grok Build's supported attach is `grok agent --leader stdio` talking to
//! `~/.grok/leader.sock` (or `GROK_LEADER_SOCKET`), then `session/load` +
//! `session/prompt`. That process is an ACP *client* of the existing leader,
//! not a replacement TUI. If the leader socket is absent, delivery is
//! `unavailable` and the task stays queued.
//!
//! Starting a leader-mode TUI lives in `bridge::channels::grok` and must
//! not be required for this module to refuse safely.

pub use crate::bridge::channels::grok::{
    connect_grok_leader_session, grok_leader_status, GrokConnectResult,
};

use crate::{HarnessInjectRequest, HarnessInjectResult, HubError, HubStore};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn grok_home() -> PathBuf {
    if let Ok(path) = std::env::var("GROK_HOME") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".grok")
}

/// Percent-encode an absolute workspace the same way Grok names session dirs.
pub fn encode_workspace_dir_name(workspace: &Path) -> String {
    workspace
        .to_string_lossy()
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// Most recent Grok conversation id for `workspace`, used by "Resume in
/// terminal" and the leader flow.
///
/// #165: prefer the *live* TUI's session id from
/// `~/.grok/active_sessions.json` when one is active for this workspace —
/// the disk-scan fallback can miss a conversation the live TUI has not yet
/// flushed to `chat_history.jsonl`, which made "Resume in terminal" spawn
/// a fresh conversation instead of continuing the one the user is looking
/// at. Falls back to the newest on-disk `chat_history.jsonl` directory.
pub fn latest_grok_session_id(workspace: &Path) -> Option<String> {
    if let Some(active) = active_grok_session_for(workspace) {
        return Some(active.session_id);
    }
    let root = grok_home()
        .join("sessions")
        .join(encode_workspace_dir_name(workspace));
    let entries = std::fs::read_dir(root).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|dir| {
            let history = dir.join("chat_history.jsonl");
            let modified = std::fs::metadata(&history).ok()?.modified().ok()?;
            let id = dir.file_name()?.to_string_lossy().into_owned();
            Some((modified, id))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, id)| id)
}

/// A Grok TUI currently listed in `~/.grok/active_sessions.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ActiveGrokSession {
    pub session_id: String,
    pub pid: u32,
    pub cwd: String,
    #[serde(default)]
    pub opened_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ActiveSessionFile {
    session_id: String,
    pid: u32,
    cwd: String,
    #[serde(default)]
    pub opened_at: Option<String>,
}

#[allow(dead_code)]
pub fn parse_active_grok_sessions(raw: &str) -> Vec<ActiveGrokSession> {
    let Ok(rows) = serde_json::from_str::<Vec<ActiveSessionFile>>(raw) else {
        return Vec::new();
    };
    rows.into_iter()
        .map(|row| ActiveGrokSession {
            session_id: row.session_id,
            pid: row.pid,
            cwd: row.cwd,
            opened_at: row.opened_at,
        })
        .collect()
}

#[allow(dead_code)]
pub fn list_active_grok_sessions() -> Vec<ActiveGrokSession> {
    let Ok(raw) = std::fs::read_to_string(grok_home().join("active_sessions.json")) else {
        return Vec::new();
    };
    parse_active_grok_sessions(&raw)
}

#[allow(dead_code)]
pub fn active_grok_session_for(workspace: &Path) -> Option<ActiveGrokSession> {
    let wanted = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    list_active_grok_sessions().into_iter().find(|row| {
        let cwd = PathBuf::from(&row.cwd);
        cwd == wanted || cwd.canonicalize().ok().as_ref() == Some(&wanted)
    })
}

pub fn default_leader_socket() -> PathBuf {
    if let Ok(path) = std::env::var("GROK_LEADER_SOCKET") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    grok_home().join("leader.sock")
}

pub fn leader_socket_available(path: &Path) -> bool {
    path.exists()
}

pub fn acp_initialize() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": 1,
            "clientInfo": { "name": "coding-assistants", "version": "0.1.0" },
            "clientCapabilities": {
                "fs": { "readTextFile": true, "writeTextFile": true },
                "terminal": true
            }
        }
    })
}

pub fn acp_session_load(session_id: &str, cwd: &Path) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "session/load",
        "params": {
            "sessionId": session_id,
            "cwd": cwd,
            "mcpServers": []
        }
    })
}

pub fn acp_session_prompt(session_id: &str, text: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "session/prompt",
        "params": {
            "sessionId": session_id,
            "prompt": [{ "type": "text", "text": text }]
        }
    })
}

pub fn grok_acp_client_args(socket: &Path) -> Vec<String> {
    vec![
        "agent".into(),
        "--leader".into(),
        "--leader-socket".into(),
        socket.display().to_string(),
        "stdio".into(),
    ]
}

fn run_acp_prompt(
    socket: &Path,
    workspace: &Path,
    session_id: &str,
    text: &str,
) -> Result<String, String> {
    let mut child = Command::new("grok")
        .args(grok_acp_client_args(socket))
        .current_dir(workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not start Grok ACP client: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Grok ACP stdin unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Grok ACP stdout unavailable".to_string())?;
    let mut reader = BufReader::new(stdout);
    write_rpc(&mut stdin, &acp_initialize())?;
    wait_rpc_result(&mut reader, 1, Duration::from_secs(8))?;
    write_rpc(&mut stdin, &acp_session_load(session_id, workspace))?;
    wait_rpc_result(&mut reader, 2, Duration::from_secs(8))?;
    write_rpc(&mut stdin, &acp_session_prompt(session_id, text))?;
    let reply = collect_prompt_reply(&mut reader, Duration::from_secs(45))?;
    let _ = child.kill();
    let _ = child.wait();
    if reply.trim().is_empty() {
        return Err("Grok session/prompt returned no assistant text".into());
    }
    Ok(reply)
}

fn write_rpc(stdin: &mut impl Write, value: &Value) -> Result<(), String> {
    writeln!(stdin, "{value}").map_err(|error| error.to_string())?;
    stdin.flush().map_err(|error| error.to_string())
}

fn wait_rpc_result(reader: &mut impl BufRead, id: i64, timeout: Duration) -> Result<Value, String> {
    let deadline = Instant::now() + timeout;
    let mut line = String::new();
    while Instant::now() < deadline {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytes == 0 {
            return Err("Grok ACP client closed".into());
        }
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if value.get("id").and_then(|item| item.as_i64()) == Some(id) {
            if value.get("error").is_some() {
                return Err(format!("Grok ACP error: {}", value["error"]));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }
    Err(format!("timed out waiting for Grok ACP id {id}"))
}

fn collect_prompt_reply(reader: &mut impl BufRead, timeout: Duration) -> Result<String, String> {
    let deadline = Instant::now() + timeout;
    let mut line = String::new();
    let mut reply = String::new();
    while Instant::now() < deadline {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytes == 0 {
            break;
        }
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if value.get("method").and_then(|item| item.as_str()) == Some("session/update") {
            let update = value
                .pointer("/params/update")
                .or_else(|| value.pointer("/params"));
            if let Some(update) = update {
                if update.get("sessionUpdate").and_then(|item| item.as_str())
                    == Some("agent_message_chunk")
                {
                    if let Some(text) = update
                        .pointer("/content/text")
                        .and_then(|item| item.as_str())
                        .or_else(|| update.get("text").and_then(|item| item.as_str()))
                    {
                        reply.push_str(text);
                    }
                }
            }
        }
        if value.get("id").and_then(|item| item.as_i64()) == Some(3) {
            if value.get("error").is_some() {
                return Err(format!("Grok session/prompt error: {}", value["error"]));
            }
            break;
        }
    }
    Ok(reply)
}

/// Deliver a task into a registered Grok session. Never writes a TTY/PTY and
/// never starts a replacement interactive `grok` TUI.
pub fn deliver_grok_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Grok active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let registration = store.get_harness_session("grok", &workspace.to_string_lossy())?;
    // `request.session_id` is the Hub work-session id, not Grok's disk/ACP
    // session id. Never pass it to `session/load`.
    let session_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .or_else(|| latest_grok_session_id(&workspace));
    let Some(session_id) = session_id else {
        return Ok(unavailable(
            "no registered or on-disk Grok session for this workspace; use Shared Hub → Channels → Connect Grok (leader mode), or register one with hub_register_harness_session",
        ));
    };
    let socket = registration
        .as_ref()
        .and_then(|row| row.leader_socket.as_deref())
        .map(PathBuf::from)
        .unwrap_or_else(default_leader_socket);
    if !leader_socket_available(&socket) {
        return Ok(unavailable(&format!(
            "no leader socket at {} — start Grok with --leader (or [cli] use_leader = true) to enable delivery. Task stays queued.",
            socket.display()
        )));
    }

    match run_acp_prompt(&socket, &workspace, &session_id, &request.body) {
        Ok(reply) => {
            if let Some(message_id) = request.message_id.as_deref() {
                let _ = store.set_message_status(message_id, crate::MessageStatus::Acked);
            }
            let _ = store.record_harness_capture(
                "grok",
                "grok",
                request.session_id.as_deref(),
                &reply,
                Some(&workspace.to_string_lossy()),
            );
            Ok(HarnessInjectResult {
                harness: "grok".into(),
                pid: None,
                status: "delivered".into(),
                detail: format!(
                    "forwarded to registered Grok session {session_id} via leader {}",
                    socket.display()
                ),
            })
        }
        Err(error) => Ok(unavailable(&error)),
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "grok".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
#[path = "grok_tests.rs"]
mod grok_tests;
