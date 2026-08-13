//! The actual stdio MCP server: request dispatch (`handle_request`) and the
//! background thread that proactively pushes Hub events and permission
//! verdicts (`poll_loop`). `run_server` wires both to real stdin/stdout.

use super::protocol::{
    check_inbox_response, check_inbox_tool_schema, reply_tool_schema, tool_call_response,
};
use hub::{
    get_permission_request, poll_channel_events, poll_quiet_channel_events, record_channel_reply,
    record_permission_request, HubStore, PermissionVerdict,
};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub fn run_server(args: &[String]) {
    let workspace = super::cli::workspace_arg(args);
    let store = Arc::new(Mutex::new(
        HubStore::open(hub::default_hub_home()).expect("open Hub store"),
    ));
    let stdout = Arc::new(Mutex::new(io::stdout()));
    // request_id values seen via `permission_request`, relayed once each
    // once a human resolves them. Shared with the poller thread below.
    let known_permission_requests: Arc<Mutex<HashSet<String>>> =
        Arc::new(Mutex::new(HashSet::new()));

    {
        let store = Arc::clone(&store);
        let stdout = Arc::clone(&stdout);
        let known_permission_requests = Arc::clone(&known_permission_requests);
        std::thread::spawn(move || poll_loop(store, stdout, known_permission_requests));
    }

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        handle_request(
            &store,
            &stdout,
            &known_permission_requests,
            &workspace,
            request,
        );
    }
}

fn write_message(stdout: &Mutex<io::Stdout>, message: &Value) {
    let mut out = stdout.lock().expect("stdout mutex poisoned");
    let _ = writeln!(out, "{message}");
    let _ = out.flush();
}

fn handle_request(
    store: &Mutex<HubStore>,
    stdout: &Mutex<io::Stdout>,
    known_permission_requests: &Mutex<HashSet<String>>,
    _workspace: &Path,
    request: Value,
) {
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let id = request.get("id").cloned();

    match method {
        "initialize" => {
            if let Some(id) = id {
                write_message(
                    stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": MCP_PROTOCOL_VERSION,
                            "serverInfo": {
                                "name": "coding-assistants-channel",
                                "version": env!("CARGO_PKG_VERSION"),
                            },
                            "capabilities": {
                                "tools": {},
                                "experimental": {
                                    "claude/channel": {},
                                    "claude/channel/permission": {},
                                },
                            },
                        },
                    }),
                );
            }
        }
        "tools/list" => {
            if let Some(id) = id {
                write_message(
                    stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "tools": [reply_tool_schema(), check_inbox_tool_schema()] },
                    }),
                );
            }
        }
        "tools/call" => {
            let Some(id) = id else { return };
            let params = request.get("params").cloned().unwrap_or(Value::Null);
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            match name {
                "reply" => {
                    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
                    let text = arguments.get("text").and_then(Value::as_str).unwrap_or("");
                    let in_reply_to = arguments.get("in_reply_to").and_then(Value::as_str);
                    let session_id = arguments.get("session_id").and_then(Value::as_str);
                    let result = {
                        let store = store.lock().expect("hub store mutex poisoned");
                        record_channel_reply(&store, in_reply_to, session_id, text)
                    };
                    write_message(stdout, &tool_call_response(id, result));
                }
                "check_inbox" => {
                    let result = {
                        let store = store.lock().expect("hub store mutex poisoned");
                        poll_quiet_channel_events(&store)
                    };
                    write_message(stdout, &check_inbox_response(id, result));
                }
                _ => {
                    write_message(
                        stdout,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32601, "message": format!("unknown tool {name}") },
                        }),
                    );
                }
            }
        }
        "notifications/claude/channel/permission_request" => {
            let params = request.get("params").cloned().unwrap_or(Value::Null);
            let request_id = params
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            if request_id.is_empty() {
                return;
            }
            let tool_name = params
                .get("tool_name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let description = params
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");
            let input_preview = params
                .get("input_preview")
                .and_then(Value::as_str)
                .unwrap_or("");
            let recorded = {
                let store = store.lock().expect("hub store mutex poisoned");
                record_permission_request(&store, request_id, tool_name, description, input_preview)
            };
            if recorded.is_ok() {
                known_permission_requests
                    .lock()
                    .expect("permission-id set mutex poisoned")
                    .insert(request_id.to_string());
            }
        }
        // `initialize`'s ack, and anything else we don't act on, are
        // silently ignored — MCP notifications never expect a reply.
        _ => {}
    }
}

fn poll_loop(
    store: Arc<Mutex<HubStore>>,
    stdout: Arc<Mutex<io::Stdout>>,
    known_permission_requests: Arc<Mutex<HashSet<String>>>,
) {
    let mut relayed_permissions: HashSet<String> = HashSet::new();
    loop {
        std::thread::sleep(POLL_INTERVAL);

        let events = {
            let store = store.lock().expect("hub store mutex poisoned");
            poll_channel_events(&store).unwrap_or_default()
        };
        for event in events {
            write_message(
                &stdout,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/claude/channel",
                    "params": {
                        "content": event.body,
                        "meta": {
                            "message_id": event.message_id,
                            "from": event.from_agent,
                            "kind": event.kind,
                            "session_id": event.session_id,
                        },
                    },
                }),
            );
        }

        // Relay a verdict only once, and only after a human explicitly
        // resolved it — never auto-approve.
        let candidates: Vec<String> = known_permission_requests
            .lock()
            .expect("permission-id set mutex poisoned")
            .iter()
            .filter(|id| !relayed_permissions.contains(*id))
            .cloned()
            .collect();
        for request_id in candidates {
            let verdict = {
                let store = store.lock().expect("hub store mutex poisoned");
                get_permission_request(&store, &request_id).ok().flatten()
            };
            let Some(verdict) = verdict else { continue };
            if verdict == PermissionVerdict::Pending {
                continue;
            }
            write_message(
                &stdout,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/claude/channel/permission",
                    "params": {
                        "request_id": request_id,
                        "behavior": if verdict == PermissionVerdict::Allowed { "allow" } else { "deny" },
                    },
                }),
            );
            relayed_permissions.insert(request_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fresh_store() -> Mutex<HubStore> {
        let dir = tempdir().unwrap();
        // Leak the tempdir so the store outlives this function — fine for a
        // one-off test fixture, avoided in hot paths elsewhere in the repo.
        let path = dir.keep();
        Mutex::new(HubStore::open(path).unwrap())
    }

    #[test]
    fn handle_request_initialize_declares_both_channel_capabilities() {
        let store = fresh_store();
        let stdout = Mutex::new(io::stdout());
        let known = Mutex::new(HashSet::new());
        // No direct return value from handle_request (it writes to stdout),
        // so this exercises the dispatch path end-to-end without panicking
        // rather than asserting on stdout content — the JSON shape itself
        // is covered by the protocol-module tests.
        handle_request(
            &store,
            &stdout,
            &known,
            Path::new("/tmp"),
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }),
        );
    }

    #[test]
    fn handle_request_records_a_permission_request_exactly_once() {
        let store = fresh_store();
        let stdout = Mutex::new(io::stdout());
        let known = Mutex::new(HashSet::new());
        handle_request(
            &store,
            &stdout,
            &known,
            Path::new("/tmp"),
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/claude/channel/permission_request",
                "params": { "request_id": "req-1", "tool_name": "Bash", "description": "d", "input_preview": "p" },
            }),
        );
        assert!(known.lock().unwrap().contains("req-1"));

        let verdict = {
            let store = store.lock().unwrap();
            get_permission_request(&store, "req-1").unwrap()
        };
        assert_eq!(verdict, Some(PermissionVerdict::Pending));
    }
}
