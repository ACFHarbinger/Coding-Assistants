//! Track C-9 — the Settings "Creative Tools" IPC surface.
//!
//! Resolves each `crates/mcp-<tool>` bridge binary against the running
//! app's directory and `$PATH`, reports per-workspace enable / install /
//! running status, and drives `hub::mcp::creative` to write the enabled
//! bridges into a workspace's Claude / Gemini / opencode MCP configs.
//!
//! Codex is intentionally *not* written by these commands — its config is
//! the user-global `~/.codex/config.toml`. [`creative_tools_codex_snippet`]
//! hands back a copy-paste block for the user to apply themselves.

use hub::mcp::creative::{self, CreativeTool};
use hub::mcp::{render_merged, ClientKind, McpServerEntry};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::store::open_store;

/// One creative-tool bridge as the Settings tab sees it: static catalog
/// data plus this machine's resolution of it.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreativeToolStatus {
    pub key: String,
    pub display_name: String,
    pub transport: String,
    pub port: Option<u16>,
    /// The `--allow-*` flag this bridge's code-execution tool needs. Shown
    /// so the user knows it exists; never set by the app.
    pub gated_flag: Option<String>,
    /// `true` when the bridge executable was found next to the app or on
    /// `$PATH`. A registered entry pointing at a missing binary is the
    /// main way this feature looks broken, so the tab surfaces it.
    pub binary_found: bool,
    pub binary_path: Option<String>,
    /// `true` when a process matching the app itself is running.
    pub app_running: bool,
    /// `true` when this bridge is in the workspace's enabled set.
    pub enabled: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreativeToolsStatus {
    pub workspace: String,
    pub tools: Vec<CreativeToolStatus>,
    /// Config files [`creative_tools_set_enabled`] wrote on the last call
    /// (empty for a plain status read).
    pub written_configs: Vec<String>,
}

fn transport_label(tool: &CreativeTool) -> &'static str {
    match tool.transport {
        creative::Transport::Socket => "socket",
        creative::Transport::Subprocess => "subprocess",
        creative::Transport::FileParse => "file-parse",
    }
}

/// Look for `basename` (and, on Windows, `basename.exe`) next to the
/// running executable, then on each `$PATH` entry.
fn resolve_binary(basename: &str) -> Option<PathBuf> {
    let names: Vec<String> = if cfg!(windows) {
        vec![format!("{basename}.exe"), basename.to_string()]
    } else {
        vec![basename.to_string()]
    };

    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }

    for dir in dirs {
        for name in &names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Basenames of creative apps currently running, from one `ps` sweep.
/// Best-effort: an unavailable `ps` just yields "nothing running".
fn running_app_basenames() -> BTreeSet<String> {
    let mut wanted: BTreeSet<String> = BTreeSet::new();
    for tool in creative::CATALOG {
        for name in tool.app_process_names {
            wanted.insert(name.to_ascii_lowercase());
        }
    }

    let mut found = BTreeSet::new();
    let Ok(output) = std::process::Command::new("ps")
        .args(["-eo", "args="])
        .output()
    else {
        return found;
    };
    if !output.status.success() {
        return found;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some(first) = line.split_whitespace().next() else {
            continue;
        };
        let base = Path::new(first)
            .file_name()
            .map(|n| {
                n.to_string_lossy()
                    .trim_end_matches(".exe")
                    .to_ascii_lowercase()
            })
            .unwrap_or_default();
        if wanted.contains(&base) {
            found.insert(base);
        }
    }
    found
}

fn tool_is_running(tool: &CreativeTool, running: &BTreeSet<String>) -> bool {
    tool.app_process_names
        .iter()
        .any(|n| running.contains(&n.to_ascii_lowercase()))
}

fn require_absolute(workspace: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(workspace);
    if !path.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    Ok(path)
}

fn build_status(
    workspace: &Path,
    enabled: &BTreeSet<String>,
    written_configs: Vec<String>,
) -> CreativeToolsStatus {
    let running = running_app_basenames();
    let tools = creative::CATALOG
        .iter()
        .map(|tool| {
            let resolved = resolve_binary(tool.binary);
            CreativeToolStatus {
                key: tool.key.to_string(),
                display_name: tool.display_name.to_string(),
                transport: transport_label(tool).to_string(),
                port: tool.port,
                gated_flag: tool.gated_flag.map(str::to_string),
                binary_found: resolved.is_some(),
                binary_path: resolved.map(|p| p.to_string_lossy().into_owned()),
                app_running: tool_is_running(tool, &running),
                enabled: enabled.contains(tool.key),
            }
        })
        .collect();

    CreativeToolsStatus {
        workspace: workspace.to_string_lossy().into_owned(),
        tools,
        written_configs,
    }
}

/// Resolve one `McpServerEntry` per enabled key whose binary was found.
/// Keys with a missing binary are skipped — they stay in the enabled set
/// (so the toggle reads back on) but are not written into a config that
/// would then point at nothing.
fn resolved_entries(enabled: &BTreeSet<String>) -> Vec<McpServerEntry> {
    enabled
        .iter()
        .filter_map(|key| creative::tool(key))
        .filter_map(|tool| resolve_binary(tool.binary).map(|path| creative::entry_for(tool, &path)))
        .collect()
}

/// Read-only: per-workspace status of every creative-tool bridge.
#[tauri::command]
pub fn creative_tools_status(workspace: String) -> Result<CreativeToolsStatus, String> {
    let path = require_absolute(&workspace)?;
    let store = open_store()?;
    let enabled = creative::enabled_keys(&store, &path);
    Ok(build_status(&path, &enabled, Vec::new()))
}

/// Toggle one bridge for a workspace: update the app-owned registry, then
/// rewrite the workspace's Claude / Gemini / opencode MCP configs so the
/// change takes effect. Returns fresh status including the files written.
#[tauri::command]
pub fn creative_tools_set_enabled(
    workspace: String,
    key: String,
    enabled: bool,
) -> Result<CreativeToolsStatus, String> {
    if creative::tool(&key).is_none() {
        return Err(format!("unknown creative tool: {key}"));
    }
    let path = require_absolute(&workspace)?;
    let store = open_store()?;

    let mut keys = creative::enabled_keys(&store, &path);
    if enabled {
        keys.insert(key);
    } else {
        keys.remove(&key);
    }
    creative::set_enabled_keys(&store, &path, &keys).map_err(|e| e.to_string())?;

    let entries = resolved_entries(&keys);
    let written = creative::apply_to_workspace(&path, &entries).map_err(|e| e.to_string())?;
    let written = written
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    Ok(build_status(&path, &keys, written))
}

/// Re-apply the current enabled set to a workspace's configs — for the
/// tab's "Re-apply" action after a bridge binary is installed or the
/// config files were edited by hand.
#[tauri::command]
pub fn creative_tools_reapply(workspace: String) -> Result<CreativeToolsStatus, String> {
    let path = require_absolute(&workspace)?;
    let store = open_store()?;
    let keys = creative::enabled_keys(&store, &path);
    let entries = resolved_entries(&keys);
    let written = creative::apply_to_workspace(&path, &entries).map_err(|e| e.to_string())?;
    let written = written
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(build_status(&path, &keys, written))
}

/// A `[mcp_servers.*]` TOML block for the workspace's currently-enabled
/// bridges, for the user to paste into `~/.codex/config.toml` themselves
/// (Codex has no per-workspace config the app can safely write).
#[tauri::command]
pub fn creative_tools_codex_snippet(workspace: String) -> Result<String, String> {
    let path = require_absolute(&workspace)?;
    let store = open_store()?;
    let keys = creative::enabled_keys(&store, &path);
    let entries = resolved_entries(&keys);
    if entries.is_empty() {
        return Ok(String::new());
    }
    Ok(render_merged(ClientKind::Codex, &entries, ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_binary_finds_something_on_path() {
        // `ps` is on PATH in every environment this test runs in; use it
        // only to prove the PATH-walk half of the resolver works.
        assert!(resolve_binary("ps").is_some());
        assert!(resolve_binary("definitely-not-a-real-binary-xyz").is_none());
    }

    #[test]
    fn require_absolute_rejects_relative() {
        assert!(require_absolute("rel/x").is_err());
        assert!(require_absolute("/abs/x").is_ok());
    }

    #[test]
    fn status_reports_all_seven_with_enabled_flags() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path();
        let enabled: BTreeSet<String> = ["coding-assistants-mcp-blender".to_string()]
            .into_iter()
            .collect();
        let status = build_status(ws, &enabled, Vec::new());
        assert_eq!(status.tools.len(), 7);
        let blender = status
            .tools
            .iter()
            .find(|t| t.key == "coding-assistants-mcp-blender")
            .unwrap();
        assert!(blender.enabled);
        assert_eq!(blender.port, Some(9765));
        assert!(status.tools.iter().filter(|t| !t.enabled).count() == 6);
    }
}
