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
//! The rendered `command` is the launcher's bare name (`npx`,
//! `pwm-mcp`), resolved by the MCP client on its own `$PATH` at spawn —
//! unlike `creative`, which renders absolute paths to app-local
//! sidecars. The Tauri layer resolves `command` against `$PATH` only to
//! report `launcherFound` in the Settings status.
//!
//! #276 / #277 confirm the two Perplexity launchers against live
//! packages (`npx -y @perplexity-ai/mcp-server` 1.2.1; `pwm-mcp` from
//! `uv tool install perplexity-web-mcp-cli` 0.14.13) and own the
//! enable/disable apply tests plus Settings copy (`notes`, token-file
//! presence). Config write is this registry, not `pwm setup add`.

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
    /// - `SessionLogin` → `None` at this layer. [`ExternalServer::auth_configured`]
    ///   may additionally report token-*file presence* without opening the
    ///   file.
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
    /// Settings copy: how to authenticate, and any quota / expiry caveat.
    /// Never a secret. Empty string means the auth descriptor is enough.
    pub notes: &'static str,
    /// Home-relative path of a vendor session-token file, if any. Used
    /// only as an existence probe — the file is never opened or logged.
    pub auth_token_relpath: Option<&'static str>,
    /// Extra PATH names that count as "installed" for Settings
    /// `launcherFound` only. Never written into client config — that
    /// always uses [`command`](Self::command). `#277` also probes `pwm`
    /// because `uv tool install perplexity-web-mcp-cli` exposes that
    /// sibling console script next to `pwm-mcp`. `uvx` is **not** a
    /// probe: it is a generic runner and would be a false positive.
    pub launcher_aliases: &'static [&'static str],
}

impl ExternalServer {
    /// Presence hint for Settings. Never reads an API key or token file.
    ///
    /// Session-login servers report `Some(true)` only when
    /// [`auth_token_relpath`](Self::auth_token_relpath) names a file
    /// that exists under `$HOME`. Missing path or missing `$HOME` is
    /// `Some(false)`, not `None` — Gemini can show "run `pwm login`".
    pub fn auth_configured(&self) -> Option<bool> {
        match self.auth {
            AuthKind::SessionLogin { .. } => Some(match self.auth_token_relpath {
                Some(relpath) => match std::env::var_os("HOME") {
                    Some(home) => token_file_exists_in(Path::new(&home), relpath),
                    None => false,
                },
                None => false,
            }),
            other => other.configured(),
        }
    }

    /// Names the Settings layer probes on this process's `$PATH`.
    /// Config write still uses only [`command`](Self::command).
    pub fn launcher_probe_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.command).chain(self.launcher_aliases.iter().copied())
    }
}

/// `true` when `home/relpath` is a regular file. The path is never
/// opened — existence only, so a session token cannot leak into logs.
pub fn token_file_exists_in(home: &Path, relpath: &str) -> bool {
    home.join(relpath).is_file()
}

/// Every known external MCP server, in display order.
///
/// Launchers were confirmed 2026-09-08 against the live packages, not
/// guessed: official npm `@perplexity-ai/mcp-server` 1.2.1
/// (`npx -y @perplexity-ai/mcp-server`); subscription
/// `perplexity-web-mcp-cli` 0.14.13 (`pwm-mcp` console script from
/// `uv tool install`, docs `{ "command": "pwm-mcp" }`).
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
        notes: "Export PERPLEXITY_API_KEY in the shell that starts Claude Code / Gemini CLI. Coding Assistants never stores the key.",
        auth_token_relpath: None,
        launcher_aliases: &[],
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
        notes: "Quota-limited Perplexity subscription; the session token lasts ~30 days and then `pwm login` must be re-run. Coding Assistants never reads the token. Install: `uv tool install perplexity-web-mcp-cli`.",
        auth_token_relpath: Some(".config/perplexity-web-mcp/token"),
        launcher_aliases: &["pwm"],
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
#[path = "external_tests.rs"]
mod tests;
