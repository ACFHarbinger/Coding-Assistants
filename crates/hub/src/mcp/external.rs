//! Track P9 — per-workspace registration of arbitrary external MCP servers.
//!
//! [`creative`](crate::mcp::creative) onboards the in-tree
//! `crates/mcp-<tool>` bridges; this module is its generalisation for
//! *third-party* stdio MCP servers the user installs themselves (an
//! `npx` package, a `uv`-installed CLI, …). It owns:
//!
//! - [`CATALOG`] — static metadata for each known server (key, the
//!   launcher command + args, and an [`AuthKind`] descriptor).
//! - The per-workspace **enabled set**, stored next to the Channel
//!   registry under `servers_dir(store)` as `<name>.external.json`
//!   (deliberately a different suffix from `creative`'s
//!   `<name>.creative.json` so the two registries never share a file).
//! - [`apply_to_workspace`] — rewrites each client's MCP config so
//!   exactly the enabled external servers are registered.
//!
//! The two registries compose because each only ever passes *its own*
//! catalog keys as the `owned` set to
//! [`render_replacing`](crate::mcp::render_replacing): a `creative`
//! apply never prunes an `external` key and vice-versa. `CATALOG` keys
//! are asserted disjoint from `creative::CATALOG` in a test.
//!
//! Launcher-path resolution is **not** done here — the Tauri command
//! layer resolves `command` against `$PATH` and passes a ready
//! [`McpServerEntry`] list in, exactly as it does for `creative`.

use crate::mcp::{render_replacing, ClientKind, McpServerEntry};
use crate::{servers_dir, workspace_server_name, HubError, HubStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// How an external server authenticates. Informational: the app never
/// performs the login or injects the key — it only reports what it can
/// see so the Settings tab can tell the user what is still needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthKind {
    /// No credential required.
    None,
    /// Reads an API key from an environment variable at spawn time.
    ApiKey { env_var: &'static str },
    /// A one-off interactive login that mints a local session token;
    /// `setup_cmd` is the command the user runs (e.g. `pwm login`).
    SessionLogin { setup_cmd: &'static str },
}

impl AuthKind {
    /// Best-effort answer to "is this server's credential in place?".
    ///
    /// - `None` → always `Some(true)`.
    /// - `ApiKey` → `Some(true)` when the variable is set **in the app
    ///   process's own environment**. The MCP client spawns the server
    ///   as *its* child and may export a different environment, so this
    ///   is a presence hint, not a guarantee the server will
    ///   authenticate.
    /// - `SessionLogin` → `None`: the token lives wherever the vendor
    ///   CLI puts it and the hub does not probe for it.
    pub fn configured(&self) -> Option<bool> {
        match self {
            AuthKind::None => Some(true),
            AuthKind::ApiKey { env_var } => {
                Some(std::env::var(env_var).is_ok_and(|v| !v.trim().is_empty()))
            }
            AuthKind::SessionLogin { .. } => None,
        }
    }
}

/// Static description of one third-party MCP server.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ExternalServer {
    /// Stable config-map key, identical across every client.
    pub key: &'static str,
    /// Human label ("Perplexity (API)").
    pub display_name: &'static str,
    /// Launcher command written into the client config **verbatim**
    /// (`npx`, `pwm-mcp`, …) — a bare name the MCP client resolves on
    /// *its own* `$PATH` at spawn. Deliberately not rendered as an
    /// absolute path: pinning e.g. one `nvm` Node's `npx` would break on
    /// the user's next `nvm use`. The Settings layer resolves it against
    /// `$PATH` separately, only to report `launcherFound`.
    pub command: &'static str,
    /// Args appended after the launcher.
    pub default_args: &'static [&'static str],
    pub auth: AuthKind,
    /// Upstream docs, surfaced by the UI so the user can follow setup.
    pub docs_url: &'static str,
}

/// Every known external MCP server, in display order.
///
/// #272 ships the mechanism plus these two rows; #276 / #277 own
/// launcher resolution, the apply flow, and the Settings integration.
pub const CATALOG: &[ExternalServer] = &[
    ExternalServer {
        key: "perplexity",
        display_name: "Perplexity (API)",
        command: "npx",
        default_args: &["-y", "@perplexity-ai/mcp-server"],
        auth: AuthKind::ApiKey {
            env_var: "PERPLEXITY_API_KEY",
        },
        docs_url: "https://github.com/perplexityai/modelcontextprotocol",
    },
    ExternalServer {
        key: "perplexity-web",
        display_name: "Perplexity (subscription)",
        command: "pwm-mcp",
        default_args: &[],
        auth: AuthKind::SessionLogin {
            setup_cmd: "pwm login",
        },
        docs_url: "https://github.com/jacob-bd/perplexity-web-mcp",
    },
];

/// Look up a catalog entry by its stable key.
pub fn server(key: &str) -> Option<&'static ExternalServer> {
    CATALOG.iter().find(|s| s.key == key)
}

/// The client CLIs a per-workspace registration writes to — identical to
/// [`creative::WORKSPACE_CLIENTS`](crate::mcp::creative::WORKSPACE_CLIENTS)
/// and Codex is excluded for the same reason (its config is user-global).
pub const WORKSPACE_CLIENTS: &[ClientKind] =
    &[ClientKind::Claude, ClientKind::Gemini, ClientKind::Opencode];

fn state_path(store: &HubStore, workspace: &Path) -> PathBuf {
    servers_dir(store).join(format!(
        "{}.external.json",
        workspace_server_name(workspace)
    ))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StateFile {
    #[serde(default)]
    enabled: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workspace: Option<String>,
}

/// The server keys currently enabled for `workspace` (empty if never
/// set). Keys with no catalog entry are dropped.
pub fn enabled_keys(store: &HubStore, workspace: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(state_path(store, workspace))
        .ok()
        .and_then(|raw| serde_json::from_str::<StateFile>(&raw).ok())
        .map(|state| {
            state
                .enabled
                .into_iter()
                .filter(|key| server(key).is_some())
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
        serde_json::to_string_pretty(&state).expect("serialize external state") + "\n",
    )?;
    Ok(())
}

/// Build the neutral MCP server entry for `server`. `command` is written
/// verbatim (a bare `npx` / `pwm-mcp` the client resolves on its own
/// `$PATH`) — see [`ExternalServer::command`].
pub fn entry_for(server: &ExternalServer) -> McpServerEntry {
    McpServerEntry {
        key: server.key.to_string(),
        command: server.command.to_string(),
        args: server
            .default_args
            .iter()
            .map(|a| (*a).to_string())
            .collect(),
    }
}

/// Rewrite every [`WORKSPACE_CLIENTS`] config in `workspace` so that
/// exactly `entries` are the app-managed external servers: enabled ones
/// are added, any [`CATALOG`] key not in `entries` is removed, and a
/// server the user (or the `creative` registry) added is left alone.
///
/// A client whose config file does not exist is only created when there
/// is at least one entry to write for it. Returns the paths written.
pub fn apply_to_workspace(
    workspace: &Path,
    entries: &[McpServerEntry],
) -> Result<Vec<PathBuf>, HubError> {
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "external MCP registration requires an absolute workspace path".into(),
        ));
    }
    let owned: Vec<&str> = CATALOG.iter().map(|s| s.key).collect();
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
    use crate::mcp::creative;
    use serde_json::{json, Value};
    use tempfile::tempdir;

    fn perplexity_entry() -> McpServerEntry {
        entry_for(server("perplexity").unwrap())
    }

    #[test]
    fn catalog_keys_are_unique_and_disjoint_from_creative() {
        let mut seen = BTreeSet::new();
        for s in CATALOG {
            assert!(seen.insert(s.key), "duplicate external key {}", s.key);
        }
        for c in creative::CATALOG {
            assert!(
                !seen.contains(c.key),
                "external key {} collides with a creative-tool key; the two \
                 registries must own disjoint namespaces",
                c.key
            );
        }
    }

    #[test]
    fn auth_configured_reports_presence_and_unknown() {
        assert_eq!(AuthKind::None.configured(), Some(true));
        assert_eq!(AuthKind::SessionLogin { setup_cmd: "x" }.configured(), None);
        // A definitionally-unset var — no `set_var` here: mutating the
        // environment races every other test thread's `getenv`.
        assert_eq!(
            AuthKind::ApiKey {
                env_var: "CA_EXTERNAL_MCP_DEFINITELY_UNSET",
            }
            .configured(),
            Some(false)
        );
        // The present-and-non-empty branch: `PATH` is always set.
        assert_eq!(
            AuthKind::ApiKey { env_var: "PATH" }.configured(),
            Some(true)
        );
    }

    #[test]
    fn enabled_set_round_trips_and_drops_unknown_keys() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let ws = dir.path().join("proj");
        assert!(enabled_keys(&store, &ws).is_empty());

        let mut keys: BTreeSet<String> = BTreeSet::new();
        keys.insert("perplexity".to_string());
        keys.insert("not-a-real-server".to_string());
        set_enabled_keys(&store, &ws, &keys).unwrap();
        assert_eq!(
            enabled_keys(&store, &ws),
            ["perplexity".to_string()].into_iter().collect()
        );
    }

    #[test]
    fn external_state_file_uses_its_own_suffix() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let ws = dir.path().join("proj");
        let keys: BTreeSet<String> = ["perplexity".to_string()].into_iter().collect();
        set_enabled_keys(&store, &ws, &keys).unwrap();
        assert!(state_path(&store, &ws)
            .to_string_lossy()
            .ends_with(".external.json"));
    }

    #[test]
    fn apply_adds_then_removes_only_its_own_keys() {
        let ws = tempdir().unwrap();
        let ws = ws.path();
        apply_to_workspace(ws, &[perplexity_entry()]).unwrap();
        let mcp_json = ws.join(".mcp.json");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        assert_eq!(
            v["mcpServers"]["perplexity"]["command"], "npx",
            "PATH launchers render as a bare command, not an absolute path"
        );
        assert_eq!(
            v["mcpServers"]["perplexity"]["args"],
            json!(["-y", "@perplexity-ai/mcp-server"])
        );

        apply_to_workspace(ws, &[]).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        assert!(v["mcpServers"]["perplexity"].is_null());
    }

    /// The load-bearing contract for #276/#277: an `external` apply must
    /// not disturb a `creative` registration and vice-versa, in either
    /// order.
    #[test]
    fn external_and_creative_registries_compose_without_clobbering() {
        let ws = tempdir().unwrap();
        let ws = ws.path();
        let blender = creative::entry_for(
            creative::tool("coding-assistants-mcp-blender").unwrap(),
            Path::new("/opt/blender-bridge"),
        );

        // creative first, then external.
        creative::apply_to_workspace(ws, std::slice::from_ref(&blender)).unwrap();
        apply_to_workspace(ws, &[perplexity_entry()]).unwrap();
        let read = |p: &Path| -> Value {
            serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
        };
        let mcp_json = ws.join(".mcp.json");
        let v = read(&mcp_json);
        assert!(v["mcpServers"]["coding-assistants-mcp-blender"].is_object());
        assert!(v["mcpServers"]["perplexity"].is_object());

        // Disable external — blender must survive.
        apply_to_workspace(ws, &[]).unwrap();
        let v = read(&mcp_json);
        assert!(
            v["mcpServers"]["coding-assistants-mcp-blender"].is_object(),
            "creative entry must survive an external teardown"
        );
        assert!(v["mcpServers"]["perplexity"].is_null());

        // Re-enable external, then disable creative — external survives.
        apply_to_workspace(ws, &[perplexity_entry()]).unwrap();
        creative::apply_to_workspace(ws, &[]).unwrap();
        let v = read(&mcp_json);
        assert!(
            v["mcpServers"]["perplexity"].is_object(),
            "external entry must survive a creative teardown"
        );
        assert!(v["mcpServers"]["coding-assistants-mcp-blender"].is_null());
    }

    #[test]
    fn apply_rejects_a_relative_workspace() {
        assert!(apply_to_workspace(Path::new("rel/path"), &[]).is_err());
    }

    #[test]
    fn apply_is_idempotent() {
        let ws = tempdir().unwrap();
        apply_to_workspace(ws.path(), &[perplexity_entry()]).unwrap();
        let first = std::fs::read_to_string(ws.path().join(".mcp.json")).unwrap();
        let written = apply_to_workspace(ws.path(), &[perplexity_entry()]).unwrap();
        assert!(written.is_empty(), "second identical apply writes nothing");
        let second = std::fs::read_to_string(ws.path().join(".mcp.json")).unwrap();
        assert_eq!(first, second);
    }
}
