//! MCP bridge to a running Krita instance.
//!
//! The `plugins/krita/` PyKrita extension opens a line-JSON TCP server;
//! this crate connects via [`mcp_core::app_link`] once per `tools/call`.
//! Second consumer of `mcp-core` after `mcp-blender` — the two share the
//! socket transport and differ only in `tools()` / gating.

use mcp_core::app_link::{result_to_text, AppLink};
use mcp_core::{ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};

pub const DEFAULT_PORT: u16 = 9766;
pub const APP_LABEL: &str = "Krita";

pub struct KritaProvider<L: AppLink> {
    link: L,
    /// `run_python` executes arbitrary code against Krita's scripting API.
    /// Off unless started with `--allow-run-python`.
    allow_run_python: bool,
}

impl<L: AppLink> KritaProvider<L> {
    pub fn new(link: L, allow_run_python: bool) -> Self {
        Self {
            link,
            allow_run_python,
        }
    }
}

impl<L: AppLink + 'static> ToolProvider for KritaProvider<L> {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-krita".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        let mut tools = vec![
            json!({
                "name": "get_document_summary",
                "description": "Active document overview: name, width, height (px), color model/depth, resolution, layer count.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "list_layers",
                "description": "Top-level layers of the active document as { name, type, visible, opacity }.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "create_document",
                "description": "Create a new document and make it active. Returns its name.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "width": { "type": "integer", "minimum": 1 },
                        "height": { "type": "integer", "minimum": 1 },
                        "name": { "type": "string" },
                        "color_model": { "type": "string", "enum": ["RGBA", "GRAYA", "CMYKA"], "description": "Default RGBA." },
                        "resolution": { "type": "number", "description": "DPI. Default 300." }
                    },
                    "required": ["width", "height"],
                },
            }),
            json!({
                "name": "create_paint_layer",
                "description": "Add a paint layer to the active document above the active node. Returns its name.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                },
            }),
            json!({
                "name": "set_layer_visible",
                "description": "Show or hide a layer by name, then refresh the projection.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "name": { "type": "string" }, "visible": { "type": "boolean" } },
                    "required": ["name", "visible"],
                },
            }),
            json!({
                "name": "set_layer_opacity",
                "description": "Set a layer's opacity (0-100) by name.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "name": { "type": "string" }, "opacity": { "type": "number", "minimum": 0, "maximum": 100 } },
                    "required": ["name", "opacity"],
                },
            }),
            json!({
                "name": "export_document",
                "description": "Flatten-export the active document to an image file. Format is inferred from the path extension (.png/.jpg/.webp/.tiff/.kra).",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "Absolute output path." } },
                    "required": ["path"],
                },
            }),
        ];
        if self.allow_run_python {
            tools.push(json!({
                "name": "run_python",
                "description": "Execute a Python snippet inside Krita (full `krita` scripting API). Returns stdout plus the repr of a `result` variable if set.",
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
                "run_python is disabled. Start the bridge with --allow-run-python to enable arbitrary Krita scripting."
                    .into(),
            );
        }
        match self.link.request(name, arguments) {
            Ok(result) => ToolResult::Ok(result_to_text(&result)),
            Err(error) => ToolResult::Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type ReplyFn = dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync;
    struct MockLink(Box<ReplyFn>);
    impl MockLink {
        fn new(
            reply: impl Fn(&str, &Value) -> Result<Value, String> + Send + Sync + 'static,
        ) -> Self {
            Self(Box::new(reply))
        }
    }
    impl AppLink for MockLink {
        fn request(&self, op: &str, args: &Value) -> Result<Value, String> {
            (self.0)(op, args)
        }
    }
    fn outcome(r: ToolResult) -> (String, bool) {
        match r {
            ToolResult::Ok(t) => (t, false),
            ToolResult::Err(t) => (t, true),
        }
    }

    #[test]
    fn server_identity_and_gated_run_python() {
        let p = KritaProvider::new(MockLink::new(|_, _| Ok(Value::Null)), false);
        assert_eq!(p.server_info().name, "coding-assistants-mcp-krita");
        assert!(!p.tools().iter().any(|t| t["name"] == "run_python"));
        assert!(
            KritaProvider::new(MockLink::new(|_, _| Ok(Value::Null)), true)
                .tools()
                .iter()
                .any(|t| t["name"] == "run_python")
        );
    }

    #[test]
    fn export_forwards_the_path_and_returns_text() {
        let p = KritaProvider::new(
            MockLink::new(|op, args| {
                assert_eq!(op, "export_document");
                assert_eq!(args["path"], "/tmp/out.png");
                Ok(json!({ "exported": "/tmp/out.png" }))
            }),
            false,
        );
        let (text, is_err) = outcome(p.call("export_document", &json!({ "path": "/tmp/out.png" })));
        assert!(!is_err);
        assert!(text.contains("/tmp/out.png"));
    }

    #[test]
    fn plugin_errors_become_tool_errors() {
        let p = KritaProvider::new(
            MockLink::new(|_, _| Err("no active document".into())),
            false,
        );
        let (text, is_err) = outcome(p.call("list_layers", &json!({})));
        assert!(is_err);
        assert_eq!(text, "no active document");
    }

    #[test]
    fn gated_run_python_never_reaches_the_link() {
        let p = KritaProvider::new(MockLink::new(|_, _| panic!("link must not be hit")), false);
        let (text, is_err) = outcome(p.call("run_python", &json!({ "code": "1" })));
        assert!(is_err);
        assert!(text.contains("--allow-run-python"));
    }
}
