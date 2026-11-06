//! P14 slice A — Coding Assistants as an MCP **client**.
//!
//! `hub::mcp` previously only *rendered config* so other CLIs could spawn
//! servers. This module performs `initialize` / `tools/list` / `tools/call`
//! itself over stdio JSON-RPC, driving a [`McpServerEntry`] (command + args)
//! that already exists in the external / creative registries — no new
//! config surface.
//!
//! Framing writes the Content-Length form the TypeScript MCP SDK expects
//! (Perplexity) and appends a trailing newline so `mcp-core`'s NDJSON
//! `read_line` loop still sees the JSON body. Reads accept either framing.
//!
//! Each call is a fresh spawn (invoke-and-show). Timeouts kill the child.

use super::McpServerEntry;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const INIT_TIMEOUT: Duration = Duration::from_secs(30);
const CALL_TIMEOUT: Duration = Duration::from_secs(45);
const CLIENT_NAME: &str = "coding-assistants";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL: &str = "2024-11-05";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpCallResult {
    pub is_error: bool,
    pub text: String,
}

pub fn list_tools(
    entry: &McpServerEntry,
    extra_env: &[(&str, &str)],
) -> Result<Vec<McpTool>, String> {
    let result = roundtrip(
        entry,
        extra_env,
        INIT_TIMEOUT,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
        2,
    )?;
    parse_tools(result.get("result").unwrap_or(&Value::Null))
}

pub fn call_tool(
    entry: &McpServerEntry,
    extra_env: &[(&str, &str)],
    name: &str,
    arguments: &Value,
) -> Result<McpCallResult, String> {
    let result = roundtrip(
        entry,
        extra_env,
        CALL_TIMEOUT,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        }),
        2,
    )?;
    if let Some(error) = result.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("MCP tools/call failed");
        return Err(message.to_string());
    }
    Ok(call_result_from(
        result.get("result").unwrap_or(&Value::Null),
    ))
}

fn parse_tools(result: &Value) -> Result<Vec<McpTool>, String> {
    let tools = result
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| "MCP tools/list returned no tools array".to_string())?;
    Ok(tools
        .iter()
        .filter_map(|tool| {
            let name = tool.get("name").and_then(Value::as_str)?.to_string();
            let description = tool
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string);
            let input_schema = tool
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));
            Some(McpTool {
                name,
                description,
                input_schema,
            })
        })
        .collect())
}

pub(crate) fn call_result_from(result: &Value) -> McpCallResult {
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let text = result
        .get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| {
                    let ty = part.get("type").and_then(Value::as_str).unwrap_or("text");
                    (ty == "text")
                        .then(|| part.get("text").and_then(Value::as_str))
                        .flatten()
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| result.to_string());
    McpCallResult { is_error, text }
}

fn write_message(out: &mut impl Write, value: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(value).map_err(|e| format!("serialize MCP message: {e}"))?;
    write!(out, "Content-Length: {}\r\n\r\n", body.len())
        .and_then(|_| out.write_all(&body))
        .and_then(|_| out.write_all(b"\n"))
        .and_then(|_| out.flush())
        .map_err(|e| format!("write MCP message: {e}"))
}

pub(crate) fn read_message(reader: &mut impl BufRead) -> Result<Value, String> {
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| format!("read MCP stream: {e}"))?;
        if n == 0 {
            return Err("MCP server closed stdout before answering".into());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            let len: usize = rest
                .trim()
                .parse()
                .map_err(|_| format!("invalid Content-Length {rest:?}"))?;
            loop {
                line.clear();
                reader
                    .read_line(&mut line)
                    .map_err(|e| format!("read MCP headers: {e}"))?;
                if line.trim().is_empty() {
                    break;
                }
            }
            let mut buf = vec![0u8; len];
            std::io::Read::read_exact(reader, &mut buf)
                .map_err(|e| format!("read MCP body: {e}"))?;
            return serde_json::from_slice(&buf).map_err(|e| format!("parse MCP body: {e}"));
        }
        if trimmed.starts_with('{') {
            return serde_json::from_str(trimmed).map_err(|e| format!("parse MCP line: {e}"));
        }
    }
}

fn spawn(entry: &McpServerEntry, extra_env: &[(&str, &str)]) -> Result<Child, String> {
    if entry.command.trim().is_empty() {
        return Err("MCP server command is empty".into());
    }
    let mut cmd = Command::new(&entry.command);
    cmd.args(&entry.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    cmd.spawn()
        .map_err(|e| format!("failed to spawn `{}`: {e}", entry.command))
}

fn roundtrip(
    entry: &McpServerEntry,
    extra_env: &[(&str, &str)],
    timeout: Duration,
    request: Value,
    expect_id: i64,
) -> Result<Value, String> {
    let mut child = spawn(entry, extra_env)?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "MCP server stdin unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MCP server stdout unavailable".to_string())?;

    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOL,
            "capabilities": {},
            "clientInfo": { "name": CLIENT_NAME, "version": CLIENT_VERSION }
        }
    });
    write_message(&mut stdin, &init)?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let outcome = (|| {
            wait_for_id(&mut reader, 1)?;
            Ok(reader)
        })();
        let _ = tx.send(outcome);
    });
    let mut reader = match rx.recv_timeout(timeout) {
        Ok(Ok(reader)) => reader,
        Ok(Err(err)) => {
            let _ = child.kill();
            return Err(err);
        }
        Err(_) => {
            let _ = child.kill();
            return Err(format!(
                "`{}` did not complete MCP initialize within {}s",
                entry.command,
                timeout.as_secs()
            ));
        }
    };

    write_message(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
    )?;
    write_message(&mut stdin, &request)?;
    drop(stdin);

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(wait_for_id(&mut reader, expect_id));
    });
    let response = match rx.recv_timeout(timeout) {
        Ok(Ok(value)) => value,
        Ok(Err(err)) => {
            let _ = child.kill();
            return Err(err);
        }
        Err(_) => {
            let _ = child.kill();
            return Err(format!(
                "`{}` did not answer within {}s",
                entry.command,
                timeout.as_secs()
            ));
        }
    };
    let _ = child.kill();
    Ok(response)
}

fn wait_for_id(reader: &mut impl BufRead, id: i64) -> Result<Value, String> {
    for _ in 0..200 {
        let value = read_message(reader)?;
        if value.get("id") == Some(&Value::from(id)) {
            if let Some(error) = value.get("error") {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("MCP request failed");
                return Err(message.to_string());
            }
            return Ok(value);
        }
    }
    Err("MCP server sent no matching response".into())
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
