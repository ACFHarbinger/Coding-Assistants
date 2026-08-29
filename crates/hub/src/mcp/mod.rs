//! Client-agnostic rendering of app-managed MCP server entries.
//!
//! Track C-1b of the creative-tool MCP program. Every `crates/mcp-<tool>`
//! bridge is delivered to an agent by writing a `command`-type server entry
//! into that agent's own MCP config — but each CLI has a different config
//! file *and shape*:
//!
//! | Client    | File                          | Shape |
//! |-----------|-------------------------------|-------|
//! | Claude    | `<workspace>/.mcp.json`       | `mcpServers.<key> = { command, args }` |
//! | Gemini    | `<workspace>/.gemini/settings.json` | same `mcpServers` shape |
//! | Codex     | `~/.codex/config.toml`        | `[mcp_servers.<key>]` table |
//! | opencode  | `<workspace>/opencode.json`   | `mcp.<key> = { type:"local", command:[cmd, ..args], enabled }` |
//!
//! This module owns [`McpServerEntry`] (the neutral form) and a renderer +
//! idempotent merge per [`ClientKind`]. The existing Claude Channel
//! registry (`bridge::channels::claude::workspaces`) keeps working
//! unchanged; it can move onto this later.

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// One app-managed MCP server, in the neutral form. `key` is the stable
/// identifier used as the config map key across every client.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpServerEntry {
    pub key: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

impl McpServerEntry {
    pub fn new(key: impl Into<String>, command: impl Into<String>, args: &[&str]) -> Self {
        Self {
            key: key.into(),
            command: command.into(),
            args: args.iter().map(|a| (*a).to_string()).collect(),
        }
    }
}

/// The MCP-client CLIs Coding-Assistants can hand a server to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientKind {
    /// Claude Code — `<workspace>/.mcp.json`.
    Claude,
    /// Gemini CLI — `<workspace>/.gemini/settings.json` (same shape as Claude).
    Gemini,
    /// Codex CLI — `~/.codex/config.toml`, `[mcp_servers.<key>]`.
    Codex,
    /// opencode — `<workspace>/opencode.json`, `mcp.<key>`.
    Opencode,
}

impl ClientKind {
    /// Config path relative to a workspace root, or `None` when the client's
    /// config is global (Codex) — the caller resolves that against `$HOME`.
    pub fn workspace_relative_config_path(self) -> Option<&'static str> {
        match self {
            ClientKind::Claude => Some(".mcp.json"),
            ClientKind::Gemini => Some(".gemini/settings.json"),
            ClientKind::Opencode => Some("opencode.json"),
            ClientKind::Codex => None,
        }
    }

    /// Global config path relative to `$HOME`, for clients that have one.
    pub fn home_relative_config_path(self) -> Option<&'static str> {
        match self {
            ClientKind::Codex => Some(".codex/config.toml"),
            _ => None,
        }
    }
}

fn mcp_servers_shape(entries: &[McpServerEntry]) -> Value {
    let mut servers = Map::new();
    for e in entries {
        servers.insert(
            e.key.clone(),
            json!({ "command": e.command, "args": e.args }),
        );
    }
    json!({ "mcpServers": servers })
}

fn opencode_shape(entries: &[McpServerEntry]) -> Value {
    let mut mcp = Map::new();
    for e in entries {
        let mut command = vec![Value::String(e.command.clone())];
        command.extend(e.args.iter().cloned().map(Value::String));
        mcp.insert(
            e.key.clone(),
            json!({ "type": "local", "command": command, "enabled": true }),
        );
    }
    json!({ "mcp": mcp })
}

/// Merge `addition`'s entries under `object_key` on top of `base`'s,
/// preserving every key `base` already has. `base` is parsed leniently —
/// a missing or non-object value is replaced with `{}`.
fn merge_json_map(base: &Value, addition: &Value, object_key: &str) -> Value {
    let mut merged = if base.is_object() {
        base.clone()
    } else {
        json!({})
    };
    if !merged.get(object_key).is_some_and(Value::is_object) {
        merged[object_key] = json!({});
    }
    if let (Some(target), Some(source)) = (
        merged[object_key].as_object_mut(),
        addition.get(object_key).and_then(Value::as_object),
    ) {
        for (k, v) in source {
            target.insert(k.clone(), v.clone());
        }
    }
    merged
}

/// Render `entries` into `client`'s config, merged idempotently on top of
/// `existing_config` (pass the current file contents, or an empty string
/// for a fresh file). Returns the full new file contents.
pub fn render_merged(
    client: ClientKind,
    entries: &[McpServerEntry],
    existing_config: &str,
) -> String {
    match client {
        ClientKind::Claude | ClientKind::Gemini => {
            let base: Value = serde_json::from_str(existing_config)
                .unwrap_or_else(|_| json!({ "mcpServers": {} }));
            let merged = merge_json_map(&base, &mcp_servers_shape(entries), "mcpServers");
            serde_json::to_string_pretty(&merged).expect("serialize mcp json") + "\n"
        }
        ClientKind::Opencode => {
            let base: Value =
                serde_json::from_str(existing_config).unwrap_or_else(|_| json!({ "mcp": {} }));
            let merged = merge_json_map(&base, &opencode_shape(entries), "mcp");
            serde_json::to_string_pretty(&merged).expect("serialize opencode json") + "\n"
        }
        ClientKind::Codex => render_codex_toml(entries, existing_config),
    }
}

/// Codex's `~/.codex/config.toml`: a `[mcp_servers.<key>]` table each.
/// Uses `toml_edit` so unrelated top-level config the user set by hand
/// survives the merge.
fn render_codex_toml(entries: &[McpServerEntry], existing_config: &str) -> String {
    use toml_edit::{Array, DocumentMut, Item, Table, Value as TomlValue};

    let mut doc = existing_config
        .parse::<DocumentMut>()
        .unwrap_or_else(|_| DocumentMut::new());

    if !doc.contains_key("mcp_servers") {
        let mut t = Table::new();
        t.set_implicit(true);
        doc["mcp_servers"] = Item::Table(t);
    }
    let servers = doc["mcp_servers"]
        .as_table_mut()
        .expect("mcp_servers is a table");

    // Deterministic order for a stable file.
    let ordered: BTreeMap<&str, &McpServerEntry> =
        entries.iter().map(|e| (e.key.as_str(), e)).collect();
    for (key, entry) in ordered {
        let mut table = Table::new();
        table["command"] = Item::Value(TomlValue::from(entry.command.clone()));
        let mut args = Array::new();
        for a in &entry.args {
            args.push(a.clone());
        }
        table["args"] = Item::Value(TomlValue::Array(args));
        servers[key] = Item::Table(table);
    }

    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<McpServerEntry> {
        vec![
            McpServerEntry::new(
                "coding-assistants-mcp-blender",
                "/opt/ca/bin/coding-assistants-mcp-blender",
                &["--port", "9765"],
            ),
            McpServerEntry::new("coding-assistants-channel", "/opt/ca/bin/x", &[]),
        ]
    }

    #[test]
    fn claude_and_gemini_use_the_mcp_servers_shape() {
        for client in [ClientKind::Claude, ClientKind::Gemini] {
            let out = render_merged(client, &sample(), "");
            let v: Value = serde_json::from_str(&out).unwrap();
            assert_eq!(
                v["mcpServers"]["coding-assistants-mcp-blender"]["command"],
                "/opt/ca/bin/coding-assistants-mcp-blender"
            );
            assert_eq!(
                v["mcpServers"]["coding-assistants-mcp-blender"]["args"],
                json!(["--port", "9765"])
            );
        }
    }

    #[test]
    fn claude_merge_preserves_a_hand_added_server_and_overwrites_only_our_keys() {
        let existing = json!({
            "mcpServers": {
                "user-filesystem": { "command": "npx", "args": ["-y", "@mcp/fs"] },
                "coding-assistants-channel": { "command": "STALE" }
            }
        })
        .to_string();
        let out = render_merged(ClientKind::Claude, &sample(), &existing);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["mcpServers"]["user-filesystem"]["command"], "npx");
        assert_eq!(
            v["mcpServers"]["coding-assistants-channel"]["command"],
            "/opt/ca/bin/x"
        );
    }

    #[test]
    fn claude_merge_is_idempotent() {
        let once = render_merged(ClientKind::Claude, &sample(), "");
        let twice = render_merged(ClientKind::Claude, &sample(), &once);
        assert_eq!(once, twice);
    }

    #[test]
    fn opencode_uses_command_array_and_local_type() {
        let out = render_merged(ClientKind::Opencode, &sample(), "");
        let v: Value = serde_json::from_str(&out).unwrap();
        let blender = &v["mcp"]["coding-assistants-mcp-blender"];
        assert_eq!(blender["type"], "local");
        assert_eq!(blender["enabled"], true);
        assert_eq!(
            blender["command"],
            json!([
                "/opt/ca/bin/coding-assistants-mcp-blender",
                "--port",
                "9765"
            ])
        );
    }

    #[test]
    fn opencode_merge_keeps_unrelated_top_level_keys() {
        let existing =
            json!({ "theme": "opencode", "mcp": { "other": { "type": "local" } } }).to_string();
        let out = render_merged(ClientKind::Opencode, &sample(), &existing);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["theme"], "opencode");
        assert!(v["mcp"]["other"].is_object());
        assert!(v["mcp"]["coding-assistants-channel"].is_object());
    }

    #[test]
    fn codex_toml_writes_a_table_per_server_and_keeps_user_config() {
        let existing = "model = \"o3\"\n\n[mcp_servers.user-thing]\ncommand = \"foo\"\n";
        let out = render_codex_toml(&sample(), existing);
        assert!(out.contains("model = \"o3\""));
        assert!(out.contains("[mcp_servers.user-thing]"));
        assert!(out.contains("[mcp_servers.coding-assistants-mcp-blender]"));
        assert!(out.contains("command = \"/opt/ca/bin/coding-assistants-mcp-blender\""));
        assert!(out.contains("args = [\"--port\", \"9765\"]"));
        // idempotent
        let twice = render_codex_toml(&sample(), &out);
        assert_eq!(out, twice);
    }

    #[test]
    fn codex_toml_from_empty_is_valid_and_reparses() {
        let out = render_codex_toml(&sample(), "");
        let doc = out.parse::<toml_edit::DocumentMut>().unwrap();
        assert!(doc["mcp_servers"]["coding-assistants-channel"].is_table());
    }

    #[test]
    fn config_paths_are_what_each_client_reads() {
        assert_eq!(
            ClientKind::Claude.workspace_relative_config_path(),
            Some(".mcp.json")
        );
        assert_eq!(
            ClientKind::Gemini.workspace_relative_config_path(),
            Some(".gemini/settings.json")
        );
        assert_eq!(
            ClientKind::Opencode.workspace_relative_config_path(),
            Some("opencode.json")
        );
        assert_eq!(ClientKind::Codex.workspace_relative_config_path(), None);
        assert_eq!(
            ClientKind::Codex.home_relative_config_path(),
            Some(".codex/config.toml")
        );
    }
}
