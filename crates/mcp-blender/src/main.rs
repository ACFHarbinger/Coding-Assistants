//! `coding-assistants-mcp-blender` — the stdio MCP server an agent's config
//! points at. Talks to a running Blender instance through the
//! `plugins/blender/` addon's TCP line-JSON socket.
//!
//! Usage (as an MCP `command` entry):
//!   coding-assistants-mcp-blender [--port <N>] [--allow-run-python]
//!
//! `--port` must match the addon's configured port (default 9765).
//! `--allow-run-python` exposes an arbitrary-`bpy`-code tool; off by default.

use mcp_blender::{BlenderProvider, TcpLink, DEFAULT_PORT};
use mcp_core::McpServer;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut port = DEFAULT_PORT;
    let mut allow_run_python = false;
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
            "--allow-run-python" => allow_run_python = true,
            other => eprintln!("ignoring unknown argument {other}"),
        }
        i += 1;
    }

    let provider = BlenderProvider::new(TcpLink { port }, allow_run_python);
    McpServer::new(Arc::new(provider)).run();
}
