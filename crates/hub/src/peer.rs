//! Peer-to-peer workflow delegation over the desktop TCP channel (P11a).
//!
//! Blocking `std` client — hub store code is sync, so this mirrors the
//! Grok ACP client's spawn-and-read shape instead of async I/O: connect →
//! `Authenticate` → `StartTask` → `TaskComplete`/`Error`. One JSON frame per
//! line, exactly like `src-tauri/src/server/tcp_server.rs` speaks.
//!
//! The pairing token is resolved by the caller through
//! `hub::secret::resolve`; an empty token fails closed before connecting.
//! The key must match the server side (`tcp_auth::TOKEN_VAULT_KEY`).

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Vault/env key for the LAN pairing secret shared with the peer.
pub const PEER_AUTH_KEY: &str = "CA_TCP_AUTH_TOKEN";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IO_TIMEOUT: Duration = Duration::from_secs(10);
const TURN_TIMEOUT: Duration = Duration::from_secs(600);

/// `Authenticate` frame for the pairing token.
pub fn authenticate_frame(token: &str) -> Value {
    json!({ "type": "Authenticate", "token": token })
}

/// `StartTask` frame with a single-role config built from the workflow step.
/// Optional model fields are sent explicitly `null`: the peer deserializes a
/// full `ModelConfig`, and missing non-default keys would fail the parse.
/// `work_dir` is empty — the executing peer runs in its own workspace.
pub fn start_task_frame(agent_name: &str, instruction: &str) -> Value {
    json!({
        "type": "StartTask",
        "config": {
            "roles": [{
                "name": agent_name,
                "config": {
                    "provider": "opencode",
                    "model": "big-pickle",
                    "endpoint": null,
                    "prompt_file": null,
                    "rule_file": null,
                    "workflow_file": null,
                    "skill_file": null
                }
            }],
            "work_dir": "",
            "mcp_config": ""
        },
        "task": instruction
    })
}

/// Run one delegated step on the peer at `host:port`. Returns the remote
/// result text. Broadcast frames (`TaskEvent`, …) are skipped while waiting
/// for the terminal `TaskComplete`/`Error`.
pub fn delegate_remote_step(
    peer: &str,
    agent_name: &str,
    instruction: &str,
    token: &str,
) -> Result<String, String> {
    if token.trim().is_empty() {
        return Err("no peer auth token: pairing is required for remote delegation".into());
    }
    if agent_name.trim().is_empty() {
        return Err("remote step agent name is required".into());
    }
    if instruction.trim().is_empty() {
        return Err("remote step instruction must not be empty".into());
    }
    let addr = peer
        .to_socket_addrs()
        .map_err(|error| format!("cannot resolve peer '{peer}': {error}"))?
        .next()
        .ok_or_else(|| format!("cannot resolve peer '{peer}'"))?;
    let stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
        .map_err(|error| format!("cannot connect to peer '{peer}': {error}"))?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|error| error.to_string())?);
    let mut writer = stream;
    write_frame(&mut writer, &authenticate_frame(token))?;
    let auth = read_frame(&mut reader, TURN_TIMEOUT)?;
    if auth.get("type").and_then(|t| t.as_str()) != Some("Status")
        || auth.get("running").and_then(|r| r.as_bool()) != Some(true)
    {
        return Err(format!(
            "peer rejected authentication: {}",
            auth.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown")
        ));
    }
    write_frame(&mut writer, &start_task_frame(agent_name, instruction))?;
    let deadline = std::time::Instant::now() + TURN_TIMEOUT;
    loop {
        let frame = read_frame_timeout(&mut reader, deadline)?;
        match frame.get("type").and_then(|t| t.as_str()) {
            Some("TaskComplete") => {
                return frame
                    .get("result")
                    .and_then(|r| r.as_str())
                    .map(str::to_string)
                    .ok_or_else(|| "peer TaskComplete carried no result".to_string());
            }
            Some("Error") => {
                return Err(format!(
                    "peer task failed: {}",
                    frame
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown")
                ));
            }
            _ => continue,
        }
    }
}

fn write_frame(writer: &mut impl Write, value: &Value) -> Result<(), String> {
    writeln!(writer, "{value}").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn read_frame(reader: &mut impl BufRead, timeout: Duration) -> Result<Value, String> {
    read_frame_timeout(reader, std::time::Instant::now() + timeout)
}

fn read_frame_timeout(
    reader: &mut impl BufRead,
    deadline: std::time::Instant,
) -> Result<Value, String> {
    let mut line = String::new();
    loop {
        if std::time::Instant::now() >= deadline {
            return Err("timed out waiting for peer".into());
        }
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Err("peer closed the connection".into()),
            Ok(_) => {}
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut
                {
                    continue;
                }
                return Err(error.to_string());
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Ok(value);
        }
    }
}

#[path = "peer_tests.rs"]
#[cfg(test)]
mod peer_tests;
