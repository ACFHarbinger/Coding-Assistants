//! MCP bridge to a running Blender instance.
//!
//! The `plugins/blender/` addon opens a line-JSON TCP server; this crate
//! is the client via [`mcp_core::app_link`]. Per `tools/call`: connect,
//! send `{ "op": <tool>, "args": {...} }`, read `{ "ok", "result"|"error" }`.

use mcp_core::app_link::{result_to_text, AppLink};
use mcp_core::{ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};

pub const DEFAULT_PORT: u16 = 9765;
pub const APP_LABEL: &str = "Blender";

pub struct BlenderProvider<L: AppLink> {
    link: L,
    /// `run_python` is arbitrary code execution inside Blender. Off unless
    /// the server was started with `--allow-run-python` (or, later, enabled
    /// per workspace in Settings).
    allow_run_python: bool,
}

impl<L: AppLink> BlenderProvider<L> {
    pub fn new(link: L, allow_run_python: bool) -> Self {
        Self {
            link,
            allow_run_python,
        }
    }
}

impl<L: AppLink + 'static> ToolProvider for BlenderProvider<L> {
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
        assert!(open.tools().iter().any(|t| t["name"] == "run_python"));
    }

    #[test]
    fn call_forwards_op_and_stringifies_result() {
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
    fn run_python_is_refused_when_gated_without_hitting_the_link() {
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
