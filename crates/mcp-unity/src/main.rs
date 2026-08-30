//! `coding-assistants-mcp-unity` — stdio MCP server bridging to a running
//! Unity editor through the `plugins/unity/Editor/` package's TCP socket.
//!
//!   coding-assistants-mcp-unity [--port <N>] [--allow-menu-exec]
//!
//! `--port` must match the Editor script's port (default 9769).

use mcp_core::app_link::TcpAppLink;
use mcp_core::McpServer;
use mcp_unity::{UnityProvider, APP_LABEL, DEFAULT_PORT};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut port = DEFAULT_PORT;
    let mut allow_menu_exec = false;
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
            "--allow-menu-exec" => allow_menu_exec = true,
            other => eprintln!("ignoring unknown argument {other}"),
        }
        i += 1;
    }

    let provider = UnityProvider::new(TcpAppLink::new(port, APP_LABEL), allow_menu_exec);
    McpServer::new(Arc::new(provider)).run();
}
