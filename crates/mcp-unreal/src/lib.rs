//! MCP bridge to a running Unreal Engine 5 editor.
//!
//! **Tier 3 (high risk).** Depends on: UE **5.x**, the *Python Editor
//! Script Plugin* enabled, and the `plugins/unreal/` startup script
//! installed in the project's `Content/Python/`. Rather than speak
//! Unreal's UDP-multicast remote-execution protocol, the startup script
//! opens a plain localhost line-JSON TCP server (same shape as the
//! Blender/Krita/Godot bridges) and marshals `unreal.*` calls to the game
//! thread via a slate post-tick callback. This crate just connects.

use mcp_core::app_link::{result_to_text, AppLink};
use mcp_core::{ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};

pub const DEFAULT_PORT: u16 = 9768;
pub const APP_LABEL: &str = "Unreal Editor";

pub struct UnrealProvider<L: AppLink> {
    link: L,
    allow_run_python: bool,
}

impl<L: AppLink> UnrealProvider<L> {
    pub fn new(link: L, allow_run_python: bool) -> Self {
        Self {
            link,
            allow_run_python,
        }
    }
}

impl<L: AppLink + 'static> ToolProvider for UnrealProvider<L> {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-unreal".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        let mut tools = vec![
            json!({
                "name": "get_editor_summary",
                "description": "Project name, engine version, current level path, and actor count.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "list_actors",
                "description": "Actors in the current level as [{ label, class, location }].",
                "inputSchema": {
                    "type": "object",
                    "properties": { "limit": { "type": "integer", "minimum": 1, "description": "Default 200." } },
                },
            }),
            json!({
                "name": "spawn_actor",
                "description": "Spawn an actor from a class (a built-in name like \"StaticMeshActor\" / \"PointLight\", or a /Game/... blueprint class path) at a world location. Returns its label.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "class_path": { "type": "string" },
                        "location": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "World XYZ (cm). Default origin." },
                        "label": { "type": "string" }
                    },
                    "required": ["class_path"],
                },
            }),
            json!({
                "name": "destroy_actor",
                "description": "Destroy an actor in the current level by label.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "label": { "type": "string" } },
                    "required": ["label"],
                },
            }),
            json!({
                "name": "set_actor_transform",
                "description": "Set an actor's transform by label. Any of location / rotation (pitch,yaw,roll degrees) / scale as [x,y,z].",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "label": { "type": "string" },
                        "location": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
                        "rotation": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
                        "scale": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 }
                    },
                    "required": ["label"],
                },
            }),
            json!({
                "name": "list_assets",
                "description": "Asset paths directly under a content directory (e.g. \"/Game/Maps\").",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Default \"/Game\"." },
                        "recursive": { "type": "boolean", "description": "Default false." }
                    },
                },
            }),
            json!({
                "name": "save_level",
                "description": "Save the current level.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
        ];
        if self.allow_run_python {
            tools.push(json!({
                "name": "run_python",
                "description": "Execute a Python snippet in the editor (full `unreal` module). Returns stdout plus repr of a `result` variable if set.",
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
                "run_python is disabled. Start the bridge with --allow-run-python to enable arbitrary Unreal Python."
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
    fn identity_and_gated_run_python() {
        let p = UnrealProvider::new(MockLink::new(|_, _| Ok(Value::Null)), false);
        assert_eq!(p.server_info().name, "coding-assistants-mcp-unreal");
        assert!(!p.tools().iter().any(|t| t["name"] == "run_python"));
        assert!(
            UnrealProvider::new(MockLink::new(|_, _| Ok(Value::Null)), true)
                .tools()
                .iter()
                .any(|t| t["name"] == "run_python")
        );
    }

    #[test]
    fn spawn_actor_forwards_class_and_location() {
        let p = UnrealProvider::new(
            MockLink::new(|op, args| {
                assert_eq!(op, "spawn_actor");
                assert_eq!(args["class_path"], "PointLight");
                assert_eq!(args["location"], json!([0, 0, 300]));
                Ok(json!({ "label": "PointLight_2" }))
            }),
            false,
        );
        let (text, is_err) = outcome(p.call(
            "spawn_actor",
            &json!({ "class_path": "PointLight", "location": [0, 0, 300] }),
        ));
        assert!(!is_err);
        assert!(text.contains("PointLight_2"));
    }

    #[test]
    fn plugin_errors_become_tool_errors() {
        let p = UnrealProvider::new(
            MockLink::new(|_, _| Err("no actor with label 'Ghost'".into())),
            false,
        );
        let (text, is_err) = outcome(p.call("destroy_actor", &json!({ "label": "Ghost" })));
        assert!(is_err);
        assert_eq!(text, "no actor with label 'Ghost'");
    }

    #[test]
    fn gated_run_python_never_reaches_the_link() {
        let p = UnrealProvider::new(MockLink::new(|_, _| panic!("must not run")), false);
        let (text, is_err) = outcome(p.call("run_python", &json!({ "code": "1" })));
        assert!(is_err);
        assert!(text.contains("--allow-run-python"));
    }
}
