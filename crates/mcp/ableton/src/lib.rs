//! MCP bridge to a running Ableton Live set.
//!
//! Ableton has no Python console. Automation is the Live Object Model (LOM),
//! reached through a MIDI Remote Script (`plugins/ableton/`) that Live's
//! embedded CPython loads. That script opens a localhost line-JSON TCP
//! server (port [`DEFAULT_PORT`]); this crate is the client via
//! [`mcp_core::app_link`].
//!
//! **Not compiler-verified against Ableton** — Live is not on this machine
//! or CI. The Remote Script is written for Live 11/12 (`_Framework`) and
//! `python3 -m py_compile`s outside Live.

use mcp_core::app_link::{result_to_text, AppLink, TcpAppLink};
use mcp_core::{McpServer, MemoryProvider, MemoryTools, ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};
use std::sync::Arc;

pub const DEFAULT_PORT: u16 = 9770;
pub const APP_LABEL: &str = "Ableton Live";

pub struct AbletonProvider<L: AppLink> {
    link: L,
    /// `run_lom` executes arbitrary Python against the LOM. Off unless
    /// started with `--allow-run-lom`.
    allow_run_lom: bool,
}

impl<L: AppLink> AbletonProvider<L> {
    pub fn new(link: L, allow_run_lom: bool) -> Self {
        Self {
            link,
            allow_run_lom,
        }
    }
}

impl<L: AppLink + 'static> ToolProvider for AbletonProvider<L> {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-ableton".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        let mut tools = vec![
            json!({
                "name": "get_song_summary",
                "description": "Live Set overview: name, tempo (BPM), time signature, playing state, track and scene counts.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "list_tracks",
                "description": "Session tracks as [{ index, name, kind, mute, solo, arm }]. `kind` is midi / audio / group / other. Use `index` with the other track tools.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "list_scenes",
                "description": "Scenes as [{ index, name, clip_count }]. `index` is what `fire_scene` / `fire_clip` expect.",
                "inputSchema": { "type": "object", "properties": {} },
            }),
            json!({
                "name": "fire_clip",
                "description": "Launch the clip slot at (track_index, scene_index). Empty armed slots start recording.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "track_index": { "type": "integer", "minimum": 0 },
                        "scene_index": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["track_index", "scene_index"],
                },
            }),
            json!({
                "name": "fire_scene",
                "description": "Launch every clip in a scene by index (Session View scene launch).",
                "inputSchema": {
                    "type": "object",
                    "properties": { "scene_index": { "type": "integer", "minimum": 0 } },
                    "required": ["scene_index"],
                },
            }),
            json!({
                "name": "set_tempo",
                "description": "Set the Live Set tempo in BPM (20–999).",
                "inputSchema": {
                    "type": "object",
                    "properties": { "bpm": { "type": "number", "minimum": 20, "maximum": 999 } },
                    "required": ["bpm"],
                },
            }),
            json!({
                "name": "create_midi_track",
                "description": "Create a MIDI track at the end of the session. Optional `name`. Returns { index, name }.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                },
            }),
            json!({
                "name": "set_track_name",
                "description": "Rename a session track by index.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "track_index": { "type": "integer", "minimum": 0 },
                        "name": { "type": "string" }
                    },
                    "required": ["track_index", "name"],
                },
            }),
        ];
        if self.allow_run_lom {
            tools.push(json!({
                "name": "run_lom",
                "description": "Execute a Python snippet inside Live against the LOM. Bindings: `song` (current Live Set), `Live` (the Live module). Returns stdout plus the repr of a `result` variable if set.",
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
        if name == "run_lom" && !self.allow_run_lom {
            return ToolResult::Err(
                "run_lom is disabled. Start the bridge with --allow-run-lom to enable arbitrary Live Object Model scripting."
                    .into(),
            );
        }
        match self.link.request(name, arguments) {
            Ok(result) => ToolResult::Ok(result_to_text(&result)),
            Err(error) => ToolResult::Err(error),
        }
    }
}

pub fn run(args: &[String]) {
    let mut port = DEFAULT_PORT;
    let mut allow_run_lom = false;
    let mut workspace = std::env::var("CA_MCP_WORKSPACE").ok();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = args.get(i).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--port needs a u16; using {DEFAULT_PORT}");
                    DEFAULT_PORT
                });
            }
            "--allow-run-lom" => allow_run_lom = true,
            "--workspace" => {
                i += 1;
                workspace = args.get(i).cloned();
            }
            other => eprintln!("ignoring unknown argument {other}"),
        }
        i += 1;
    }

    let provider = AbletonProvider::new(TcpAppLink::new(port, APP_LABEL), allow_run_lom);
    McpServer::new(Arc::new(MemoryProvider::new(
        provider,
        MemoryTools::new("ableton", workspace),
    )))
    .run();
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
    fn identity_and_gated_run_lom() {
        let p = AbletonProvider::new(MockLink::new(|_, _| Ok(Value::Null)), false);
        assert_eq!(p.server_info().name, "coding-assistants-mcp-ableton");
        assert!(!p.tools().iter().any(|t| t["name"] == "run_lom"));
        assert!(
            AbletonProvider::new(MockLink::new(|_, _| Ok(Value::Null)), true)
                .tools()
                .iter()
                .any(|t| t["name"] == "run_lom")
        );
    }

    #[test]
    fn fire_clip_forwards_indices() {
        let p = AbletonProvider::new(
            MockLink::new(|op, args| {
                assert_eq!(op, "fire_clip");
                assert_eq!(args["track_index"], 1);
                assert_eq!(args["scene_index"], 2);
                Ok(json!({ "fired": true }))
            }),
            false,
        );
        let (text, is_err) =
            outcome(p.call("fire_clip", &json!({ "track_index": 1, "scene_index": 2 })));
        assert!(!is_err);
        assert!(text.contains("fired"));
    }

    #[test]
    fn plugin_errors_become_tool_errors() {
        let p = AbletonProvider::new(
            MockLink::new(|_, _| Err("track_index 9 out of range".into())),
            false,
        );
        let (text, is_err) =
            outcome(p.call("set_track_name", &json!({ "track_index": 9, "name": "x" })));
        assert!(is_err);
        assert_eq!(text, "track_index 9 out of range");
    }

    #[test]
    fn gated_run_lom_never_reaches_the_link() {
        let p = AbletonProvider::new(MockLink::new(|_, _| panic!("must not run")), false);
        let (text, is_err) = outcome(p.call("run_lom", &json!({ "code": "result = 1" })));
        assert!(is_err);
        assert!(text.contains("--allow-run-lom"));
    }

    #[test]
    fn memory_tools_remember_and_recall_only_ableton_memories() {
        let dir = tempfile::tempdir().unwrap();
        let provider = MemoryProvider::new(
            AbletonProvider::new(MockLink::new(|_, _| Ok(Value::Null)), false),
            MemoryTools::with_hub_home("ableton", Some("/project".into()), dir.path().into()),
        );

        assert!(provider
            .tools()
            .iter()
            .any(|tool| tool["name"] == "remember"));
        assert!(matches!(
            provider.call("remember", &json!({"body": "Keep drums on track 1"})),
            ToolResult::Ok(_)
        ));
        assert!(matches!(
            provider.call("recall", &json!({"query": "drums"})),
            ToolResult::Ok(text) if text.contains("track 1")
        ));
    }
}
