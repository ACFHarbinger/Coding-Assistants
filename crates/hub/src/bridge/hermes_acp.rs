//! Hermes ACP delivery client (issue #329).
//!
//! Speaks to `hermes acp`, the first-party stdio ACP server shipped with the
//! Hermes Agent CLI (`hermes acp --check` verifies the install; handshake is
//! standard ACP `initialize`, protocolVersion 1). Same category as Kimi's
//! `kimi acp` and the Grok leader ACP client in [super::grok]: one-shot
//! `initialize` → `session/new` → `session/prompt`, collecting the
//! `agent_message_chunk` texts until the prompt result arrives.
//!
//! Strict-sandbox posture (same class as Vibe/Qwen, per #322): the server is
//! NEVER spawned with `--accept-hooks`, and any `session/request_permission`
//! from the server is answered with a deny — tool approval is never granted
//! headlessly. The existing CLI-arg delivery path (`hermes_spawn.rs` +
//! usage-file discovery + `sessions export` capture + `--usage-file` quota)
//! stays untouched until ACP parity is proven; nothing here changes inject
//! dispatch, capture, quota, or health.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const INIT_TIMEOUT: Duration = Duration::from_secs(10);
const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

/// Argv for the ACP server. Exactly `acp` — never `--accept-hooks`
/// (strict-sandbox block: unseen shell hooks must not auto-approve).
pub fn hermes_acp_args() -> Vec<String> {
    vec!["acp".into()]
}

/// Standard ACP `initialize` frame identifying this client.
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

/// Open a fresh server-side session rooted at `workspace`. One-shot delivery
/// always starts a new session (mirrors the `-z` one-shot CLI path); resume
/// of an existing ACP session is out of scope for this slice.
pub fn acp_session_new(workspace: &Path) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "session/new",
        "params": {
            "cwd": workspace,
            "mcpServers": []
        }
    })
}

/// Prompt frame for an open session.
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

/// Deny frame for a `session/request_permission` server request. The request
/// `id` is echoed back verbatim (number or string); the outcome is always a
/// cancellation — approval is never granted headlessly.
pub fn acp_permission_deny(request_id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": { "outcome": { "outcome": "cancelled" } }
    })
}

/// Run one headless ACP prompt turn: spawn `binary acp`, handshake, open a
/// session, prompt, collect assistant text. Returns `(session_id, reply)`.
/// stderr is discarded (the server logs plugin discovery there); stdout must
/// be newline-delimited JSON-RPC.
pub fn run_hermes_acp_prompt(
    binary: &Path,
    workspace: &Path,
    text: &str,
) -> Result<(String, String), String> {
    if text.trim().is_empty() {
        return Err("Hermes ACP prompt text must not be empty".into());
    }
    let mut child = Command::new(binary)
        .args(hermes_acp_args())
        .current_dir(workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not start Hermes ACP server: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Hermes ACP stdin unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Hermes ACP stdout unavailable".to_string())?;
    let mut reader = BufReader::new(stdout);
    let outcome = run_turn(&mut stdin, &mut reader, workspace, text);
    let _ = child.kill();
    let _ = child.wait();
    outcome
}

fn run_turn(
    stdin: &mut impl Write,
    reader: &mut impl BufRead,
    workspace: &Path,
    text: &str,
) -> Result<(String, String), String> {
    write_rpc(stdin, &acp_initialize())?;
    wait_rpc_result(stdin, reader, &json!(1), INIT_TIMEOUT)?;
    write_rpc(stdin, &acp_session_new(workspace))?;
    let new_result = wait_rpc_result(stdin, reader, &json!(2), INIT_TIMEOUT)?;
    let session_id = new_result
        .get("sessionId")
        .and_then(|item| item.as_str())
        .ok_or_else(|| "Hermes ACP session/new returned no sessionId".to_string())?
        .to_owned();
    write_rpc(stdin, &acp_session_prompt(&session_id, text))?;
    let reply = collect_prompt_reply(stdin, reader, PROMPT_TIMEOUT)?;
    if reply.trim().is_empty() {
        return Err("Hermes ACP session/prompt returned no assistant text".into());
    }
    Ok((session_id, reply))
}

fn write_rpc(stdin: &mut impl Write, value: &Value) -> Result<(), String> {
    writeln!(stdin, "{value}").map_err(|error| error.to_string())?;
    stdin.flush().map_err(|error| error.to_string())
}

/// Wait for the response frame whose `id` equals `id`, answering any
/// `session/request_permission` traffic with a deny while waiting.
fn wait_rpc_result(
    stdin: &mut impl Write,
    reader: &mut impl BufRead,
    id: &Value,
    timeout: Duration,
) -> Result<Value, String> {
    let deadline = Instant::now() + timeout;
    let mut line = String::new();
    while Instant::now() < deadline {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytes == 0 {
            return Err("Hermes ACP server closed".into());
        }
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if value.get("method").and_then(|item| item.as_str()) == Some("session/request_permission")
        {
            let request_id = value.get("id").cloned().unwrap_or(Value::Null);
            write_rpc(stdin, &acp_permission_deny(&request_id))?;
            continue;
        }
        if value.get("id") == Some(id) {
            if value.get("error").is_some() {
                return Err(format!("Hermes ACP error: {}", value["error"]));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }
    Err(format!("timed out waiting for Hermes ACP id {id}"))
}

/// Collect `agent_message_chunk` texts until the id-3 prompt result (or
/// error) arrives, denying permission requests along the way. Needs `stdin`
/// because denies are written back to the server mid-turn.
fn collect_prompt_reply(
    stdin: &mut impl Write,
    reader: &mut impl BufRead,
    timeout: Duration,
) -> Result<String, String> {
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
        if value.get("method").and_then(|item| item.as_str()) == Some("session/request_permission")
        {
            let request_id = value.get("id").cloned().unwrap_or(Value::Null);
            write_rpc(stdin, &acp_permission_deny(&request_id))?;
            continue;
        }
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
        if value.get("id") == Some(&json!(3)) {
            if value.get("error").is_some() {
                return Err(format!("Hermes session/prompt error: {}", value["error"]));
            }
            break;
        }
    }
    Ok(reply)
}

#[path = "hermes_acp_tests.rs"]
#[cfg(test)]
mod hermes_acp_tests;
