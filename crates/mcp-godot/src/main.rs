//! `coding-assistants-mcp-godot` — stdio MCP server bridging to a running
//! Godot 4 editor through the `plugins/godot/` editor plugin's TCP socket.
//!
//!   coding-assistants-mcp-godot [--port <N>] [--allow-run-script]
//!
//! `--port` must match the plugin's configured port (default 9767).

use mcp_core::app_link::TcpAppLink;
use mcp_core::{McpServer, MemoryProvider, MemoryTools};
use mcp_godot::{GodotProvider, APP_LABEL, DEFAULT_PORT};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut port = DEFAULT_PORT;
    let mut allow_run_script = false;
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
            "--allow-run-script" => allow_run_script = true,
            "--workspace" => {
                i += 1;
                workspace = args.get(i).cloned();
            }
            other => eprintln!("ignoring unknown argument {other}"),
        }
        i += 1;
    }

    let provider = GodotProvider::new(TcpAppLink::new(port, APP_LABEL), allow_run_script);
    McpServer::new(Arc::new(MemoryProvider::new(
        provider,
        MemoryTools::new("godot", workspace),
    )))
    .run();
}
