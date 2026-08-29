//! `coding-assistants-mcp-krita` — stdio MCP server bridging to a running
//! Krita instance through the `plugins/krita/` extension's TCP socket.
//!
//!   coding-assistants-mcp-krita [--port <N>] [--allow-run-python]
//!
//! `--port` must match the plugin's configured port (default 9766).

use mcp_core::app_link::TcpAppLink;
use mcp_core::McpServer;
use mcp_krita::{KritaProvider, APP_LABEL, DEFAULT_PORT};
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

    let provider = KritaProvider::new(TcpAppLink::new(port, APP_LABEL), allow_run_python);
    McpServer::new(Arc::new(provider)).run();
}
