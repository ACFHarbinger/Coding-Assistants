//! MCP bridge to a running Blender instance.
//!
//! Transport contract (see `crates/mcp-core/CONTRACT.md`): the
//! `plugins/blender/` addon opens a line-JSON TCP server on
//! `127.0.0.1:<port>` (default [`DEFAULT_PORT`]). This crate is the
//! client: for each `tools/call`, connect, send one
//! `{ "op": <tool>, "args": {...} }` line, read one
//! `{ "ok": bool, "result"|"error": ... }` line, close.
//!
//! Per-call connections keep the bridge stateless and resilient to the
//! addon restarting; Blender operations are not latency-sensitive.

use mcp_core::{ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 9765;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Sends one line-JSON request to the addon and returns its `result`
/// payload, or an error string. Abstracted behind a trait so the provider
/// is unit-testable against a mock peer.
pub trait BlenderLink: Send + Sync {
    fn request(&self, op: &str, args: &Value) -> Result<Value, String>;
}

/// The real link: a fresh TCP connection to `127.0.0.1:<port>` per call.
pub struct TcpLink {
    pub port: u16,
}

impl BlenderLink for TcpLink {
    fn request(&self, op: &str, args: &Value) -> Result<Value, String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let stream = TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|e| format!("bad address {addr}: {e}"))?,
            CONNECT_TIMEOUT,
        )
        .map_err(|e| {
            format!(
                "could not reach the Blender addon on {addr}: {e}. \
                 Is Blender running with the Coding-Assistants addon enabled?"
            )
        })?;
        stream.set_read_timeout(Some(IO_TIMEOUT)).ok();
        stream.set_write_timeout(Some(IO_TIMEOUT)).ok();

        let mut writer = &stream;
        let payload = json!({ "op": op, "args": args });
        writeln!(writer, "{payload}").map_err(|e| format!("write to addon failed: {e}"))?;
        writer.flush().ok();

        let mut line = String::new();
        BufReader::new(&stream)
            .read_line(&mut line)
            .map_err(|e| format!("read from addon failed: {e}"))?;
        parse_response(&line)
    }
}

/// Interpret one response line from the addon.
pub fn parse_response(line: &str) -> Result<Value, String> {
    let line = line.trim();
    if line.is_empty() {
        return Err("empty response from the Blender addon".to_string());
    }
    let value: Value =
        serde_json::from_str(line).map_err(|e| format!("addon sent invalid JSON: {e}"))?;
    match value.get("ok").and_then(Value::as_bool) {
        Some(true) => Ok(value.get("result").cloned().unwrap_or(Value::Null)),
        Some(false) => Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unspecified Blender error")
            .to_string()),
        None => Err("addon response missing the `ok` field".to_string()),
    }
}

pub struct BlenderProvider<L: BlenderLink> {
    link: L,
    /// `run_python` is arbitrary code execution inside Blender. Off unless
    /// the server was started with `--allow-run-python` (or, later, enabled
    /// per workspace in Settings).
    allow_run_python: bool,
}

impl<L: BlenderLink> BlenderProvider<L> {
    pub fn new(link: L, allow_run_python: bool) -> Self {
        Self {
            link,
            allow_run_python,
        }
    }

    fn to_text(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
        }
    }
}

impl<L: BlenderLink + 'static> ToolProvider for BlenderProvider<L> {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-blender".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        let mut tools = vec![
            json!({
                "name": "get_scene_summary",
                "description": "Active scene overview: object count, active object, frame range, render engine, unit system.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "list_objects",
                "description": "Every object in the active scene as { name, type, location }.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "create_primitive",
                "description": "Add a mesh primitive to the active scene and return its object name.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["cube", "uv_sphere", "cylinder", "cone", "plane", "torus", "ico_sphere"] },
                        "location": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "World XYZ. Defaults to origin." },
                        "size": { "type": "number", "description": "Primitive size / radius. Defaults to 2.0." },
                        "name": { "type": "string", "description": "Optional name for the new object." }
                    },
                    "required": ["kind"],
                },
            }),
            json!({
                "name": "delete_object",
                "description": "Delete an object from the active scene by name.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                    "required": ["name"],
                },
            }),
            json!({
                "name": "export_scene",
                "description": "Export the active scene to a file. Format is inferred from the path extension (.glb/.gltf/.obj/.fbx/.stl).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute output path." },
                        "selection_only": { "type": "boolean", "description": "Export only selected objects. Default false." }
                    },
                    "required": ["path"],
                },
            }),
            json!({
                "name": "render_still",
                "description": "Render the current frame to an image file at the given absolute path.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"],
                },
            }),
        ];
        if self.allow_run_python {
            tools.push(json!({
                "name": "run_python",
                "description": "Execute a Python snippet inside Blender (full `bpy` access). Returns stdout plus the repr of a `result` variable if the snippet sets one.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "code": { "type": "string" } },
                    "required": ["code"],
                },
            }));
        }
        tools
    }

    fn call(&self, name: &str, arguments: &Value) -> ToolResult {
        if name == "run_python" && !self.allow_run_python {
            return ToolResult::Err(
                "run_python is disabled. Start the bridge with --allow-run-python to enable arbitrary Blender scripting."
                    .into(),
            );
        }
        match self.link.request(name, arguments) {
            Ok(result) => ToolResult::Ok(Self::to_text(&result)),
            Err(error) => ToolResult::Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type ReplyFn = dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync;

    struct MockLink {
        reply: Box<ReplyFn>,
    }
    impl MockLink {
        fn new(
            reply: impl Fn(&str, &Value) -> Result<Value, String> + Send + Sync + 'static,
        ) -> Self {
            Self {
                reply: Box::new(reply),
            }
        }
    }
    impl BlenderLink for MockLink {
        fn request(&self, op: &str, args: &Value) -> Result<Value, String> {
            (self.reply)(op, args)
        }
    }

    fn outcome(r: ToolResult) -> (String, bool) {
        match r {
            ToolResult::Ok(t) => (t, false),
            ToolResult::Err(t) => (t, true),
        }
    }

    #[test]
    fn parse_response_maps_ok_err_and_malformed() {
        assert_eq!(
            parse_response(r#"{"ok":true,"result":{"n":3}}"#).unwrap(),
            json!({ "n": 3 })
        );
        assert_eq!(
            parse_response(r#"{"ok":false,"error":"no active object"}"#).unwrap_err(),
            "no active object"
        );
        assert!(parse_response("").is_err());
        assert!(parse_response("not json").is_err());
        assert!(parse_response(r#"{"result":1}"#).is_err());
    }

    #[test]
    fn base_tools_are_always_present_run_python_is_gated() {
        let closed = BlenderProvider::new(MockLink::new(|_, _| Ok(Value::Null)), false);
        let names: Vec<_> = closed
            .tools()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        assert!(names.contains(&"create_primitive".to_string()));
        assert!(!names.contains(&"run_python".to_string()));

        let open = BlenderProvider::new(MockLink::new(|_, _| Ok(Value::Null)), true);
        let names: Vec<_> = open
            .tools()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        assert!(names.contains(&"run_python".to_string()));
    }

    #[test]
    fn call_forwards_op_and_args_and_stringifies_result() {
        let provider = BlenderProvider::new(
            MockLink::new(|op, _| {
                assert_eq!(op, "create_primitive");
                Ok(json!({ "name": "Cube.001" }))
            }),
            false,
        );
        let (text, is_err) = outcome(provider.call(
            "create_primitive",
            &json!({ "kind": "cube", "location": [1, 2, 3] }),
        ));
        assert!(!is_err);
        assert!(text.contains("Cube.001"));
    }

    #[test]
    fn call_surfaces_addon_errors_as_tool_errors() {
        let provider = BlenderProvider::new(
            MockLink::new(|_, _| Err("object 'Ghost' not found".into())),
            false,
        );
        let (text, is_err) = outcome(provider.call("delete_object", &json!({ "name": "Ghost" })));
        assert!(is_err);
        assert_eq!(text, "object 'Ghost' not found");
    }

    #[test]
    fn run_python_is_refused_when_gated_without_ever_hitting_the_link() {
        let provider = BlenderProvider::new(
            MockLink::new(|_, _| panic!("link must not be called when run_python is gated")),
            false,
        );
        let (text, is_err) = outcome(provider.call("run_python", &json!({ "code": "print(1)" })));
        assert!(is_err);
        assert!(text.contains("--allow-run-python"));
    }

    #[test]
    fn run_python_reaches_the_link_when_allowed() {
        let provider = BlenderProvider::new(
            MockLink::new(|op, _| {
                assert_eq!(op, "run_python");
                Ok(Value::String("1\n".into()))
            }),
            true,
        );
        let (text, is_err) = outcome(provider.call("run_python", &json!({ "code": "print(1)" })));
        assert!(!is_err);
        assert_eq!(text, "1\n");
    }
}
