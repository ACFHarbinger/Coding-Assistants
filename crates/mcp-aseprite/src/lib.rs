//! MCP bridge to Aseprite.
//!
//! Unlike the Blender/Krita/Godot bridges, Aseprite has **no live session
//! and no socket** — its automation surface is batch-mode Lua:
//!
//! ```text
//! aseprite -b --script plugins/aseprite/dispatch.lua \
//!          --script-param op=<tool> --script-param <k>=<v> ...
//! ```
//!
//! So [`CliAsepriteLink`] spawns Aseprite once per `tools/call`. The
//! dispatch script runs the op against a file path and prints one
//! `{"ok":..., "result"|"error":...}` line, which [`mcp_core::app_link`]'s
//! `parse_response` reads. Every tool is file-oriented (there is no
//! "active sprite").

use mcp_core::app_link::{parse_response, result_to_text, AppLink};
use mcp_core::{ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

pub const APP_LABEL: &str = "Aseprite";

/// Spawns `aseprite -b --script <dispatch.lua>` per call. `args` values
/// are flattened to `--script-param key=value` (Aseprite has no bundled
/// JSON parser, so requests are flat scalars; the script emits JSON).
pub struct CliAsepriteLink {
    pub bin: PathBuf,
    pub script: PathBuf,
}

impl AppLink for CliAsepriteLink {
    fn request(&self, op: &str, args: &Value) -> Result<Value, String> {
        let mut cmd = Command::new(&self.bin);
        cmd.arg("-b").arg("--script").arg(&self.script);
        cmd.arg("--script-param").arg(format!("op={op}"));
        if let Some(map) = args.as_object() {
            for (k, v) in map {
                cmd.arg("--script-param").arg(format!("{k}={}", scalar(v)));
            }
        }
        let output = cmd.output().map_err(|e| {
            format!(
                "could not run {}: {e}. Is Aseprite installed and on PATH (or pass --aseprite <path>)?",
                self.bin.display()
            )
        })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let last = stdout
            .lines()
            .rev()
            .find(|l| l.trim_start().starts_with('{'))
            .ok_or_else(|| {
                let stderr = String::from_utf8_lossy(&output.stderr);
                format!(
                    "Aseprite produced no JSON result line. stderr: {}",
                    stderr.trim()
                )
            })?;
        parse_response(last)
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub struct AsepriteProvider<L: AppLink> {
    link: L,
    allow_apply_script: bool,
}

impl<L: AppLink> AsepriteProvider<L> {
    pub fn new(link: L, allow_apply_script: bool) -> Self {
        Self {
            link,
            allow_apply_script,
        }
    }
}

impl<L: AppLink + 'static> ToolProvider for AsepriteProvider<L> {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-aseprite".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        let path_prop = json!({ "type": "string", "description": "Absolute path to a .aseprite/.ase/.png sprite file." });
        let mut tools = vec![
            json!({
                "name": "sprite_info",
                "description": "Read a sprite file's dimensions, color mode, frame count, layer count, and palette size.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": path_prop },
                    "required": ["path"],
                },
            }),
            json!({
                "name": "list_layers",
                "description": "Layer names of a sprite file, bottom to top, with visibility.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": path_prop },
                    "required": ["path"],
                },
            }),
            json!({
                "name": "export",
                "description": "Save a sprite to another path/format. Extension of `out` picks the format (.png, .gif, .jpg, .aseprite, ...). For animated GIF/APNG the whole timeline is written.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": path_prop,
                        "out": { "type": "string", "description": "Absolute output path." },
                        "scale": { "type": "number", "description": "Integer upscale factor. Default 1." }
                    },
                    "required": ["path", "out"],
                },
            }),
            json!({
                "name": "resize",
                "description": "Resize a sprite to width x height (px) and save to `out` (or overwrite `path` if omitted).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": path_prop,
                        "width": { "type": "integer", "minimum": 1 },
                        "height": { "type": "integer", "minimum": 1 },
                        "out": { "type": "string" }
                    },
                    "required": ["path", "width", "height"],
                },
            }),
            json!({
                "name": "export_spritesheet",
                "description": "Pack every frame of a sprite into a single sheet image at `out`, plus a JSON metadata file alongside it.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": path_prop,
                        "out": { "type": "string", "description": "Absolute sheet image path (.png)." },
                        "columns": { "type": "integer", "minimum": 1, "description": "Sheet columns. Default: a square-ish packing." }
                    },
                    "required": ["path", "out"],
                },
            }),
            json!({
                "name": "get_palette",
                "description": "The sprite's palette as an array of #RRGGBBAA strings.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": path_prop },
                    "required": ["path"],
                },
            }),
        ];
        if self.allow_apply_script {
            tools.push(json!({
                "name": "apply_script",
                "description": "Run an arbitrary Aseprite Lua snippet with the sprite open as `spr` (a Sprite). Set `result` to return a string. Saves the sprite afterwards unless the snippet sets `no_save = true`.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": path_prop, "code": { "type": "string" } },
                    "required": ["path", "code"],
                },
            }));
        }
        tools
    }

    fn call(&self, name: &str, arguments: &Value) -> ToolResult {
        if name == "apply_script" && !self.allow_apply_script {
            return ToolResult::Err(
                "apply_script is disabled. Start the bridge with --allow-apply-script to enable arbitrary Aseprite Lua."
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
    fn identity_and_gated_apply_script() {
        let p = AsepriteProvider::new(MockLink::new(|_, _| Ok(Value::Null)), false);
        assert_eq!(p.server_info().name, "coding-assistants-mcp-aseprite");
        assert!(!p.tools().iter().any(|t| t["name"] == "apply_script"));
        assert!(
            AsepriteProvider::new(MockLink::new(|_, _| Ok(Value::Null)), true)
                .tools()
                .iter()
                .any(|t| t["name"] == "apply_script")
        );
    }

    #[test]
    fn export_forwards_path_and_out() {
        let p = AsepriteProvider::new(
            MockLink::new(|op, args| {
                assert_eq!(op, "export");
                assert_eq!(args["out"], "/tmp/a.gif");
                Ok(json!({ "saved": "/tmp/a.gif" }))
            }),
            false,
        );
        let (text, is_err) = outcome(p.call(
            "export",
            &json!({ "path": "/x/a.aseprite", "out": "/tmp/a.gif" }),
        ));
        assert!(!is_err);
        assert!(text.contains("/tmp/a.gif"));
    }

    #[test]
    fn link_errors_become_tool_errors() {
        let p = AsepriteProvider::new(
            MockLink::new(|_, _| Err("file not found: /x/missing.ase".into())),
            false,
        );
        let (text, is_err) = outcome(p.call("sprite_info", &json!({ "path": "/x/missing.ase" })));
        assert!(is_err);
        assert_eq!(text, "file not found: /x/missing.ase");
    }

    #[test]
    fn gated_apply_script_never_reaches_the_link() {
        let p = AsepriteProvider::new(MockLink::new(|_, _| panic!("must not run")), false);
        let (text, is_err) = outcome(p.call("apply_script", &json!({ "path": "/x", "code": "1" })));
        assert!(is_err);
        assert!(text.contains("--allow-apply-script"));
    }

    #[test]
    fn cli_link_reports_a_helpful_error_when_aseprite_is_missing() {
        let link = CliAsepriteLink {
            bin: PathBuf::from("definitely-not-a-real-binary-xyz"),
            script: PathBuf::from("/tmp/nope.lua"),
        };
        let err = link
            .request("sprite_info", &json!({ "path": "/x" }))
            .unwrap_err();
        assert!(err.contains("Aseprite") && err.contains("PATH"));
    }

    #[test]
    fn scalar_flattens_json_values() {
        assert_eq!(scalar(&json!("hi")), "hi");
        assert_eq!(scalar(&json!(3)), "3");
        assert_eq!(scalar(&json!(true)), "true");
        assert_eq!(scalar(&Value::Null), "");
    }
}
