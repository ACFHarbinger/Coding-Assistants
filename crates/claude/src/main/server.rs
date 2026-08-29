//! The Channel stdio MCP server, built on `mcp-core`.
//!
//! `mcp-core::McpServer` owns the JSON-RPC loop, `initialize` / `tools/list`
//! / `tools/call` dispatch, and the notification emitter. This module
//! supplies the Channel-specific behaviour as a [`ChannelProvider`]:
//! - the `reply` / `check_inbox` tools (`ToolProvider::call`),
//! - the two `experimental` capabilities (`extra_capabilities`),
//! - recording an inbound `permission_request` notification
//!   (`on_notification`),
//! - and a background [`poll_loop`] that proactively pushes Hub events and
//!   human-resolved permission verdicts through an [`mcp_core::Emitter`].

use super::protocol::{
    check_inbox_outcome, check_inbox_tool_schema, reply_outcome, reply_tool_schema,
};
use hub::{
    get_permission_request, poll_channel_events, poll_quiet_channel_events, record_channel_reply,
    record_permission_request, HubStore, PermissionVerdict,
};
use mcp_core::{Emitter, McpServer, ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

struct ChannelProvider {
    store: Arc<Mutex<HubStore>>,
    /// `request_id`s seen via `permission_request`, shared with [`poll_loop`]
    /// which relays each verdict once a human resolves it.
    known_permission_requests: Arc<Mutex<HashSet<String>>>,
}

impl ToolProvider for ChannelProvider {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-channel".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        vec![reply_tool_schema(), check_inbox_tool_schema()]
    }

    fn extra_capabilities(&self) -> Value {
        json!({
            "experimental": {
                "claude/channel": {},
                "claude/channel/permission": {},
            },
        })
    }

    fn call(&self, name: &str, arguments: &Value) -> ToolResult {
        match name {
            "reply" => {
                let text = arguments.get("text").and_then(Value::as_str).unwrap_or("");
                let in_reply_to = arguments.get("in_reply_to").and_then(Value::as_str);
                let session_id = arguments.get("session_id").and_then(Value::as_str);
                let store = self.store.lock().expect("hub store mutex poisoned");
                reply_outcome(record_channel_reply(&store, in_reply_to, session_id, text))
            }
            "check_inbox" => {
                let store = self.store.lock().expect("hub store mutex poisoned");
                check_inbox_outcome(poll_quiet_channel_events(&store))
            }
            other => ToolResult::Err(format!("unknown tool {other}")),
        }
    }

    fn on_notification(&self, method: &str, params: &Value, _emitter: &Emitter) {
        if method != "notifications/claude/channel/permission_request" {
            return;
        }
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
            let store = self.store.lock().expect("hub store mutex poisoned");
            record_permission_request(&store, request_id, tool_name, description, input_preview)
        };
        if recorded.is_ok() {
            self.known_permission_requests
                .lock()
                .expect("permission-id set mutex poisoned")
                .insert(request_id.to_string());
        }
    }
}

pub fn run_server(args: &[String]) {
    // `--workspace` is still accepted for CLI parity; the running server
    // does not need it (the Hub store is global).
    let _workspace = super::cli::workspace_arg(args);

    let store = Arc::new(Mutex::new(
        HubStore::open(hub::default_hub_home()).expect("open Hub store"),
    ));
    let known_permission_requests: Arc<Mutex<HashSet<String>>> =
        Arc::new(Mutex::new(HashSet::new()));

    let provider = Arc::new(ChannelProvider {
        store: Arc::clone(&store),
        known_permission_requests: Arc::clone(&known_permission_requests),
    });
    let server = McpServer::new(provider);

    {
        let emitter = server.emitter();
        std::thread::spawn(move || poll_loop(store, known_permission_requests, emitter));
    }

    server.run();
}

fn poll_loop(
    store: Arc<Mutex<HubStore>>,
    known_permission_requests: Arc<Mutex<HashSet<String>>>,
    emitter: Emitter,
) {
    let mut relayed_permissions: HashSet<String> = HashSet::new();
    loop {
        std::thread::sleep(POLL_INTERVAL);

        let events = {
            let store = store.lock().expect("hub store mutex poisoned");
            poll_channel_events(&store).unwrap_or_default()
        };
        for event in events {
            emitter.notify(
                "notifications/claude/channel",
                json!({
                    "content": event.body,
                    "meta": {
                        "message_id": event.message_id,
                        "from": event.from_agent,
                        "kind": event.kind,
                        "session_id": event.session_id,
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
            emitter.notify(
                "notifications/claude/channel/permission",
                json!({
                    "request_id": request_id,
                    "behavior": if verdict == PermissionVerdict::Allowed { "allow" } else { "deny" },
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

    fn provider_with_fresh_store() -> ChannelProvider {
        let dir = tempdir().unwrap();
        let path = dir.keep();
        ChannelProvider {
            store: Arc::new(Mutex::new(HubStore::open(path).unwrap())),
            known_permission_requests: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    #[test]
    fn initialize_declares_both_channel_capabilities() {
        let provider = provider_with_fresh_store();
        let caps = provider.extra_capabilities();
        assert_eq!(caps["experimental"]["claude/channel"], json!({}));
        assert_eq!(caps["experimental"]["claude/channel/permission"], json!({}));
    }

    #[test]
    fn tools_list_exposes_reply_and_check_inbox() {
        let provider = provider_with_fresh_store();
        let names: Vec<String> = provider
            .tools()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(names, ["reply", "check_inbox"]);
    }

    #[test]
    fn records_a_permission_request_exactly_once() {
        let provider = provider_with_fresh_store();
        // A throwaway server just to mint an Emitter for the handler.
        let scratch = McpServer::with_stdout(
            Arc::new(provider_with_fresh_store()),
            Box::new(std::io::sink()),
        );
        let emitter = scratch.emitter();

        provider.on_notification(
            "notifications/claude/channel/permission_request",
            &json!({ "request_id": "req-1", "tool_name": "Bash", "description": "d", "input_preview": "p" }),
            &emitter,
        );
        assert!(provider
            .known_permission_requests
            .lock()
            .unwrap()
            .contains("req-1"));

        let verdict = {
            let store = provider.store.lock().unwrap();
            get_permission_request(&store, "req-1").unwrap()
        };
        assert_eq!(verdict, Some(PermissionVerdict::Pending));
    }

    #[test]
    fn unknown_tool_name_is_a_tool_error() {
        let provider = provider_with_fresh_store();
        match provider.call("nope", &json!({})) {
            ToolResult::Err(msg) => assert!(msg.contains("nope")),
            ToolResult::Ok(_) => panic!("expected an error"),
        }
    }
}
