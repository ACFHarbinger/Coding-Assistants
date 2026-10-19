//! P14 slice A — human-launched MCP tool invoke (direct-invoke-and-show).
//!
//! Lists enabled registry entries (external + creative), then spawns the
//! server as a stdio MCP client for `tools/list` / `tools/call`. Secrets
//! cross the child environment only; they never enter IPC, logs, or errors.

use hub::mcp::client::{self, McpCallResult, McpTool};
use hub::mcp::creative;
use hub::mcp::external::{self, AuthKind};
use hub::mcp::McpServerEntry;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn invoke_process_json(
    workspace: &Path,
    server: &str,
    tool: &str,
    outcome: &str,
    duration: Duration,
) -> Value {
    json!({
        "kind": "mcp.invoke",
        "server": server,
        "tool": tool,
        "workspace": workspace.to_string_lossy(),
        "actor": "human",
        "outcome": outcome,
        "duration_ms": duration_ms(duration),
    })
}

fn invoke_outcome(result: &Result<McpCallResult, String>) -> &'static str {
    match result {
        Ok(call) if call.is_error => "tool_error",
        Ok(_) => "ok",
        Err(_) => "error",
    }
}

fn record_invoke(
    workspace: &Path,
    server: &str,
    tool: &str,
    result: &Result<McpCallResult, String>,
    duration: Duration,
) -> Result<(), String> {
    let store = open_store()?;
    let payload = invoke_process_json(workspace, server, tool, invoke_outcome(result), duration);
    let event = store
        .record_audit_event(
            workspace,
            workspace,
            "mcp.invoke",
            &payload.to_string(),
            None,
        )
        .map_err(|error| error.to_string())?;
    store
        .set_audit_status(&event.id, "approved")
        .map_err(|error| error.to_string())
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
        let started = Instant::now();
        let result = with_env(&key, |env| {
            client::call_tool(&entry, env, &tool, &arguments)
        });
        let duration = started.elapsed();
        // Visibility only: the call already happened, so audit it even when
        // the server or transport failed. Arguments and payloads never enter
        // this row.
        record_invoke(&path, &key, &tool, &result, duration)?;
        result
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

    #[test]
    fn invoke_payload_carries_metadata_but_not_arguments() {
        let value = invoke_process_json(
            Path::new("/tmp/ca-p14-ws"),
            "perplexity",
            "search",
            "ok",
            Duration::from_millis(12),
        );
        assert_eq!(value["kind"], "mcp.invoke");
        assert_eq!(value["server"], "perplexity");
        assert_eq!(value["tool"], "search");
        assert_eq!(value["workspace"], "/tmp/ca-p14-ws");
        assert_eq!(value["actor"], "human");
        assert_eq!(value["outcome"], "ok");
        assert_eq!(value["duration_ms"], 12);
        assert!(value.get("arguments").is_none());
    }

    #[test]
    fn record_invoke_writes_approved_metadata_rows() {
        use super::super::tests::CA_HOME_ENV_LOCK;

        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("CA_HOME");
        std::env::set_var("CA_HOME", home.path());
        let workspace = Path::new("/tmp/ca-p14-audit");

        record_invoke(
            workspace,
            "perplexity",
            "search",
            &Ok(McpCallResult {
                is_error: false,
                text: "SECRET-PAYLOAD".into(),
            }),
            Duration::from_millis(12),
        )
        .unwrap();
        record_invoke(
            workspace,
            "perplexity",
            "search",
            &Ok(McpCallResult {
                is_error: true,
                text: "SECRET-PAYLOAD".into(),
            }),
            Duration::from_millis(7),
        )
        .unwrap();
        record_invoke(
            workspace,
            "perplexity",
            "search",
            &Err("transport failed".into()),
            Duration::from_millis(3),
        )
        .unwrap();

        let events = open_store().unwrap().list_audit_events(false).unwrap();
        match previous {
            Some(value) => std::env::set_var("CA_HOME", value),
            None => std::env::remove_var("CA_HOME"),
        }

        assert_eq!(events.len(), 3);
        for event in &events {
            assert_eq!(event.operation, "mcp.invoke");
            assert_eq!(event.status, "approved");
            assert!(!event.process_json.contains("SECRET-PAYLOAD"));
        }
        assert!(events[0].process_json.contains("\"outcome\":\"ok\""));
        assert!(events[1]
            .process_json
            .contains("\"outcome\":\"tool_error\""));
        assert!(events[2].process_json.contains("\"outcome\":\"error\""));
        assert!(events[0].process_json.contains("\"actor\":\"human\""));
    }
}
