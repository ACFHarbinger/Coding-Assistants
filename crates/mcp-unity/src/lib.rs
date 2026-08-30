//! MCP bridge to a running Unity editor.
//!
//! **Tier 3 (high risk).** Needs `plugins/unity/Editor/` copied into a
//! project's `Assets/` (or referenced as a local package) so the
//! `[InitializeOnLoad]` bridge starts a localhost line-JSON TCP server
//! (port [`DEFAULT_PORT`]) and pumps jobs on the editor main thread from
//! `EditorApplication.update`. This crate is the client via
//! [`mcp_core::app_link`].
//!
//! Unity has no C# eval; the gated tool here is `execute_menu_item`
//! (`EditorApplication.ExecuteMenuItem`), not arbitrary code.

use mcp_core::app_link::{result_to_text, AppLink};
use mcp_core::{ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};

pub const DEFAULT_PORT: u16 = 9769;
pub const APP_LABEL: &str = "Unity Editor";

pub struct UnityProvider<L: AppLink> {
    link: L,
    allow_menu_exec: bool,
}

impl<L: AppLink> UnityProvider<L> {
    pub fn new(link: L, allow_menu_exec: bool) -> Self {
        Self {
            link,
            allow_menu_exec,
        }
    }
}

impl<L: AppLink + 'static> ToolProvider for UnityProvider<L> {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-unity".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        let mut tools = vec![
            json!({
                "name": "get_editor_summary",
                "description": "Unity version, active scene name/path, root GameObject count, and play-mode state.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "list_gameobjects",
                "description": "The active scene hierarchy as [{ name, path, active, components }]. `path` (e.g. \"Player/Weapon\") is what the other tools expect.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "limit": { "type": "integer", "minimum": 1, "description": "Default 300." } },
                },
            }),
            json!({
                "name": "create_gameobject",
                "description": "Create a GameObject in the active scene. `primitive` (Cube/Sphere/Capsule/Cylinder/Plane/Quad) creates a primitive; otherwise an empty GameObject. Returns its path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "primitive": { "type": "string", "enum": ["Cube", "Sphere", "Capsule", "Cylinder", "Plane", "Quad"] },
                        "parent": { "type": "string", "description": "Path of the parent GameObject. Empty = scene root." },
                        "position": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 }
                    },
                },
            }),
            json!({
                "name": "delete_gameobject",
                "description": "Destroy a GameObject (and its children) by path.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"],
                },
            }),
            json!({
                "name": "set_transform",
                "description": "Set a GameObject's local transform by path. Any of position / rotation (euler degrees) / scale as [x,y,z].",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "position": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
                        "rotation": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
                        "scale": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 }
                    },
                    "required": ["path"],
                },
            }),
            json!({
                "name": "add_component",
                "description": "Add a component to a GameObject by type name (e.g. \"Rigidbody\", \"BoxCollider\", \"Light\").",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "component": { "type": "string" }
                    },
                    "required": ["path", "component"],
                },
            }),
            json!({
                "name": "list_assets",
                "description": "Asset paths under a project folder (e.g. \"Assets/Prefabs\"), via AssetDatabase.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "folder": { "type": "string", "description": "Default \"Assets\"." },
                        "filter": { "type": "string", "description": "AssetDatabase search filter, e.g. \"t:Prefab\". Optional." }
                    },
                },
            }),
            json!({
                "name": "save_scene",
                "description": "Save the active scene.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
        ];
        if self.allow_menu_exec {
            tools.push(json!({
                "name": "execute_menu_item",
                "description": "Run an editor menu command by its full path, e.g. \"GameObject/Align With View\" or \"Assets/Refresh\". Returns whether Unity found the item.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "menu_path": { "type": "string" } },
                    "required": ["menu_path"],
                },
            }));
        }
        tools
    }

    fn call(&self, name: &str, arguments: &Value) -> ToolResult {
        if name == "execute_menu_item" && !self.allow_menu_exec {
            return ToolResult::Err(
                "execute_menu_item is disabled. Start the bridge with --allow-menu-exec to enable running editor menu commands."
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
    fn identity_and_gated_menu_exec() {
        let p = UnityProvider::new(MockLink::new(|_, _| Ok(Value::Null)), false);
        assert_eq!(p.server_info().name, "coding-assistants-mcp-unity");
        assert!(!p.tools().iter().any(|t| t["name"] == "execute_menu_item"));
        assert!(
            UnityProvider::new(MockLink::new(|_, _| Ok(Value::Null)), true)
                .tools()
                .iter()
                .any(|t| t["name"] == "execute_menu_item")
        );
    }

    #[test]
    fn create_gameobject_forwards_primitive_and_parent() {
        let p = UnityProvider::new(
            MockLink::new(|op, args| {
                assert_eq!(op, "create_gameobject");
                assert_eq!(args["primitive"], "Cube");
                assert_eq!(args["parent"], "Level");
                Ok(json!({ "path": "Level/Cube" }))
            }),
            false,
        );
        let (text, is_err) = outcome(p.call(
            "create_gameobject",
            &json!({ "primitive": "Cube", "parent": "Level" }),
        ));
        assert!(!is_err);
        assert!(text.contains("Level/Cube"));
    }

    #[test]
    fn bridge_errors_become_tool_errors() {
        let p = UnityProvider::new(
            MockLink::new(|_, _| Err("GameObject 'Ghost' not found".into())),
            false,
        );
        let (text, is_err) = outcome(p.call("delete_gameobject", &json!({ "path": "Ghost" })));
        assert!(is_err);
        assert_eq!(text, "GameObject 'Ghost' not found");
    }

    #[test]
    fn gated_menu_exec_never_reaches_the_link() {
        let p = UnityProvider::new(MockLink::new(|_, _| panic!("must not run")), false);
        let (text, is_err) = outcome(p.call(
            "execute_menu_item",
            &json!({ "menu_path": "Assets/Refresh" }),
        ));
        assert!(is_err);
        assert!(text.contains("--allow-menu-exec"));
    }
}
