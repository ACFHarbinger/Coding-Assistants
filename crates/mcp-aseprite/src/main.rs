//! `coding-assistants-mcp-aseprite` — stdio MCP server that drives Aseprite
//! through batch-mode Lua (`aseprite -b --script`). No running instance is
//! needed; every tool operates on a sprite file path.
//!
//!   coding-assistants-mcp-aseprite [--aseprite <path>] [--script <dispatch.lua>] [--allow-apply-script]
//!
//! `--aseprite` defaults to `aseprite` on PATH. `--script` defaults to a
//! `dispatch.lua` sitting next to this binary, then to
//! `plugins/aseprite/dispatch.lua` relative to the current directory.

use mcp_aseprite::{AsepriteProvider, CliAsepriteLink};
use mcp_core::McpServer;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut bin = PathBuf::from("aseprite");
    let mut script: Option<PathBuf> = None;
    let mut allow_apply_script = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--aseprite" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    bin = PathBuf::from(v);
                }
            }
            "--script" => {
                i += 1;
                script = args.get(i).map(PathBuf::from);
            }
            "--allow-apply-script" => allow_apply_script = true,
            other => eprintln!("ignoring unknown argument {other}"),
        }
        i += 1;
    }

    let script = script.unwrap_or_else(default_script_path);
    let link = CliAsepriteLink { bin, script };
    let provider = AsepriteProvider::new(link, allow_apply_script);
    McpServer::new(Arc::new(provider)).run();
}

fn default_script_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let beside = dir.join("dispatch.lua");
            if beside.is_file() {
                return beside;
            }
        }
    }
    PathBuf::from("plugins/aseprite/dispatch.lua")
}
