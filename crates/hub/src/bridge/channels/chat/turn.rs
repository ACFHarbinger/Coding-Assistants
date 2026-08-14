//! Speaks the documented `codex app-server` JSON-RPC protocol over a
//! disposable child process's stdio to deliver one turn and capture its
//! reply.
//!
//! The protocol shape here (`turn/start` returns an in-progress `Turn`
//! immediately; the actual reply text arrives later as an unsolicited
//! `turn/completed` notification carrying the turn's final `items`, one of
//! which may be an `agentMessage` item with the reply text) was confirmed
//! against the real server, not assumed: `codex app-server generate-ts`
//! emits authoritative bindings for `ServerNotification`/`Turn`/`ThreadItem`
//! (checked live 2026-08-14, codex-cli 0.147.0), and a live `--listen
//! stdio://` handshake was driven by hand to see the actual wire shape.
//! `turn/start`'s own JSON-RPC result only ever carried the *initial*
//! (not-yet-completed) `Turn`, which is why the previous implementation —
//! reading only that result before killing the child — never captured any
//! reply text at all.
//!
//! A genuinely persistent per-thread broker (`codex app-server daemon`,
//! reused instead of spawn-per-turn) is documented and running on this
//! machine, but its control socket did not answer plain newline-delimited
//! JSON-RPC the way `--listen stdio://` does — spawning `codex app-server
//! proxy` in front of it produced no response either within a live probe.
//! Reverse-engineering that framing is out of scope here: this module still
//! spawns one `app-server` child per turn, but now blocks (bounded by a
//! generous line budget, matching this file's existing no-wall-clock-timeout
//! style) until `turn/completed` arrives before killing it, so a reply is
//! actually captured. #149's "long-lived streaming broker" remains a
//! genuine follow-up for continuous/streaming delivery, not for reply
//! capture itself.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Outcome of one delivered turn: the app-server-assigned turn id (used only
/// for the human-readable delivery detail message) and, when `turn/completed`
/// arrived within the read budget, the concatenated text of any `agentMessage`
/// items — Codex's actual reply.
pub struct CodexTurnOutcome {
    pub turn_id: String,
    pub reply_text: Option<String>,
}

pub fn send_codex_turn(thread_id: &str, body: &str) -> Result<CodexTurnOutcome, String> {
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
        let reply_text = read_turn_completed(&mut stdout, thread_id);
        Ok(CodexTurnOutcome {
            turn_id,
            reply_text,
        })
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

/// Reads unsolicited server notifications (interleaved `item/*` streaming
/// deltas, `turn/started`, etc.) looking for `turn/completed` on this
/// thread, then extracts the reply text from its `agentMessage` items.
///
/// Not receiving `turn/completed` within the read budget is not treated as
/// a delivery failure — `turn/start` already acked the turn, so the task
/// was genuinely delivered; a long-running turn simply produces no reply
/// text to route back into the Hub yet.
fn read_turn_completed(stdout: &mut impl BufRead, thread_id: &str) -> Option<String> {
    for line in stdout.lines().take(20_000) {
        let line = line.ok()?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("method").and_then(Value::as_str) != Some("turn/completed") {
            continue;
        }
        let params = value.get("params")?;
        if params.get("threadId").and_then(Value::as_str) != Some(thread_id) {
            continue;
        }
        let items = params
            .get("turn")
            .and_then(|turn| turn.get("items"))
            .and_then(Value::as_array)?;
        let text = items
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("agentMessage"))
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        return if text.trim().is_empty() {
            None
        } else {
            Some(text)
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn turn_completed_line(thread_id: &str, text: &str) -> String {
        json!({
            "method": "turn/completed",
            "params": {
                "threadId": thread_id,
                "turn": {
                    "id": "turn-1",
                    "items": [
                        { "type": "reasoning", "id": "r1", "summary": [], "content": [] },
                        { "type": "agentMessage", "id": "a1", "text": text, "phase": null, "memoryCitation": null }
                    ],
                    "itemsView": "full",
                    "status": "completed",
                    "error": null,
                    "startedAt": 0,
                    "completedAt": 1,
                    "durationMs": 1000
                }
            }
        })
        .to_string()
    }

    #[test]
    fn extracts_agent_message_text_from_matching_turn_completed() {
        let stream = format!(
            "{}\n{}\n",
            json!({"method":"item/agentMessage/delta","params":{"threadId":"thread-x","turnId":"turn-1","itemId":"a1","delta":"partial"}}),
            turn_completed_line("thread-x", "final reply text")
        );
        let mut reader = Cursor::new(stream);
        assert_eq!(
            read_turn_completed(&mut reader, "thread-x"),
            Some("final reply text".into())
        );
    }

    #[test]
    fn ignores_turn_completed_for_a_different_thread() {
        let stream = format!("{}\n", turn_completed_line("other-thread", "not this one"));
        let mut reader = Cursor::new(stream);
        assert_eq!(read_turn_completed(&mut reader, "thread-x"), None);
    }

    #[test]
    fn returns_none_when_the_stream_ends_without_turn_completed() {
        let mut reader = Cursor::new(String::new());
        assert_eq!(read_turn_completed(&mut reader, "thread-x"), None);
    }
}
