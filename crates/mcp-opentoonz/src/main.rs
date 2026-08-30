//! `coding-assistants-mcp-opentoonz` — a deliberately minimal MCP server.
//!
//! OpenToonz has no scripting API / plugin IPC, so this does NOT bridge to
//! a running instance. `scene_info` parses a `.tnz` file directly; `render`
//! is a gated best-effort passthrough to the OpenToonz binary.
//!
//!   coding-assistants-mcp-opentoonz [--opentoonz <path>] [--allow-render]

use mcp_core::McpServer;
use mcp_opentoonz::{OpenToonzLink, OpenToonzProvider};
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut bin = PathBuf::from("opentoonz");
    let mut allow_render = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--opentoonz" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    bin = PathBuf::from(v);
                }
            }
            "--allow-render" => allow_render = true,
            other => eprintln!("ignoring unknown argument {other}"),
        }
        i += 1;
    }

    let link = OpenToonzLink { bin, allow_render };
    McpServer::new(Arc::new(OpenToonzProvider::new(link))).run();
}
