//! MCP bridge to a running Godot 4 editor.
//!
//! The `plugins/godot/` editor plugin (`EditorPlugin`, GDScript) opens a
//! line-JSON `TCPServer` polled from `_process`; this crate connects via
//! [`mcp_core::app_link`] once per `tools/call`. Third `mcp-core` consumer
//! after Blender and Krita — same `AppLink`, different `tools()`.
//!
//! Godot has no in-editor Python. `run_gdscript` (gated) hands a snippet
//! to the plugin, which wraps it in a temporary tool script.

use mcp_core::app_link::{result_to_text, AppLink};
use mcp_core::{ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};

pub const DEFAULT_PORT: u16 = 9767;
pub const APP_LABEL: &str = "Godot";

pub struct GodotProvider<L: AppLink> {
    link: L,
    /// `run_gdscript` runs arbitrary GDScript in the editor. Off unless
    /// started with `--allow-run-script`.
    allow_run_script: bool,
}

impl<L: AppLink> GodotProvider<L> {
    pub fn new(link: L, allow_run_script: bool) -> Self {
        Self {
            link,
            allow_run_script,
        }
    }
}

impl<L: AppLink + 'static> ToolProvider for GodotProvider<L> {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-godot".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        let mut tools = vec![
            json!({
                "name": "get_scene_summary",
                "description": "Currently edited scene: its res:// path, root node name/type, and total node count.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "list_nodes",
                "description": "The edited scene tree as [{ name, type, path }], depth-first. `path` is relative to the scene root and is what add_node/delete_node/set_node_property expect.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "max_depth": { "type": "integer", "minimum": 1, "description": "Default 6." } },
                },
            }),
            json!({
                "name": "add_node",
                "description": "Instance a built-in node class and add it under `parent` (a node path, or empty for the scene root). The new node's owner is set so it saves with the scene. Returns its path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "class_name": { "type": "string", "description": "e.g. \"Node2D\", \"Sprite2D\", \"Label\", \"CollisionShape2D\"." },
                        "name": { "type": "string" },
                        "parent": { "type": "string", "description": "Node path relative to the scene root. Empty = root." }
                    },
                    "required": ["class_name"],
                },
            }),
            json!({
                "name": "delete_node",
                "description": "Remove a node (and its children) from the edited scene by path.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"],
                },
            }),
            json!({
                "name": "set_node_property",
                "description": "Set one property on a node by path. `value` is JSON; Vector2/Vector3/Color accept a [number,...] array.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "property": { "type": "string", "description": "e.g. \"position\", \"text\", \"visible\"." },
                        "value": {}
                    },
                    "required": ["path", "property", "value"],
                },
            }),
            json!({
                "name": "save_scene",
                "description": "Save the currently edited scene to its file.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "open_scene",
                "description": "Open a scene file in the editor and make it the edited scene.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "res:// path to a .tscn/.scn." } },
                    "required": ["path"],
                },
            }),
            json!({
                "name": "list_project_scenes",
                "description": "Every .tscn / .scn file in the project, as res:// paths.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
        ];
        if self.allow_run_script {
            tools.push(json!({
                "name": "run_gdscript",
                "description": "Run a GDScript snippet in the editor. The plugin wraps it in a temporary `@tool` script's `_run()` — set a `result` local to return a repr. Returns stdout + result.",
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
        if name == "run_gdscript" && !self.allow_run_script {
            return ToolResult::Err(
                "run_gdscript is disabled. Start the bridge with --allow-run-script to enable arbitrary GDScript execution."
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
    fn identity_and_gated_run_gdscript() {
        let p = GodotProvider::new(MockLink::new(|_, _| Ok(Value::Null)), false);
        assert_eq!(p.server_info().name, "coding-assistants-mcp-godot");
        assert!(!p.tools().iter().any(|t| t["name"] == "run_gdscript"));
        assert!(
            GodotProvider::new(MockLink::new(|_, _| Ok(Value::Null)), true)
                .tools()
                .iter()
                .any(|t| t["name"] == "run_gdscript")
        );
    }

    #[test]
    fn add_node_forwards_class_and_parent() {
        let p = GodotProvider::new(
            MockLink::new(|op, args| {
                assert_eq!(op, "add_node");
                assert_eq!(args["class_name"], "Sprite2D");
                assert_eq!(args["parent"], "Player");
                Ok(json!({ "path": "Player/Sprite2D" }))
            }),
            false,
        );
        let (text, is_err) = outcome(p.call(
            "add_node",
            &json!({ "class_name": "Sprite2D", "parent": "Player" }),
        ));
        assert!(!is_err);
        assert!(text.contains("Player/Sprite2D"));
    }

    #[test]
    fn plugin_errors_become_tool_errors() {
        let p = GodotProvider::new(
            MockLink::new(|_, _| Err("no scene is being edited".into())),
            false,
        );
        let (text, is_err) = outcome(p.call("get_scene_summary", &json!({})));
        assert!(is_err);
        assert_eq!(text, "no scene is being edited");
    }

    #[test]
    fn gated_run_gdscript_never_reaches_the_link() {
        let p = GodotProvider::new(MockLink::new(|_, _| panic!("must not be hit")), false);
        let (text, is_err) = outcome(p.call("run_gdscript", &json!({ "code": "print(1)" })));
        assert!(is_err);
        assert!(text.contains("--allow-run-script"));
    }
}
