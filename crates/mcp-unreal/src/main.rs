//! `coding-assistants-mcp-unreal` — stdio MCP server bridging to a running
//! Unreal Engine 5 editor through the `plugins/unreal/` startup script's
//! TCP socket.
//!
//!   coding-assistants-mcp-unreal [--port <N>] [--allow-run-python]
//!
//! `--port` must match the startup script's port (default 9768).

use mcp_core::app_link::TcpAppLink;
use mcp_core::{McpServer, MemoryProvider, MemoryTools};
use mcp_unreal::{UnrealProvider, APP_LABEL, DEFAULT_PORT};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut port = DEFAULT_PORT;
    let mut allow_run_python = false;
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
            "--allow-run-python" => allow_run_python = true,
            "--workspace" => {
                i += 1;
                workspace = args.get(i).cloned();
            }
            other => eprintln!("ignoring unknown argument {other}"),
        }
        i += 1;
    }

    let provider = UnrealProvider::new(TcpAppLink::new(port, APP_LABEL), allow_run_python);
    McpServer::new(Arc::new(MemoryProvider::new(
        provider,
        MemoryTools::new("unreal", workspace),
    )))
    .run();
}
