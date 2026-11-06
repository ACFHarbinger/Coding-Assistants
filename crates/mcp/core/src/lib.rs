//! Reusable stdio MCP server core.
//!
//! Extracted from `crates/claude`'s hand-rolled Channel bridge so every
//! Coding-Assistants tool bridge (`crates/mcp-<tool>`) shares one
//! line-framed JSON-RPC loop, one `initialize` / `tools/list` / `tools/call`
//! dispatch, and one notification emitter. A bridge implements
//! [`ToolProvider`] — value-in, value-out, no I/O — and calls
//! [`McpServer::run`], which owns stdin/stdout.
//!
//! Transport: newline-delimited JSON objects on stdio, matching what
//! Claude Code / Codex / other MCP clients speak to a `command` server
//! entry in their config. One JSON value per line, `\n`-terminated,
//! flushed after every write.
//!
//! What stays in the bridge crate: anything client-specific — extra
//! `capabilities` (e.g. Claude's `experimental["claude/channel"]`), any
//! proactive push loop (spawn a thread, hand it an [`Emitter`]), and any
//! non-standard notification method (handled in
//! [`ToolProvider::on_notification`]).

pub mod app_link;
mod memory_tools;

pub use memory_tools::{MemoryProvider, MemoryTools};

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

/// The MCP protocol revision this core implements. Bump deliberately when a
/// client requires a newer one; the value is echoed back in `initialize`.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Server identity reported in the `initialize` result. `version` is the
/// *bridge crate's* version (`env!("CARGO_PKG_VERSION")` at its call site),
/// not `mcp-core`'s.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// The result of a `tools/call`. `Ok` and `Err` both produce a normal
/// JSON-RPC *result* with a `content` array — `Err` just sets
/// `isError: true`, per the MCP tool-error convention (a JSON-RPC `error`
/// is reserved for protocol faults like an unknown tool).
pub enum ToolResult {
    Ok(String),
    Err(String),
}

impl ToolResult {
    fn into_result_body(self) -> Value {
        match self {
            ToolResult::Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
            ToolResult::Err(text) => {
                json!({ "content": [{ "type": "text", "text": text }], "isError": true })
            }
        }
    }
}

/// A bridge crate implements this. Every method is pure except `call`,
/// which may touch the bridged application — but it still returns a value
/// rather than writing to the wire, so it is unit-testable directly.
pub trait ToolProvider: Send + Sync + 'static {
    fn server_info(&self) -> ServerInfo;

    /// Tool schema objects (`{ "name", "description", "inputSchema" }`),
    /// returned verbatim in `tools/list`.
    fn tools(&self) -> Vec<Value>;

    /// Dispatch a `tools/call`. `name` is guaranteed to be one of
    /// [`ToolProvider::tools`]'s names *only if* the client behaves;
    /// return [`ToolResult::Err`] for an unrecognised name you still want
    /// surfaced as a tool error rather than a protocol error.
    fn call(&self, name: &str, arguments: &Value) -> ToolResult;

    /// Extra top-level entries merged into the `initialize` result's
    /// `capabilities` object (on top of the always-present `"tools": {}`).
    /// Default: nothing. Claude's Channel bridge overrides this to add
    /// `experimental`.
    fn extra_capabilities(&self) -> Value {
        json!({})
    }

    /// Called for any inbound method `mcp-core` does not handle itself
    /// (everything except `initialize` / `tools/list` / `tools/call`).
    /// Use it for client-specific notifications. `emitter` is available
    /// for sending a follow-up notification. Default: ignore.
    fn on_notification(&self, _method: &str, _params: &Value, _emitter: &Emitter) {}
}

/// A cloneable handle for writing JSON-RPC notifications to the client out
/// of band — hand one to a background push thread before calling
/// [`McpServer::run`].
#[derive(Clone)]
pub struct Emitter {
    stdout: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl Emitter {
    fn write_value(&self, message: &Value) {
        let mut out = self.stdout.lock().expect("mcp-core stdout mutex poisoned");
        let _ = writeln!(out, "{message}");
        let _ = out.flush();
    }

    /// Send a JSON-RPC notification (no `id`, no response expected).
    pub fn notify(&self, method: &str, params: Value) {
        self.write_value(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }));
    }

    fn respond(&self, id: Value, result: Value) {
        self.write_value(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    }

    fn respond_error(&self, id: Value, code: i64, message: String) {
        self.write_value(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message },
        }));
    }
}

/// Owns stdio for the lifetime of the server. Construct with a provider,
/// optionally take an [`McpServer::emitter`] for a push thread, then
/// [`McpServer::run`] the blocking stdin loop.
pub struct McpServer<P: ToolProvider> {
    provider: Arc<P>,
    emitter: Emitter,
}

impl<P: ToolProvider> McpServer<P> {
    pub fn new(provider: Arc<P>) -> Self {
        Self::with_stdout(provider, Box::new(io::stdout()))
    }

    /// Testing seam: swap stdout for an in-memory buffer.
    pub fn with_stdout(provider: Arc<P>, stdout: Box<dyn Write + Send>) -> Self {
        Self {
            provider,
            emitter: Emitter {
                stdout: Arc::new(Mutex::new(stdout)),
            },
        }
    }

    /// Clone this before `run()` and give it to a background thread that
    /// needs to push notifications.
    pub fn emitter(&self) -> Emitter {
        self.emitter.clone()
    }

    /// Blocking: read newline-delimited JSON-RPC from stdin until EOF,
    /// dispatching each message.
    pub fn run(&self) {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let Ok(request) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            self.dispatch(&request);
        }
    }

    /// One message. Public for direct testing without a stdin pipe.
    pub fn dispatch(&self, request: &Value) {
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let id = request.get("id").cloned();
        let params = request.get("params").cloned().unwrap_or(Value::Null);

        match method {
            "initialize" => {
                let Some(id) = id else { return };
                let info = self.provider.server_info();
                let mut capabilities = json!({ "tools": {} });
                if let (Some(cap), Some(extra)) = (
                    capabilities.as_object_mut(),
                    self.provider.extra_capabilities().as_object(),
                ) {
                    for (k, v) in extra {
                        cap.insert(k.clone(), v.clone());
                    }
                }
                self.emitter.respond(
                    id,
                    json!({
                        "protocolVersion": MCP_PROTOCOL_VERSION,
                        "serverInfo": { "name": info.name, "version": info.version },
                        "capabilities": capabilities,
                    }),
                );
            }
            "tools/list" => {
                let Some(id) = id else { return };
                self.emitter
                    .respond(id, json!({ "tools": self.provider.tools() }));
            }
            "tools/call" => {
                let Some(id) = id else { return };
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
                if name.is_empty() {
                    self.emitter.respond_error(
                        id,
                        -32602,
                        "tools/call requires a params.name".to_string(),
                    );
                    return;
                }
                if !self
                    .provider
                    .tools()
                    .iter()
                    .any(|t| t.get("name").and_then(Value::as_str) == Some(name))
                {
                    self.emitter
                        .respond_error(id, -32601, format!("unknown tool {name}"));
                    return;
                }
                let outcome = self.provider.call(name, &arguments);
                self.emitter.respond(id, outcome.into_result_body());
            }
            _ => self
                .provider
                .on_notification(method, &params, &self.emitter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// Collects everything the server writes.
    #[derive(Clone, Default)]
    struct Sink(Arc<StdMutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Sink {
        fn lines(&self) -> Vec<Value> {
            String::from_utf8(self.0.lock().unwrap().clone())
                .unwrap()
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| serde_json::from_str(l).unwrap())
                .collect()
        }
    }

    struct Echo;
    impl ToolProvider for Echo {
        fn server_info(&self) -> ServerInfo {
            ServerInfo {
                name: "echo".into(),
                version: "9.9.9".into(),
            }
        }
        fn tools(&self) -> Vec<Value> {
            vec![json!({
                "name": "echo",
                "description": "Echo the text back.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "text": { "type": "string" } },
                    "required": ["text"],
                },
            })]
        }
        fn call(&self, name: &str, arguments: &Value) -> ToolResult {
            match name {
                "echo" => match arguments.get("text").and_then(Value::as_str) {
                    Some(t) => ToolResult::Ok(t.to_string()),
                    None => ToolResult::Err("text is required".into()),
                },
                other => ToolResult::Err(format!("no such tool {other}")),
            }
        }
    }

    fn server_with_sink() -> (McpServer<Echo>, Sink) {
        let sink = Sink::default();
        (
            McpServer::with_stdout(Arc::new(Echo), Box::new(sink.clone())),
            sink,
        )
    }

    #[test]
    fn initialize_echoes_protocol_version_and_provider_identity() {
        let (server, sink) = server_with_sink();
        server.dispatch(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }));
        let out = sink.lines();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(out[0]["result"]["serverInfo"]["name"], "echo");
        assert_eq!(out[0]["result"]["serverInfo"]["version"], "9.9.9");
        assert_eq!(out[0]["result"]["capabilities"]["tools"], json!({}));
    }

    #[test]
    fn extra_capabilities_are_merged_not_replaced() {
        struct WithExperimental;
        impl ToolProvider for WithExperimental {
            fn server_info(&self) -> ServerInfo {
                ServerInfo {
                    name: "x".into(),
                    version: "1".into(),
                }
            }
            fn tools(&self) -> Vec<Value> {
                vec![]
            }
            fn call(&self, _: &str, _: &Value) -> ToolResult {
                ToolResult::Err("n/a".into())
            }
            fn extra_capabilities(&self) -> Value {
                json!({ "experimental": { "claude/channel": {} } })
            }
        }
        let sink = Sink::default();
        let server = McpServer::with_stdout(Arc::new(WithExperimental), Box::new(sink.clone()));
        server.dispatch(&json!({ "jsonrpc": "2.0", "id": 7, "method": "initialize" }));
        let caps = &sink.lines()[0]["result"]["capabilities"];
        assert_eq!(caps["tools"], json!({}));
        assert_eq!(caps["experimental"]["claude/channel"], json!({}));
    }

    #[test]
    fn tools_list_returns_provider_schemas() {
        let (server, sink) = server_with_sink();
        server.dispatch(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
        let tools = sink.lines()[0]["result"]["tools"].clone();
        assert_eq!(tools[0]["name"], "echo");
    }

    #[test]
    fn tools_call_dispatches_and_wraps_ok() {
        let (server, sink) = server_with_sink();
        server.dispatch(&json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "echo", "arguments": { "text": "hi" } },
        }));
        let r = &sink.lines()[0]["result"];
        assert_eq!(r["content"][0]["text"], "hi");
        assert_eq!(r["isError"], Value::Null);
    }

    #[test]
    fn tools_call_wraps_err_as_tool_error_not_protocol_error() {
        let (server, sink) = server_with_sink();
        server.dispatch(&json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": { "name": "echo", "arguments": {} },
        }));
        let out = &sink.lines()[0];
        assert!(out.get("error").is_none());
        assert_eq!(out["result"]["isError"], json!(true));
    }

    #[test]
    fn unknown_tool_is_a_protocol_error() {
        let (server, sink) = server_with_sink();
        server.dispatch(&json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": { "name": "nope", "arguments": {} },
        }));
        assert_eq!(sink.lines()[0]["error"]["code"], -32601);
    }

    #[test]
    fn notifications_reach_the_provider_and_never_get_a_response() {
        struct Spy(Arc<StdMutex<Vec<String>>>);
        impl ToolProvider for Spy {
            fn server_info(&self) -> ServerInfo {
                ServerInfo {
                    name: "spy".into(),
                    version: "1".into(),
                }
            }
            fn tools(&self) -> Vec<Value> {
                vec![]
            }
            fn call(&self, _: &str, _: &Value) -> ToolResult {
                ToolResult::Err("n/a".into())
            }
            fn on_notification(&self, method: &str, _params: &Value, emitter: &Emitter) {
                self.0.lock().unwrap().push(method.to_string());
                emitter.notify("notifications/echo", json!({ "seen": method }));
            }
        }
        let seen = Arc::new(StdMutex::new(vec![]));
        let sink = Sink::default();
        let server = McpServer::with_stdout(Arc::new(Spy(seen.clone())), Box::new(sink.clone()));
        server.dispatch(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            ["notifications/initialized"]
        );
        let out = sink.lines();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["method"], "notifications/echo");
        assert!(out[0].get("id").is_none());
    }

    #[test]
    fn emitter_is_cloneable_and_writes_line_framed_json() {
        let (server, sink) = server_with_sink();
        let emitter = server.emitter();
        emitter.notify("notifications/tick", json!({ "n": 1 }));
        emitter.notify("notifications/tick", json!({ "n": 2 }));
        let out = sink.lines();
        assert_eq!(out.len(), 2);
        assert_eq!(out[1]["params"]["n"], 2);
    }
}
