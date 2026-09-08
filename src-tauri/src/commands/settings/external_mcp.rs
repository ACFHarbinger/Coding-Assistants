//! Track P9 — the Settings "External MCP servers" IPC surface.
//!
//! The generic twin of [`creative_tools`](super::creative_tools): it
//! resolves each catalog server's launcher (`npx`, `pwm-mcp`, …) against
//! `$PATH`, reports per-workspace enable / install / auth status, and
//! drives `hub::mcp::external` to write the enabled servers into a
//! workspace's Claude / Gemini / opencode MCP configs.
//!
//! `authConfigured` is a **presence hint**, not verification: for an
//! API-key server it means the variable is set in *this* process's
//! environment; the MCP client spawns the server as its own child and
//! may export something different. For a session-login server it is
//! `null` — the token lives wherever the vendor CLI put it.
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
    /// `true` / `false` for an API-key server (presence in this
    /// process's env), `null` for a session-login server. Never a
    /// guarantee the spawned server will authenticate.
    pub auth_configured: Option<bool>,
    /// `true` when the launcher (`npx`, `pwm-mcp`, …) was found on
    /// `$PATH` or next to the app.
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

fn status_row(srv: &ExternalServer, enabled: &BTreeSet<String>) -> ExternalServerStatus {
    let resolved = resolve_binary(srv.command);
    ExternalServerStatus {
        key: srv.key.to_string(),
        display_name: srv.display_name.to_string(),
        docs_url: srv.docs_url.to_string(),
        auth: serde_json::to_value(srv.auth).unwrap_or(serde_json::Value::Null),
        auth_configured: srv.auth.configured(),
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

/// One `McpServerEntry` per enabled key whose launcher was found on
/// `$PATH`. A key with a missing launcher stays enabled (so the toggle
/// reads back on) but is not written into a config pointing at nothing.
/// The entry's `command` is the bare launcher name, not the resolved
/// absolute path — see [`external::ExternalServer::command`].
fn resolved_entries(enabled: &BTreeSet<String>) -> Vec<McpServerEntry> {
    enabled
        .iter()
        .filter_map(|key| external::server(key))
        .filter(|srv| resolve_binary(srv.command).is_some())
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

    let written = external::apply_to_workspace(&path, &resolved_entries(&keys))
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
    let written = external::apply_to_workspace(&path, &resolved_entries(&keys))
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
        assert_eq!(
            web.auth_configured, None,
            "session-login auth is unknowable"
        );
    }

    #[test]
    fn set_enabled_rejects_an_unknown_key() {
        match external_mcp_set_enabled_blocking("/tmp/ws".into(), "nope".into(), true) {
            Err(err) => assert!(err.contains("unknown external MCP server"), "{err}"),
            Ok(_) => panic!("an unknown key must be rejected"),
        }
    }
}
