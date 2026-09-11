//! P14 slice A — human-launched MCP tool invoke (direct-invoke-and-show).
//!
//! Lists enabled registry entries (external + creative), then spawns the
//! server as a stdio MCP client for `tools/list` / `tools/call`. Secrets
//! cross the child environment only; they never enter IPC, logs, or errors.

use hub::mcp::client::{self, McpCallResult, McpTool};
use hub::mcp::creative;
use hub::mcp::external::{self, AuthKind};
use hub::mcp::McpServerEntry;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::creative_tools::resolve_binary;
use super::store::open_store;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInvokeServer {
    pub key: String,
    pub display_name: String,
    pub kind: String,
    pub command: String,
    pub args: Vec<String>,
}

fn require_absolute(workspace: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(workspace);
    if !path.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    Ok(path)
}

fn extra_env_for(key: &str) -> Vec<(String, String)> {
    let Some(server) = external::server(key) else {
        return Vec::new();
    };
    match server.auth {
        AuthKind::ApiKey { env_var } => hub::secret::resolve(env_var)
            .filter(|secret| !secret.expose().trim().is_empty())
            .map(|secret| vec![(env_var.to_string(), secret.expose().to_string())])
            .unwrap_or_default(),
        AuthKind::None | AuthKind::SessionLogin { .. } => Vec::new(),
    }
}

fn with_env<T>(
    key: &str,
    f: impl FnOnce(&[(&str, &str)]) -> Result<T, String>,
) -> Result<T, String> {
    let owned = extra_env_for(key);
    let refs: Vec<(&str, &str)> = owned
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    f(&refs)
}

fn enabled_servers(workspace: &Path) -> Result<Vec<(McpInvokeServer, McpServerEntry)>, String> {
    let store = open_store()?;
    let mut out = Vec::new();

    let external_keys: BTreeSet<String> = external::enabled_keys(&store, workspace);
    for key in external_keys {
        let Some(server) = external::server(&key) else {
            continue;
        };
        let entry = external::entry_for(server);
        out.push((
            McpInvokeServer {
                key: server.key.to_string(),
                display_name: server.display_name.to_string(),
                kind: "external".into(),
                command: entry.command.clone(),
                args: entry.args.clone(),
            },
            entry,
        ));
    }

    let creative_keys: BTreeSet<String> = creative::enabled_keys(&store, workspace);
    for key in creative_keys {
        let Some(tool) = creative::tool(&key) else {
            continue;
        };
        let Some(path) = resolve_binary(tool.binary) else {
            continue;
        };
        let entry = creative::entry_for(tool, &path);
        out.push((
            McpInvokeServer {
                key: tool.key.to_string(),
                display_name: tool.display_name.to_string(),
                kind: "creative".into(),
                command: entry.command.clone(),
                args: entry.args.clone(),
            },
            entry,
        ));
    }
    Ok(out)
}

fn lookup_entry(workspace: &Path, key: &str) -> Result<McpServerEntry, String> {
    enabled_servers(workspace)?
        .into_iter()
        .find(|(row, _)| row.key == key)
        .map(|(_, entry)| entry)
        .ok_or_else(|| {
            format!("MCP server `{key}` is not enabled for this workspace (Settings → External MCP / Creative Tools)")
        })
}

#[tauri::command]
pub async fn hub_mcp_client_servers(workspace: String) -> Result<Vec<McpInvokeServer>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = require_absolute(&workspace)?;
        Ok(enabled_servers(&path)?
            .into_iter()
            .map(|(row, _)| row)
            .collect())
    })
    .await
    .map_err(|e| format!("MCP server list worker panicked: {e}"))?
}

#[tauri::command]
pub async fn hub_mcp_client_tools(workspace: String, key: String) -> Result<Vec<McpTool>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = require_absolute(&workspace)?;
        let entry = lookup_entry(&path, &key)?;
        with_env(&key, |env| client::list_tools(&entry, env))
    })
    .await
    .map_err(|e| format!("MCP tools/list worker panicked: {e}"))?
}

#[tauri::command]
pub async fn hub_mcp_client_call(
    workspace: String,
    key: String,
    tool: String,
    arguments: Value,
) -> Result<McpCallResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = require_absolute(&workspace)?;
        let entry = lookup_entry(&path, &key)?;
        with_env(&key, |env| {
            client::call_tool(&entry, env, &tool, &arguments)
        })
    })
    .await
    .map_err(|e| format!("MCP tools/call worker panicked: {e}"))?
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
    fn extra_env_skips_session_login_and_unknown_keys() {
        assert!(extra_env_for("perplexity-web").is_empty());
        assert!(extra_env_for("not-a-catalog-server").is_empty());
        for (key, _) in extra_env_for("perplexity") {
            assert_eq!(key, "PERPLEXITY_API_KEY");
        }
    }

    #[test]
    fn lookup_names_disabled_servers() {
        use super::super::tests::CA_HOME_ENV_LOCK;

        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("CA_HOME");
        std::env::set_var("CA_HOME", home.path());
        let err = lookup_entry(Path::new("/tmp/ca-p14-ws"), "perplexity").unwrap_err();
        match previous {
            Some(value) => std::env::set_var("CA_HOME", value),
            None => std::env::remove_var("CA_HOME"),
        }
        assert!(err.contains("not enabled"), "{err}");
        assert!(err.contains("perplexity"), "{err}");
    }
}
