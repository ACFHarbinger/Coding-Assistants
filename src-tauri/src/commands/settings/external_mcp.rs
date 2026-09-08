//! Track P9 — the Settings "External MCP servers" IPC surface.
//!
//! The generic twin of [`creative_tools`](super::creative_tools): it
//! reports per-workspace enable / install / auth status and drives
//! `hub::mcp::external` to write the enabled servers into a workspace's
//! Claude / Gemini / opencode MCP configs.
//!
//! `launcherFound` is **informational**. The MCP client (Claude Code,
//! Gemini CLI, …) resolves the bare command on *its* `$PATH`, which may
//! differ from this desktop process — so a missing `npx` / `pwm-mcp`
//! here must not omit an enabled server from the written config.
//!
//! `authConfigured` is a **presence hint**, not verification: for an
//! API-key server it means the variable is set in *this* process's
//! environment; the MCP client spawns the server as its own child and
//! may export something different. For a session-login server it is
//! `true` only when the vendor token *file* exists (existence probe,
//! contents never read). `notes` carries the Settings copy (#278).
//!
//! Codex is intentionally not written here (its config is user-global),
//! matching `creative_tools`.

use hub::mcp::external::{self, ExternalServer};
use hub::mcp::McpServerEntry;
use std::collections::BTreeSet;
use std::path::PathBuf;

use super::creative_tools::resolve_binary;
use super::store::open_store;

/// One external server as the Settings tab sees it: static catalog data
/// plus this machine's resolution of it.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalServerStatus {
    pub key: String,
    pub display_name: String,
    pub docs_url: String,
    /// `{ "kind": "none" | "api_key" | "session_login", .. }`.
    pub auth: serde_json::Value,
    /// Presence hint. API-key: env var set in this process. Session-login:
    /// vendor token file exists. Never a guarantee the spawned server
    /// will authenticate, and never the secret itself.
    pub auth_configured: Option<bool>,
    /// Quota / setup copy for the Settings tab. Never a secret.
    pub notes: String,
    /// Informational: the launcher was found on *this* process's `$PATH`.
    /// Not a write gate — the MCP client may resolve a different PATH.
    pub launcher_found: bool,
    pub launcher_path: Option<String>,
    /// `true` when this server is in the workspace's enabled set.
    pub enabled: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMcpStatus {
    pub workspace: String,
    pub servers: Vec<ExternalServerStatus>,
    /// Config files the last `set_enabled` / `reapply` call wrote (empty
    /// for a plain status read).
    pub written_configs: Vec<String>,
}

fn require_absolute(workspace: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(workspace);
    if !path.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    Ok(path)
}

fn probe_launcher(srv: &ExternalServer) -> Option<PathBuf> {
    srv.launcher_probe_names().find_map(resolve_binary)
}

fn status_row(srv: &ExternalServer, enabled: &BTreeSet<String>) -> ExternalServerStatus {
    let resolved = probe_launcher(srv);
    ExternalServerStatus {
        key: srv.key.to_string(),
        display_name: srv.display_name.to_string(),
        docs_url: srv.docs_url.to_string(),
        auth: serde_json::to_value(srv.auth).unwrap_or(serde_json::Value::Null),
        auth_configured: srv.auth_configured(),
        notes: srv.notes.to_string(),
        launcher_found: resolved.is_some(),
        launcher_path: resolved.map(|p| p.to_string_lossy().into_owned()),
        enabled: enabled.contains(srv.key),
    }
}

fn build_status(
    workspace: &std::path::Path,
    enabled: &BTreeSet<String>,
    written_configs: Vec<String>,
) -> ExternalMcpStatus {
    ExternalMcpStatus {
        workspace: workspace.to_string_lossy().into_owned(),
        servers: external::CATALOG
            .iter()
            .map(|srv| status_row(srv, enabled))
            .collect(),
        written_configs,
    }
}

/// One `McpServerEntry` per enabled catalog key. Always the bare
/// launcher name (`npx`, `pwm-mcp`) — never gated on
/// [`resolve_binary`], which only feeds `launcherFound` for Settings.
fn enabled_entries(enabled: &BTreeSet<String>) -> Vec<McpServerEntry> {
    enabled
        .iter()
        .filter_map(|key| external::server(key))
        .map(external::entry_for)
        .collect()
}

/// Read-only: per-workspace status of every catalog server.
#[tauri::command]
pub async fn external_mcp_status(workspace: String) -> Result<ExternalMcpStatus, String> {
    tauri::async_runtime::spawn_blocking(move || external_mcp_status_blocking(workspace))
        .await
        .map_err(|error| format!("external_mcp_status worker panic: {error}"))?
}

pub(crate) fn external_mcp_status_blocking(workspace: String) -> Result<ExternalMcpStatus, String> {
    let path = require_absolute(&workspace)?;
    let store = open_store()?;
    let enabled = external::enabled_keys(&store, &path);
    Ok(build_status(&path, &enabled, Vec::new()))
}

/// Toggle one server for a workspace: update the app-owned registry,
/// then rewrite the workspace's Claude / Gemini / opencode MCP configs.
#[tauri::command]
pub async fn external_mcp_set_enabled(
    workspace: String,
    key: String,
    enabled: bool,
) -> Result<ExternalMcpStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        external_mcp_set_enabled_blocking(workspace, key, enabled)
    })
    .await
    .map_err(|error| format!("external_mcp_set_enabled worker panic: {error}"))?
}

pub(crate) fn external_mcp_set_enabled_blocking(
    workspace: String,
    key: String,
    enabled: bool,
) -> Result<ExternalMcpStatus, String> {
    if external::server(&key).is_none() {
        return Err(format!("unknown external MCP server: {key}"));
    }
    let path = require_absolute(&workspace)?;
    let store = open_store()?;

    let mut keys = external::enabled_keys(&store, &path);
    if enabled {
        keys.insert(key);
    } else {
        keys.remove(&key);
    }
    external::set_enabled_keys(&store, &path, &keys).map_err(|e| e.to_string())?;

    let written = external::apply_to_workspace(&path, &enabled_entries(&keys))
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(build_status(&path, &keys, written))
}

/// Re-apply the current enabled set — for the tab's "Re-apply" action
/// after a launcher is installed or the config files were hand-edited.
#[tauri::command]
pub async fn external_mcp_reapply(workspace: String) -> Result<ExternalMcpStatus, String> {
    tauri::async_runtime::spawn_blocking(move || external_mcp_reapply_blocking(workspace))
        .await
        .map_err(|error| format!("external_mcp_reapply worker panic: {error}"))?
}

pub(crate) fn external_mcp_reapply_blocking(
    workspace: String,
) -> Result<ExternalMcpStatus, String> {
    let path = require_absolute(&workspace)?;
    let store = open_store()?;
    let keys = external::enabled_keys(&store, &path);
    let written = external::apply_to_workspace(&path, &enabled_entries(&keys))
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(build_status(&path, &keys, written))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_absolute_rejects_relative() {
        assert!(require_absolute("rel/x").is_err());
        assert!(require_absolute("/abs/x").is_ok());
    }

    #[test]
    fn status_reports_every_catalog_server_with_auth_and_enabled_flags() {
        let dir = tempfile::tempdir().unwrap();
        let enabled: BTreeSet<String> = ["perplexity".to_string()].into_iter().collect();
        let status = build_status(dir.path(), &enabled, Vec::new());
        assert_eq!(status.servers.len(), external::CATALOG.len());

        let api = status
            .servers
            .iter()
            .find(|s| s.key == "perplexity")
            .unwrap();
        assert!(api.enabled);
        assert_eq!(api.auth["kind"], "api_key");

        let web = status
            .servers
            .iter()
            .find(|s| s.key == "perplexity-web")
            .unwrap();
        assert!(!web.enabled);
        assert_eq!(web.auth["kind"], "session_login");
        assert!(
            web.auth_configured.is_some(),
            "session-login reports token-file presence, not null"
        );
        assert!(web.notes.contains("Quota-limited"));
        assert!(api.notes.contains("PERPLEXITY_API_KEY"));
    }

    #[test]
    fn enabling_writes_bare_command_even_when_this_process_path_hides_it() {
        use crate::commands::commands::tests::CA_HOME_ENV_LOCK;
        use serde_json::{json, Value};

        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("CA_HOME", home.path());
        let old_path = std::env::var_os("PATH");
        std::env::set_var("PATH", "");

        for (key, command) in [("perplexity", "npx"), ("perplexity-web", "pwm-mcp")] {
            let ws = tempfile::tempdir().unwrap();
            let mcp = ws.path().join(".mcp.json");
            std::fs::write(
                &mcp,
                serde_json::to_string_pretty(&json!({
                    "mcpServers": { "user-fs": { "command": "echo", "args": ["ok"] } }
                }))
                .unwrap(),
            )
            .unwrap();
            let ws_s = ws.path().to_string_lossy().into_owned();

            let on =
                external_mcp_set_enabled_blocking(ws_s.clone(), key.into(), true).expect("enable");
            assert!(on.servers.iter().any(|s| s.key == key && s.enabled));
            let v: Value = serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
            assert_eq!(
                v["mcpServers"][key]["command"], command,
                "enabled {key} must be written even if this process cannot resolve {command}"
            );
            assert_eq!(v["mcpServers"]["user-fs"]["command"], "echo");

            external_mcp_set_enabled_blocking(ws_s, key.into(), false).expect("disable");
            let v: Value = serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
            assert!(v["mcpServers"][key].is_null());
            assert_eq!(v["mcpServers"]["user-fs"]["command"], "echo");
        }

        match old_path {
            Some(path) => std::env::set_var("PATH", path),
            None => std::env::remove_var("PATH"),
        }
        std::env::remove_var("CA_HOME");
    }

    #[test]
    fn subscription_launcher_probe_includes_pwm_and_uvx() {
        let web = external::server("perplexity-web").unwrap();
        let names: Vec<_> = web.launcher_probe_names().collect();
        assert_eq!(names, ["pwm-mcp", "pwm", "uvx"]);
        assert_eq!(
            entry_for_command(web),
            "pwm-mcp",
            "client config still writes pwm-mcp, not uvx"
        );
    }

    fn entry_for_command(srv: &external::ExternalServer) -> String {
        external::entry_for(srv).command
    }

    #[test]
    fn enabled_entries_are_not_gated_on_this_process_path() {
        let keys: BTreeSet<String> = external::CATALOG
            .iter()
            .map(|s| s.key.to_string())
            .collect();
        let entries = enabled_entries(&keys);
        assert_eq!(entries.len(), keys.len());
        assert!(entries
            .iter()
            .any(|e| e.key == "perplexity" && e.command == "npx"));
        assert!(entries
            .iter()
            .any(|e| e.key == "perplexity-web" && e.command == "pwm-mcp"));
        assert!(
            entries
                .iter()
                .all(|e| !e.command.contains('/') && !e.command.contains('\\')),
            "written command must stay a bare name the MCP client resolves"
        );
    }

    #[test]
    fn set_enabled_rejects_an_unknown_key() {
        match external_mcp_set_enabled_blocking("/tmp/ws".into(), "nope".into(), true) {
            Err(err) => assert!(err.contains("unknown external MCP server"), "{err}"),
            Ok(_) => panic!("an unknown key must be rejected"),
        }
    }

    #[test]
    fn set_enabled_adds_then_removes_each_key_and_spares_hand_added() {
        use crate::commands::commands::tests::CA_HOME_ENV_LOCK;
        use serde_json::{json, Value};

        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("CA_HOME", home.path());

        let ws = tempfile::tempdir().unwrap();
        let mcp = ws.path().join(".mcp.json");
        std::fs::write(
            &mcp,
            serde_json::to_string_pretty(&json!({
                "mcpServers": { "user-fs": { "command": "npx", "args": ["-y", "@mcp/fs"] } }
            }))
            .unwrap(),
        )
        .unwrap();
        let ws_s = ws.path().to_string_lossy().into_owned();

        for key in ["perplexity", "perplexity-web"] {
            let on =
                external_mcp_set_enabled_blocking(ws_s.clone(), key.into(), true).expect("enable");
            let row = on.servers.iter().find(|s| s.key == key).unwrap();
            assert!(
                row.enabled,
                "{key} must stay enabled even if its launcher is missing"
            );
            let v: Value = serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
            assert!(
                v["mcpServers"][key].is_object(),
                "{key} must be written even when this process cannot see the launcher"
            );
            assert!(v["mcpServers"][key].get("env").is_none());

            let off = external_mcp_set_enabled_blocking(ws_s.clone(), key.into(), false)
                .expect("disable");
            assert!(off.servers.iter().any(|s| s.key == key && !s.enabled));

            let v: Value = serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
            assert!(
                v["mcpServers"][key].is_null(),
                "disabled {key} must not remain in .mcp.json"
            );
            assert_eq!(
                v["mcpServers"]["user-fs"]["command"], "npx",
                "hand-added server must survive {key} toggle"
            );
        }

        std::env::remove_var("CA_HOME");
    }
}
