//! C14.3: opt-in Claude Code "Channel" MCP bridge.
//!
//! Claude Code spawns this binary as a stdio MCP server once the owner has
//! (1) run `--setup` for a workspace, which writes a `.mcp.json` entry via
//! [`hub::setup_claude_channel`], and (2) starts `claude --channels` in
//! that workspace so the documented, research-preview `claude/channel`
//! capability is active. Every event this process sends or receives
//! crosses the documented MCP `claude/channel`/`claude/channel/permission`
//! surface — it never touches Claude Code's undocumented internal
//! `cc-socks` control socket, and it never mutates
//! `hub::bridge::claude`'s C12 capture-only delivery path. A Claude
//! session that has not opted into a Channel keeps using that
//! capture-only path exactly as before.
//!
//! Protocol notes (from Anthropic's documented Channels research preview):
//! - `initialize` capability negotiation declares
//!   `capabilities.experimental["claude/channel"]` (push events in) and
//!   `["claude/channel/permission"]` (opt-in permission relay).
//! - Pushing an event into the session is an MCP *notification*
//!   (`notifications/claude/channel`, `{content, meta}`), not a response
//!   to any request — Claude Code does not poll this server, so a
//!   background thread here polls the Hub and pushes proactively.
//! - Claude replies through a normal MCP *tool* (`tools/call`); the tool
//!   itself is not special, only what the server's handler does with the
//!   call (here: route it back into the Hub) is bridge-specific.
//! - A permission dialog opening in the session arrives here as
//!   `notifications/claude/channel/permission_request`; the relayed
//!   verdict (`notifications/claude/channel/permission`) is sent only
//!   after a human resolves it via [`hub::resolve_permission_request`] —
//!   this process never decides on its own.

use hub::{
    get_permission_request, poll_channel_events, record_channel_reply, record_permission_request,
    setup_claude_channel, HubStore, PermissionVerdict,
};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const POLL_INTERVAL: Duration = Duration::from_secs(2);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--setup") {
        run_setup(&args[1..]);
    } else {
        run_server(&args);
    }
}

fn workspace_arg(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|pair| pair[0] == "--workspace")
        .map(|pair| PathBuf::from(&pair[1]))
        .unwrap_or_else(|| std::env::current_dir().expect("current directory"))
}

fn run_setup(args: &[String]) {
    let requested = workspace_arg(args);
    let workspace = requested
        .canonicalize()
        .unwrap_or_else(|_| requested.clone());
    let store = HubStore::open(hub::default_hub_home()).expect("open Hub store");
    let bridge_binary = std::env::current_exe().expect("resolve this binary's own path");

    let config = match setup_claude_channel(&store, &workspace, &bridge_binary) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Claude Channel setup failed: {error}");
            std::process::exit(1);
        }
    };

    let mcp_path = workspace.join(".mcp.json");
    let merged = merge_mcp_config(&mcp_path, &config);
    let rendered = serde_json::to_string_pretty(&merged).expect("serialize .mcp.json");
    if let Err(error) = std::fs::write(&mcp_path, rendered + "\n") {
        eprintln!("failed to write {}: {error}", mcp_path.display());
        std::process::exit(1);
    }

    println!("Claude Channel configured for {}.", workspace.display());
    println!("Wrote {}.", mcp_path.display());
    println!("Start Claude Code in this workspace with: claude --channels");
    println!(
        "(research preview; requires Claude Code 2.1.231+, Anthropic authentication, and \
         --dangerously-load-development-channels until this bridge is allowlisted)"
    );
}

/// Merges the new `mcpServers` entry into an existing `.mcp.json` without
/// discarding any other server the owner already configured there.
fn merge_mcp_config(path: &Path, addition: &Value) -> Value {
    let mut existing: Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({ "mcpServers": {} }));
    if !existing.get("mcpServers").is_some_and(Value::is_object) {
        existing["mcpServers"] = json!({});
    }
    if let (Some(target), Some(source)) = (
        existing["mcpServers"].as_object_mut(),
        addition["mcpServers"].as_object(),
    ) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    existing
}

fn run_server(args: &[String]) {
    let workspace = workspace_arg(args);
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
                    &json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": [reply_tool_schema()] } }),
                );
            }
        }
        "tools/call" => {
            let Some(id) = id else { return };
            let params = request.get("params").cloned().unwrap_or(Value::Null);
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            if name != "reply" {
                write_message(
                    stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": format!("unknown tool {name}") },
                    }),
                );
                return;
            }
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

fn reply_tool_schema() -> Value {
    json!({
        "name": "reply",
        "description": "Send a reply back to the Coding-Assistants Hub, routed to the original sender or session that reached this Channel.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The reply body." },
                "in_reply_to": { "type": "string", "description": "Hub message id this replies to, if known." },
                "session_id": { "type": "string", "description": "Hub session id to reply within, if known." },
            },
            "required": ["text"],
        },
    })
}

fn tool_call_response(id: Value, result: Result<hub::MessageRecord, hub::HubError>) -> Value {
    match result {
        Ok(message) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "content": [{ "type": "text", "text": format!("relayed to Hub as message {}", message.id) }] },
        }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "content": [{ "type": "text", "text": format!("failed to relay reply: {error}") }], "isError": true },
        }),
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

    #[test]
    fn workspace_arg_reads_the_flag() {
        let args = vec!["--workspace".to_string(), "/abs/repo".to_string()];
        assert_eq!(workspace_arg(&args), PathBuf::from("/abs/repo"));
    }

    #[test]
    fn workspace_arg_falls_back_to_cwd_when_absent() {
        let args: Vec<String> = vec![];
        assert_eq!(workspace_arg(&args), std::env::current_dir().unwrap());
    }

    #[test]
    fn merge_mcp_config_creates_a_fresh_file_when_none_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".mcp.json");
        let addition = json!({ "mcpServers": { "coding-assistants-channel": { "command": "x" } } });
        let merged = merge_mcp_config(&path, &addition);
        assert_eq!(
            merged["mcpServers"]["coding-assistants-channel"]["command"],
            "x"
        );
    }

    #[test]
    fn merge_mcp_config_preserves_other_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".mcp.json");
        std::fs::write(
            &path,
            json!({ "mcpServers": { "other-server": { "command": "y" } } }).to_string(),
        )
        .unwrap();
        let addition = json!({ "mcpServers": { "coding-assistants-channel": { "command": "x" } } });
        let merged = merge_mcp_config(&path, &addition);
        assert_eq!(merged["mcpServers"]["other-server"]["command"], "y");
        assert_eq!(
            merged["mcpServers"]["coding-assistants-channel"]["command"],
            "x"
        );
    }

    #[test]
    fn merge_mcp_config_overwrites_only_its_own_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".mcp.json");
        std::fs::write(
            &path,
            json!({ "mcpServers": { "coding-assistants-channel": { "command": "stale" } } })
                .to_string(),
        )
        .unwrap();
        let addition =
            json!({ "mcpServers": { "coding-assistants-channel": { "command": "fresh" } } });
        let merged = merge_mcp_config(&path, &addition);
        assert_eq!(
            merged["mcpServers"]["coding-assistants-channel"]["command"],
            "fresh"
        );
    }

    #[test]
    fn reply_tool_schema_requires_text_only() {
        let schema = reply_tool_schema();
        assert_eq!(schema["inputSchema"]["required"], json!(["text"]));
    }

    #[test]
    fn tool_call_response_reports_success_and_failure_distinctly() {
        let ok = tool_call_response(
            json!(1),
            Ok(hub::MessageRecord {
                id: "msg-1".into(),
                from_agent: "claude".into(),
                to_agent: "human".into(),
                workspace_path: None,
                task_id: None,
                kind: "message".into(),
                status: "pending".into(),
                subject: None,
                body: "hi".into(),
                created_at: "now".into(),
                acked_at: None,
            }),
        );
        assert_eq!(ok["result"]["isError"], Value::Null);

        let err = tool_call_response(json!(2), Err(hub::HubError::Invalid("bad".into())));
        assert_eq!(err["result"]["isError"], json!(true));
    }
}
