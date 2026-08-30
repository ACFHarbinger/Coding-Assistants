//! Track C-9 — per-workspace registration of the creative-tool MCP bridges.
//!
//! The `crates/mcp-<tool>` bridges (Blender, Krita, Godot, Aseprite,
//! Unreal, Unity, OpenToonz) are all built but nothing hands them to an
//! agent. This module owns:
//!
//! - [`CATALOG`] — static metadata for each bridge (key, binary basename,
//!   default args, transport, the `--allow-*` flag it never sets on its
//!   own, and the process names that mean the app is running).
//! - The per-workspace **enabled set**, stored next to the Channel
//!   registry under `servers_dir(store)` as `<name>.creative.json`, keyed
//!   by [`workspace_server_name`] so two repos with the same directory
//!   name never collide and the record survives a repo being deleted.
//! - [`apply_to_workspace`] — rewrites each client's MCP config so exactly
//!   the enabled bridges are registered, removing disabled ones and never
//!   touching a server the user added by hand.
//!
//! Binary-path resolution is **not** done here — the caller (the Tauri
//! command layer) resolves each `binary` basename against the running
//! app's directory and `$PATH`, and passes the resolved
//! [`McpServerEntry`] list in. That keeps this crate free of any
//! assumption about where the bridges are installed.

use crate::mcp::{render_replacing, ClientKind, McpServerEntry};
use crate::{servers_dir, workspace_server_name, HubError, HubStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// How a bridge reaches its app — informational, so the UI can say what
/// "installed" means for each tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    /// Native plugin listening on a localhost TCP port.
    Socket,
    /// One `app -b --script …` subprocess per call (Aseprite).
    Subprocess,
    /// Reads project files directly; no live connection (OpenToonz).
    FileParse,
}

/// Static description of one `crates/mcp-<tool>` bridge.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CreativeTool {
    /// Stable config-map key, identical across every client.
    pub key: &'static str,
    /// Human label ("Blender").
    pub display_name: &'static str,
    /// Bridge executable basename (its Cargo `[[bin]]` name).
    pub binary: &'static str,
    /// Args appended after the resolved binary path.
    pub default_args: &'static [&'static str],
    pub transport: Transport,
    /// Localhost port the plugin listens on, for [`Transport::Socket`].
    pub port: Option<u16>,
    /// The `--allow-*` flag that unlocks this bridge's code-execution tool,
    /// or `None` if it has none. Never passed automatically.
    pub gated_flag: Option<&'static str>,
    /// Process basenames that indicate the app itself is running.
    pub app_process_names: &'static [&'static str],
}

/// Every creative-tool bridge, in display order.
pub const CATALOG: &[CreativeTool] = &[
    CreativeTool {
        key: "coding-assistants-mcp-blender",
        display_name: "Blender",
        binary: "coding-assistants-mcp-blender",
        default_args: &["--port", "9765"],
        transport: Transport::Socket,
        port: Some(9765),
        gated_flag: Some("--allow-run-python"),
        app_process_names: &["blender"],
    },
    CreativeTool {
        key: "coding-assistants-mcp-krita",
        display_name: "Krita",
        binary: "coding-assistants-mcp-krita",
        default_args: &["--port", "9766"],
        transport: Transport::Socket,
        port: Some(9766),
        gated_flag: Some("--allow-run-python"),
        app_process_names: &["krita"],
    },
    CreativeTool {
        key: "coding-assistants-mcp-godot",
        display_name: "Godot",
        binary: "coding-assistants-mcp-godot",
        default_args: &["--port", "9767"],
        transport: Transport::Socket,
        port: Some(9767),
        gated_flag: Some("--allow-run-script"),
        app_process_names: &["godot", "godot4"],
    },
    CreativeTool {
        key: "coding-assistants-mcp-aseprite",
        display_name: "Aseprite",
        binary: "coding-assistants-mcp-aseprite",
        default_args: &[],
        transport: Transport::Subprocess,
        port: None,
        gated_flag: Some("--allow-apply-script"),
        app_process_names: &["aseprite"],
    },
    CreativeTool {
        key: "coding-assistants-mcp-unreal",
        display_name: "Unreal Editor",
        binary: "coding-assistants-mcp-unreal",
        default_args: &["--port", "9768"],
        transport: Transport::Socket,
        port: Some(9768),
        gated_flag: Some("--allow-run-python"),
        app_process_names: &["UnrealEditor", "UE4Editor"],
    },
    CreativeTool {
        key: "coding-assistants-mcp-unity",
        display_name: "Unity Editor",
        binary: "coding-assistants-mcp-unity",
        default_args: &["--port", "9769"],
        transport: Transport::Socket,
        port: Some(9769),
        gated_flag: Some("--allow-menu-exec"),
        app_process_names: &["Unity", "unity-editor"],
    },
    CreativeTool {
        key: "coding-assistants-mcp-opentoonz",
        display_name: "OpenToonz",
        binary: "coding-assistants-mcp-opentoonz",
        default_args: &[],
        transport: Transport::FileParse,
        port: None,
        gated_flag: Some("--allow-render"),
        app_process_names: &["OpenToonz", "opentoonz"],
    },
];

/// Look up a catalog entry by its stable key.
pub fn tool(key: &str) -> Option<&'static CreativeTool> {
    CATALOG.iter().find(|t| t.key == key)
}

/// The client CLIs a per-workspace registration writes to.
///
/// **Codex is deliberately excluded.** Its config lives at
/// `~/.codex/config.toml`, global to the user — a per-workspace toggle
/// must never mutate it. The Settings tab gives Codex a copy-paste
/// snippet ([`crate::mcp::render_merged`] with [`ClientKind::Codex`])
/// instead, which the user applies themselves.
pub const WORKSPACE_CLIENTS: &[ClientKind] =
    &[ClientKind::Claude, ClientKind::Gemini, ClientKind::Opencode];

fn state_path(store: &HubStore, workspace: &Path) -> PathBuf {
    servers_dir(store).join(format!(
        "{}.creative.json",
        workspace_server_name(workspace)
    ))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StateFile {
    #[serde(default)]
    enabled: Vec<String>,
    /// Echoed back for an owner-management list view, like the Channel
    /// registry's per-workspace files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workspace: Option<String>,
}

/// The tool keys currently enabled for `workspace` (empty if never set).
/// Unknown keys — a catalog entry removed in a later version — are
/// dropped rather than surfaced.
pub fn enabled_keys(store: &HubStore, workspace: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(state_path(store, workspace))
        .ok()
        .and_then(|raw| serde_json::from_str::<StateFile>(&raw).ok())
        .map(|state| {
            state
                .enabled
                .into_iter()
                .filter(|key| tool(key).is_some())
                .collect()
        })
        .unwrap_or_default()
}

/// Persist the enabled set for `workspace`. Writes only the app-owned
/// registry file — call [`apply_to_workspace`] afterwards to push the
/// change into the client configs.
pub fn set_enabled_keys(
    store: &HubStore,
    workspace: &Path,
    keys: &BTreeSet<String>,
) -> Result<(), HubError> {
    std::fs::create_dir_all(servers_dir(store))?;
    let state = StateFile {
        enabled: keys.iter().cloned().collect(),
        workspace: Some(workspace.to_string_lossy().into_owned()),
    };
    std::fs::write(
        state_path(store, workspace),
        serde_json::to_string_pretty(&state).expect("serialize creative state") + "\n",
    )?;
    Ok(())
}

/// Build the neutral MCP server entry for `tool`, given the resolved
/// absolute path to its bridge binary.
pub fn entry_for(tool: &CreativeTool, binary_path: &Path) -> McpServerEntry {
    McpServerEntry {
        key: tool.key.to_string(),
        command: binary_path.to_string_lossy().into_owned(),
        args: tool.default_args.iter().map(|a| (*a).to_string()).collect(),
    }
}

/// Rewrite every [`WORKSPACE_CLIENTS`] config in `workspace` so that
/// exactly `entries` (already resolved to real binary paths by the
/// caller) are the app-managed creative bridges: the enabled ones are
/// added, any catalog key not in `entries` is removed, and a server the
/// user added by hand is left alone.
///
/// A client whose config file does not exist is only created when there
/// is at least one entry to write for it — toggling a tool on and back
/// off does not litter empty `.mcp.json` files. Returns the paths that
/// were written (unchanged files are skipped).
pub fn apply_to_workspace(
    workspace: &Path,
    entries: &[McpServerEntry],
) -> Result<Vec<PathBuf>, HubError> {
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "creative-tool registration requires an absolute workspace path".into(),
        ));
    }
    let owned: Vec<&str> = CATALOG.iter().map(|t| t.key).collect();
    let mut written = Vec::new();
    for &client in WORKSPACE_CLIENTS {
        let rel = client
            .workspace_relative_config_path()
            .expect("WORKSPACE_CLIENTS are all workspace-scoped");
        let path = workspace.join(rel);
        let existed = path.exists();
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existed && entries.is_empty() {
            continue;
        }
        let rendered = render_replacing(client, &owned, entries, &existing);
        if existed && rendered == existing {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &rendered)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use tempfile::tempdir;

    fn blender_entry() -> McpServerEntry {
        entry_for(
            tool("coding-assistants-mcp-blender").unwrap(),
            Path::new("/opt/b"),
        )
    }
    fn krita_entry() -> McpServerEntry {
        entry_for(
            tool("coding-assistants-mcp-krita").unwrap(),
            Path::new("/opt/k"),
        )
    }

    #[test]
    fn catalog_keys_are_unique_and_match_binary_basenames() {
        let mut seen = BTreeSet::new();
        for t in CATALOG {
            assert!(seen.insert(t.key), "duplicate key {}", t.key);
            assert_eq!(t.key, t.binary, "key and binary basename should match");
        }
        assert_eq!(CATALOG.len(), 7);
    }

    #[test]
    fn enabled_set_round_trips_through_the_registry_file() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let ws = dir.path().join("proj");

        assert!(enabled_keys(&store, &ws).is_empty());

        let want: BTreeSet<String> = ["coding-assistants-mcp-blender".to_string()]
            .into_iter()
            .collect();
        set_enabled_keys(&store, &ws, &want).unwrap();
        assert_eq!(enabled_keys(&store, &ws), want);
    }

    #[test]
    fn unknown_keys_in_the_state_file_are_ignored() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let ws = dir.path().join("proj");
        let mut keys: BTreeSet<String> = BTreeSet::new();
        keys.insert("coding-assistants-mcp-blender".to_string());
        keys.insert("coding-assistants-mcp-obsolete".to_string());
        set_enabled_keys(&store, &ws, &keys).unwrap();
        assert_eq!(
            enabled_keys(&store, &ws),
            ["coding-assistants-mcp-blender".to_string()]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn apply_adds_then_removes_the_entry_and_spares_hand_added_servers() {
        let ws = tempdir().unwrap();
        let ws = ws.path();

        // Enable blender + krita.
        apply_to_workspace(ws, &[blender_entry(), krita_entry()]).unwrap();
        let mcp_json = ws.join(".mcp.json");
        let mut v: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        assert!(v["mcpServers"]["coding-assistants-mcp-blender"].is_object());
        assert!(v["mcpServers"]["coding-assistants-mcp-krita"].is_object());

        // A user adds their own server by hand.
        v["mcpServers"]["user-fs"] = json!({ "command": "npx", "args": ["-y", "@mcp/fs"] });
        std::fs::write(&mcp_json, serde_json::to_string_pretty(&v).unwrap()).unwrap();

        // Disable krita (blender still on).
        apply_to_workspace(ws, &[blender_entry()]).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        assert!(v["mcpServers"]["coding-assistants-mcp-blender"].is_object());
        assert!(
            v["mcpServers"]["coding-assistants-mcp-krita"].is_null(),
            "disabled bridge must be gone from .mcp.json"
        );
        assert_eq!(
            v["mcpServers"]["user-fs"]["command"], "npx",
            "hand-added server must survive"
        );

        // Disable everything — the file stays but our keys are gone.
        apply_to_workspace(ws, &[]).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        assert!(v["mcpServers"]["coding-assistants-mcp-blender"].is_null());
        assert_eq!(v["mcpServers"]["user-fs"]["command"], "npx");
    }

    #[test]
    fn apply_does_not_create_empty_config_files() {
        let ws = tempdir().unwrap();
        apply_to_workspace(ws.path(), &[]).unwrap();
        assert!(!ws.path().join(".mcp.json").exists());
        assert!(!ws.path().join("opencode.json").exists());
        assert!(!ws.path().join(".gemini/settings.json").exists());
    }

    #[test]
    fn apply_writes_every_workspace_client() {
        let ws = tempdir().unwrap();
        let written = apply_to_workspace(ws.path(), &[blender_entry()]).unwrap();
        assert_eq!(written.len(), WORKSPACE_CLIENTS.len());
        assert!(ws.path().join(".mcp.json").exists());
        assert!(ws.path().join(".gemini/settings.json").exists());
        assert!(ws.path().join("opencode.json").exists());
    }

    #[test]
    fn apply_rejects_a_relative_workspace() {
        assert!(apply_to_workspace(Path::new("rel/path"), &[]).is_err());
    }

    #[test]
    fn apply_is_idempotent() {
        let ws = tempdir().unwrap();
        apply_to_workspace(ws.path(), &[blender_entry()]).unwrap();
        let first = std::fs::read_to_string(ws.path().join(".mcp.json")).unwrap();
        let written = apply_to_workspace(ws.path(), &[blender_entry()]).unwrap();
        assert!(written.is_empty(), "second identical apply writes nothing");
        let second = std::fs::read_to_string(ws.path().join(".mcp.json")).unwrap();
        assert_eq!(first, second);
    }
}
