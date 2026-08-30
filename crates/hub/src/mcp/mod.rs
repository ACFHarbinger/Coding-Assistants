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

pub mod creative;

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
///
/// This only ever *adds or overwrites* keys. It has no way to know a
/// previously-app-managed server should now be gone — use
/// [`render_replacing`] when the caller owns a known set of keys and needs
/// removals to take effect (e.g. the user disabling a creative tool).
pub fn render_merged(
    client: ClientKind,
    entries: &[McpServerEntry],
    existing_config: &str,
) -> String {
    render_replacing(client, &[], entries, existing_config)
}

/// Like [`render_merged`], but first drops every key in `owned_keys` that
/// is *not* present in `entries`. Keys in neither list — servers a user
/// added by hand — always survive. Use this when the app owns a fixed
/// namespace of server keys (the `crates/mcp-<tool>` bridges) and a
/// toggle-off has to actually remove the entry, not just stop refreshing
/// it.
///
/// Still idempotent: rendering the output through it again is a no-op.
pub fn render_replacing(
    client: ClientKind,
    owned_keys: &[&str],
    entries: &[McpServerEntry],
    existing_config: &str,
) -> String {
    let to_remove: Vec<&str> = owned_keys
        .iter()
        .copied()
        .filter(|key| !entries.iter().any(|e| e.key == *key))
        .collect();

    match client {
        ClientKind::Claude | ClientKind::Gemini => {
            let mut base: Value = serde_json::from_str(existing_config)
                .unwrap_or_else(|_| json!({ "mcpServers": {} }));
            prune_json_map(&mut base, "mcpServers", &to_remove);
            let merged = merge_json_map(&base, &mcp_servers_shape(entries), "mcpServers");
            serde_json::to_string_pretty(&merged).expect("serialize mcp json") + "\n"
        }
        ClientKind::Opencode => {
            let mut base: Value =
                serde_json::from_str(existing_config).unwrap_or_else(|_| json!({ "mcp": {} }));
            prune_json_map(&mut base, "mcp", &to_remove);
            let merged = merge_json_map(&base, &opencode_shape(entries), "mcp");
            serde_json::to_string_pretty(&merged).expect("serialize opencode json") + "\n"
        }
        ClientKind::Codex => {
            let pruned = prune_codex_toml(existing_config, &to_remove);
            render_codex_toml(entries, &pruned)
        }
    }
}

/// Remove `keys` from the object at `base[object_key]`, if that object
/// exists. Leaves everything else untouched.
fn prune_json_map(base: &mut Value, object_key: &str, keys: &[&str]) {
    if keys.is_empty() {
        return;
    }
    if let Some(target) = base.get_mut(object_key).and_then(Value::as_object_mut) {
        for key in keys {
            target.remove(*key);
        }
    }
}

/// Drop `[mcp_servers.<key>]` tables for each of `keys` from a Codex
/// `config.toml`, preserving the rest of the document.
fn prune_codex_toml(existing_config: &str, keys: &[&str]) -> String {
    if keys.is_empty() {
        return existing_config.to_string();
    }
    use toml_edit::DocumentMut;
    let Ok(mut doc) = existing_config.parse::<DocumentMut>() else {
        return existing_config.to_string();
    };
    if let Some(servers) = doc.get_mut("mcp_servers").and_then(|i| i.as_table_mut()) {
        for key in keys {
            servers.remove(key);
        }
    }
    doc.to_string()
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
    fn render_replacing_removes_a_disabled_owned_key_but_keeps_hand_added_servers() {
        let owned = [
            "coding-assistants-mcp-blender",
            "coding-assistants-mcp-krita",
        ];
        // Start: blender + krita both app-managed, plus a user server.
        let both = vec![
            McpServerEntry::new("coding-assistants-mcp-blender", "/b", &["--port", "9765"]),
            McpServerEntry::new("coding-assistants-mcp-krita", "/k", &["--port", "9766"]),
        ];
        let start = render_replacing(ClientKind::Claude, &owned, &both, "");
        let with_user: Value = {
            let mut v: Value = serde_json::from_str(&start).unwrap();
            v["mcpServers"]["user-fs"] = json!({ "command": "npx", "args": ["-y", "@mcp/fs"] });
            v
        };
        // Now only blender stays enabled.
        let only_blender = vec![McpServerEntry::new(
            "coding-assistants-mcp-blender",
            "/b",
            &["--port", "9765"],
        )];
        let out = render_replacing(
            ClientKind::Claude,
            &owned,
            &only_blender,
            &with_user.to_string(),
        );
        let v: Value = serde_json::from_str(&out).unwrap();
        assert!(v["mcpServers"]["coding-assistants-mcp-blender"].is_object());
        assert!(
            v["mcpServers"]["coding-assistants-mcp-krita"].is_null(),
            "disabled owned key must be removed"
        );
        assert_eq!(
            v["mcpServers"]["user-fs"]["command"], "npx",
            "a hand-added server must survive"
        );
    }

    #[test]
    fn render_replacing_removes_a_disabled_codex_table() {
        let owned = ["coding-assistants-mcp-blender"];
        let with_it = vec![McpServerEntry::new(
            "coding-assistants-mcp-blender",
            "/b",
            &[],
        )];
        let start = render_replacing(ClientKind::Codex, &owned, &with_it, "model = \"o3\"\n");
        assert!(start.contains("[mcp_servers.coding-assistants-mcp-blender]"));
        let out = render_replacing(ClientKind::Codex, &owned, &[], &start);
        assert!(!out.contains("coding-assistants-mcp-blender"));
        assert!(out.contains("model = \"o3\""));
    }

    #[test]
    fn render_replacing_with_no_owned_keys_matches_render_merged() {
        let existing = json!({ "mcpServers": { "x": { "command": "y" } } }).to_string();
        assert_eq!(
            render_replacing(ClientKind::Claude, &[], &sample(), &existing),
            render_merged(ClientKind::Claude, &sample(), &existing),
        );
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
