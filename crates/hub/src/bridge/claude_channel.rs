//! C14.3: opt-in Claude Code "Channel" bridge glue.
//!
//! This module is pure Hub-side logic: reading pending authenticated
//! events addressed to `claude`, recording Claude's reply back into the
//! Hub, and durably tracking permission-relay requests so nothing is ever
//! auto-approved. Nothing here speaks MCP or touches Claude Code's
//! internal `cc-socks` control socket — the actual MCP stdio server that
//! Claude Code spawns via the documented `claude/channel` capability lives
//! in the separate `claude-channel` binary crate, which links against
//! this module. This file also never mutates `bridge::claude`'s C12
//! capture-only delivery-safety path; that bridge continues to serve
//! sessions that have not opted into a Channel.
//!
//! **Authenticated sender gate:** rather than a bespoke crypto layer, the
//! gate reuses the Hub's existing trust boundary — only messages from an
//! *enrolled team member* are ever pushed into a live Claude session.
//! Both the bridge process and the Hub already trust the same local
//! SQLite store; the risk this gate defends against is an unenrolled or
//! stray identity string reaching a live session, not a network attacker.
//!
//! **Permission relay:** reuses the existing hash-chained `audit_events`
//! table (the same one Settings' audit stream is a typed filter over)
//! instead of a new table. A request starts `pending`; the bridge relays
//! `notifications/claude/channel/permission` only after a human explicitly
//! calls [`resolve_permission_request`] — there is no code path that
//! marks a request `approved` on its own.

use crate::{HubError, HubStore, MessageKind, MessageRecord};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const CLAUDE_AGENT_ID: &str = "claude";

const PERMISSION_ROOT: &str = "claude_channel_permission";
const SERVERS_SUBDIR: &str = "servers";
const GLOBAL_SERVERS_FILE: &str = "global.mcp.json";
const CHANNEL_SERVER_KEY: &str = "coding-assistants-channel";

/// One authenticated Hub event ready to push into a Channel-connected
/// Claude Code session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelEvent {
    pub message_id: String,
    pub from_agent: String,
    pub session_id: Option<String>,
    pub kind: String,
    pub body: String,
}

/// Resolved (or still-pending) state of a relayed permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionVerdict {
    Pending,
    Allowed,
    Denied,
}

/// A previously-configured Channel workspace, for owner management
/// (list/rename/delete) — e.g. a Shared Hub panel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChannelWorkspace {
    pub workspace: String,
    pub display_name: String,
}

/// Every Channel `.mcp.json` this app owns lives here — never inside a
/// repository except the one file Claude Code itself requires (written
/// separately by the `claude-channel` binary's `--setup`). This directory
/// is the durable, app-owned record: it survives a repo being deleted or
/// re-cloned, and lets an owner list/rename/delete configured workspaces
/// without hunting through project directories.
///
/// Derived from `store.data_dir()` rather than the process-global
/// `default_hub_home()` — the same fix applied to the S5 orchestration
/// policy lookup — so a `HubStore` opened at a non-default path (every
/// test, and any future caller with a custom data dir) keeps its Channel
/// config alongside its own data instead of the host machine's real home.
pub fn servers_dir(store: &HubStore) -> PathBuf {
    store.data_dir().join(SERVERS_SUBDIR)
}

/// Servers merged as a base layer into *every* workspace's generated
/// config. Empty (`{"mcpServers": {}}`) until the owner adds something.
pub fn global_servers_path(store: &HubStore) -> PathBuf {
    servers_dir(store).join(GLOBAL_SERVERS_FILE)
}

/// Deterministic, collision-proof file stem for a workspace: its
/// directory name plus a short stable hash of the full absolute path, so
/// two differently-located repos that happen to share a directory name
/// never collide.
pub fn workspace_server_name(workspace: &Path) -> String {
    let base = workspace
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "workspace".to_string());
    let digest = Sha256::digest(workspace.to_string_lossy().as_bytes());
    let short: String = digest
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{base}-{short}")
}

/// The canonical, app-owned copy of one workspace's Channel config.
pub fn workspace_servers_path(store: &HubStore, workspace: &Path) -> PathBuf {
    servers_dir(store).join(format!("{}.mcp.json", workspace_server_name(workspace)))
}

fn read_mcp_config(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({ "mcpServers": {} }))
}

/// Merges `addition`'s `mcpServers` entries on top of `base`'s, keeping
/// every other server `base` already defines.
fn merge_mcp_configs(base: &Value, addition: &Value) -> Value {
    let mut merged = base.clone();
    if !merged.get("mcpServers").is_some_and(Value::is_object) {
        merged["mcpServers"] = json!({});
    }
    if let (Some(target), Some(source)) = (
        merged["mcpServers"].as_object_mut(),
        addition.get("mcpServers").and_then(Value::as_object),
    ) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    merged
}

fn channel_server_entry(bridge_binary: &Path, workspace: &Path) -> Value {
    json!({
        "mcpServers": {
            CHANNEL_SERVER_KEY: {
                "command": bridge_binary.to_string_lossy(),
                "args": ["--workspace", workspace.to_string_lossy()],
            }
        }
    })
}

/// Opt-in setup: registers `claude` as a Hub-managed harness session for
/// `workspace` (so the existing C14.1 single-writer lease applies to it
/// like any other managed provider), writes the canonical per-workspace
/// copy under [`servers_dir`], and returns the *effective* config (this
/// workspace's entry merged on top of [`global_servers_path`]'s base
/// layer) for the caller to write into the workspace's own `.mcp.json` —
/// the one file Claude Code actually reads. This never touches an
/// *existing*, non-opted-in Claude session; registration is a separate,
/// deliberate action the owner takes per workspace.
pub fn setup_claude_channel(
    store: &HubStore,
    workspace: &Path,
    bridge_binary: &Path,
) -> Result<Value, HubError> {
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Claude Channel setup requires an absolute workspace".into(),
        ));
    }
    let label = format!("channel:{}", chrono::Utc::now().timestamp());
    store.register_managed_harness_session(
        CLAUDE_AGENT_ID,
        &workspace.to_string_lossy(),
        &label,
        std::process::id(),
    )?;

    std::fs::create_dir_all(servers_dir(store))?;
    let global_path = global_servers_path(store);
    if !global_path.exists() {
        std::fs::write(
            &global_path,
            serde_json::to_string_pretty(&json!({ "mcpServers": {} }))
                .expect("serialize empty config")
                + "\n",
        )?;
    }

    let mut canonical = channel_server_entry(bridge_binary, workspace);
    canonical["_workspace"] = json!(workspace.to_string_lossy());
    canonical["_display_name"] = json!(default_display_name(workspace));
    std::fs::write(
        workspace_servers_path(store, workspace),
        serde_json::to_string_pretty(&canonical).map_err(|e| HubError::Invalid(e.to_string()))?
            + "\n",
    )?;

    let global = read_mcp_config(&global_path);
    Ok(merge_mcp_configs(&global, &canonical))
}

fn default_display_name(workspace: &Path) -> String {
    workspace
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| workspace.to_string_lossy().to_string())
}

/// Every workspace previously configured for a Channel bridge. Pure
/// filesystem scan over [`servers_dir`] — the canonical files are the
/// durable record.
pub fn list_channel_workspaces(store: &HubStore) -> Result<Vec<ChannelWorkspace>, HubError> {
    let dir = servers_dir(store);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut workspaces = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(GLOBAL_SERVERS_FILE) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let config = read_mcp_config(&path);
        let Some(workspace) = config.get("_workspace").and_then(Value::as_str) else {
            continue;
        };
        let display_name = config
            .get("_display_name")
            .and_then(Value::as_str)
            .unwrap_or(workspace)
            .to_string();
        workspaces.push(ChannelWorkspace {
            workspace: workspace.to_string(),
            display_name,
        });
    }
    workspaces.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(workspaces)
}

/// Updates only the cosmetic display name of an already-configured
/// workspace; the canonical filename, the workspace path, and the Hub
/// registration are unaffected.
pub fn rename_channel_workspace(
    store: &HubStore,
    workspace: &Path,
    display_name: &str,
) -> Result<(), HubError> {
    let display_name = display_name.trim();
    if display_name.is_empty() {
        return Err(HubError::Invalid(
            "Channel workspace display name must not be empty".into(),
        ));
    }
    let path = workspace_servers_path(store, workspace);
    if !path.exists() {
        return Err(HubError::NotFound(format!(
            "no Channel configuration for {}",
            workspace.display()
        )));
    }
    let mut config = read_mcp_config(&path);
    config["_display_name"] = json!(display_name);
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&config).map_err(|e| HubError::Invalid(e.to_string()))? + "\n",
    )?;
    Ok(())
}

/// Removes the canonical Channel config for `workspace` and downgrades
/// its Hub registration back to `observed` (the same state a purely
/// discovered, non-managed session has) — it does not delete the
/// workspace's own `.mcp.json`; the owner (or a future setup run) manages
/// that file directly.
pub fn delete_channel_workspace(store: &HubStore, workspace: &Path) -> Result<(), HubError> {
    let path = workspace_servers_path(store, workspace);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    if let Some(registration) =
        store.get_harness_session(CLAUDE_AGENT_ID, &workspace.to_string_lossy())?
    {
        store.register_harness_session(
            CLAUDE_AGENT_ID,
            &workspace.to_string_lossy(),
            &registration.disk_session_id,
            registration.leader_socket.as_deref(),
        )?;
    }
    Ok(())
}

/// Authenticated sender gate + drain: every pending message addressed to
/// `claude` from an enrolled team member. Marks each returned message
/// acked (same semantics as `ca inbox watch` / `hub_poll_messages`) so a
/// restarted bridge does not replay history.
pub fn poll_channel_events(store: &HubStore) -> Result<Vec<ChannelEvent>, HubError> {
    let pending = store.poll_messages(CLAUDE_AGENT_ID, true)?;
    let enrolled: std::collections::HashSet<String> = store
        .list_team_members()?
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    Ok(pending
        .into_iter()
        .filter(|message| enrolled.contains(&message.from_agent))
        .map(|message| ChannelEvent {
            session_id: session_id_from_subject(message.subject.as_deref()),
            message_id: message.id,
            from_agent: message.from_agent,
            kind: message.kind,
            body: message.body,
        })
        .collect())
}

/// The `reply` MCP tool calls this: routes Claude's output back to the
/// Hub, addressed to the original sender of `in_reply_to` when known
/// (falling back to `human`), inside `session_id` when given.
pub fn record_channel_reply(
    store: &HubStore,
    in_reply_to: Option<&str>,
    session_id: Option<&str>,
    body: &str,
) -> Result<MessageRecord, HubError> {
    if body.trim().is_empty() {
        return Err(HubError::Invalid(
            "Channel reply body must not be empty".into(),
        ));
    }
    let recipient = match in_reply_to {
        Some(id) => store
            .get_message(id)?
            .map(|message| message.from_agent)
            .unwrap_or_else(|| "human".to_string()),
        None => "human".to_string(),
    };
    let subject = session_id.map(|id| format!("channel:session:{id}:reply"));
    if let Some(session_id) = session_id {
        let sent = store.send_session_message(
            CLAUDE_AGENT_ID,
            session_id,
            &[recipient],
            body,
            subject.as_deref(),
            None,
            None,
        )?;
        sent.into_iter()
            .next()
            .ok_or_else(|| HubError::Invalid("Channel reply produced no recipient".into()))
    } else {
        store.send_message(
            CLAUDE_AGENT_ID,
            &recipient,
            MessageKind::Message,
            body,
            subject.as_deref(),
            None,
            None,
        )
    }
}

/// A `notifications/claude/channel/permission_request` arrived from
/// Claude Code. Durably recorded as `pending` on the shared Hub audit
/// chain — never auto-approved.
pub fn record_permission_request(
    store: &HubStore,
    request_id: &str,
    tool_name: &str,
    description: &str,
    input_preview: &str,
) -> Result<(), HubError> {
    if request_id.trim().is_empty() {
        return Err(HubError::Invalid(
            "permission request_id must not be empty".into(),
        ));
    }
    let process_json = serde_json::json!({
        "tool_name": tool_name,
        "description": description,
        "input_preview": input_preview,
    })
    .to_string();
    store.record_audit_event(
        Path::new(PERMISSION_ROOT),
        Path::new(request_id),
        "request",
        &process_json,
        None,
    )?;
    Ok(())
}

/// Current verdict for a relayed permission request, if one was ever
/// recorded.
pub fn get_permission_request(
    store: &HubStore,
    request_id: &str,
) -> Result<Option<PermissionVerdict>, HubError> {
    Ok(store
        .list_audit_events(false)?
        .into_iter()
        .rfind(|event| event.root_path == PERMISSION_ROOT && event.path == request_id)
        .map(|event| verdict_from_status(&event.status)))
}

/// A human explicitly approves or denies a pending permission request.
/// This is the *only* function in this module that can move a request out
/// of `pending` — the bridge relays a verdict to Claude Code only after
/// this returns `Ok`.
pub fn resolve_permission_request(
    store: &HubStore,
    request_id: &str,
    allow: bool,
) -> Result<PermissionVerdict, HubError> {
    let event = store
        .list_audit_events(true)?
        .into_iter()
        .rfind(|event| event.root_path == PERMISSION_ROOT && event.path == request_id)
        .ok_or_else(|| {
            HubError::NotFound(format!("pending Channel permission request {request_id}"))
        })?;
    let status = if allow { "approved" } else { "quarantined" };
    store.set_audit_status(&event.id, status)?;
    Ok(verdict_from_status(status))
}

fn verdict_from_status(status: &str) -> PermissionVerdict {
    match status {
        "approved" => PermissionVerdict::Allowed,
        "quarantined" => PermissionVerdict::Denied,
        _ => PermissionVerdict::Pending,
    }
}

/// Extracts a session id from the `channel:session:<id>:...` subject
/// convention already used elsewhere in the Hub (see
/// `store/agents/capture.rs`, `store/messages/mod.rs`).
fn session_id_from_subject(subject: Option<&str>) -> Option<String> {
    subject
        .and_then(|subject| subject.strip_prefix("channel:session:"))
        .and_then(|rest| rest.split(':').next())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn setup_registers_a_managed_session_and_returns_mcp_config() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        let bridge = Path::new("/usr/local/bin/claude-channel");

        let config = setup_claude_channel(&store, &workspace, bridge).unwrap();
        assert_eq!(
            config["mcpServers"]["coding-assistants-channel"]["command"],
            bridge.to_string_lossy().to_string()
        );

        let registration = store
            .get_harness_session("claude", &workspace.to_string_lossy())
            .unwrap()
            .expect("registered");
        assert_eq!(registration.mode, crate::HarnessSessionMode::Managed);
    }

    #[test]
    fn setup_rejects_a_relative_workspace() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let bridge = Path::new("claude-channel");
        assert!(setup_claude_channel(&store, Path::new("relative/path"), bridge).is_err());
    }

    #[test]
    fn setup_writes_a_canonical_copy_and_an_empty_global_base() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        let bridge = Path::new("/usr/local/bin/coding-assistants-claude-channel");

        setup_claude_channel(&store, &workspace, bridge).unwrap();

        assert!(global_servers_path(&store).exists());
        let canonical_path = workspace_servers_path(&store, &workspace);
        assert!(canonical_path.exists());
        let canonical = read_mcp_config(&canonical_path);
        assert_eq!(canonical["_workspace"], json!(workspace.to_string_lossy()));
        assert_eq!(
            canonical["mcpServers"][CHANNEL_SERVER_KEY]["command"],
            bridge.to_string_lossy().to_string()
        );
    }

    #[test]
    fn setup_merges_the_global_base_layer_into_the_effective_config() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(servers_dir(&store)).unwrap();
        std::fs::write(
            global_servers_path(&store),
            json!({ "mcpServers": { "shared-tool": { "command": "shared" } } }).to_string(),
        )
        .unwrap();

        let effective = setup_claude_channel(&store, &workspace, Path::new("bridge")).unwrap();
        assert_eq!(effective["mcpServers"]["shared-tool"]["command"], "shared");
        assert_eq!(
            effective["mcpServers"][CHANNEL_SERVER_KEY]["command"],
            "bridge"
        );
    }

    #[test]
    fn distinct_workspaces_sharing_a_directory_name_do_not_collide() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let a = dir.path().join("a/repo");
        let b = dir.path().join("b/repo");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();

        setup_claude_channel(&store, &a, Path::new("bridge")).unwrap();
        setup_claude_channel(&store, &b, Path::new("bridge")).unwrap();

        assert_ne!(
            workspace_servers_path(&store, &a),
            workspace_servers_path(&store, &b)
        );
        let workspaces = list_channel_workspaces(&store).unwrap();
        assert_eq!(workspaces.len(), 2);
    }

    #[test]
    fn list_rename_and_delete_manage_the_registry() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        setup_claude_channel(&store, &workspace, Path::new("bridge")).unwrap();

        let workspaces = list_channel_workspaces(&store).unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].workspace, workspace.to_string_lossy());
        assert_eq!(workspaces[0].display_name, "repo");

        rename_channel_workspace(&store, &workspace, "My Repo").unwrap();
        let renamed = list_channel_workspaces(&store).unwrap();
        assert_eq!(renamed[0].display_name, "My Repo");

        delete_channel_workspace(&store, &workspace).unwrap();
        assert!(list_channel_workspaces(&store).unwrap().is_empty());
        let registration = store
            .get_harness_session(CLAUDE_AGENT_ID, &workspace.to_string_lossy())
            .unwrap()
            .expect("registration record survives, downgraded");
        assert_eq!(registration.mode, crate::HarnessSessionMode::Observed);
    }

    #[test]
    fn rename_rejects_an_empty_name_and_an_unconfigured_workspace() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        setup_claude_channel(&store, &workspace, Path::new("bridge")).unwrap();

        assert!(rename_channel_workspace(&store, &workspace, "   ").is_err());
        assert!(rename_channel_workspace(&store, Path::new("/never/configured"), "x").is_err());
    }

    #[test]
    fn list_channel_workspaces_is_empty_before_any_setup() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(list_channel_workspaces(&store).unwrap().is_empty());
    }

    #[test]
    fn poll_only_returns_events_from_enrolled_senders() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();

        store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "from an enrolled sender",
                None,
                None,
                None,
            )
            .unwrap();
        store
            .send_message(
                "unknown-stray-id",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "from an unenrolled sender",
                None,
                None,
                None,
            )
            .unwrap();

        let events = poll_channel_events(&store).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].from_agent, "grok");
        assert_eq!(events[0].body, "from an enrolled sender");

        // Draining acks the messages; a second poll returns nothing new.
        assert!(poll_channel_events(&store).unwrap().is_empty());
    }

    #[test]
    fn poll_extracts_session_id_from_the_channel_subject_convention() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();
        store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "session-scoped",
                Some("channel:session:abc-123:message"),
                None,
                None,
            )
            .unwrap();

        let events = poll_channel_events(&store).unwrap();
        assert_eq!(events[0].session_id.as_deref(), Some("abc-123"));
    }

    #[test]
    fn reply_routes_to_the_original_senders_and_falls_back_to_human() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();
        let original = store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "question",
                None,
                None,
                None,
            )
            .unwrap();

        let reply = record_channel_reply(&store, Some(&original.id), None, "answer").unwrap();
        assert_eq!(reply.from_agent, CLAUDE_AGENT_ID);
        assert_eq!(reply.to_agent, "grok");

        let fallback = record_channel_reply(&store, None, None, "unprompted note").unwrap();
        assert_eq!(fallback.to_agent, "human");
    }

    #[test]
    fn reply_rejects_an_empty_body() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(record_channel_reply(&store, None, None, "   ").is_err());
    }

    #[test]
    fn permission_request_starts_pending_and_is_never_auto_approved() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        record_permission_request(&store, "req-1", "Bash", "run tests", "cargo test").unwrap();

        assert_eq!(
            get_permission_request(&store, "req-1").unwrap(),
            Some(PermissionVerdict::Pending)
        );

        // Unrelated audit activity must not change the verdict.
        let watched = dir.path().join("watched");
        std::fs::create_dir_all(&watched).unwrap();
        store
            .record_audit_event(&watched, Path::new("file.rs"), "modified", "{}", None)
            .unwrap();
        assert_eq!(
            get_permission_request(&store, "req-1").unwrap(),
            Some(PermissionVerdict::Pending)
        );
    }

    #[test]
    fn permission_request_resolves_only_through_explicit_human_action() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        record_permission_request(&store, "req-2", "Write", "edit file", "src/lib.rs").unwrap();

        let verdict = resolve_permission_request(&store, "req-2", true).unwrap();
        assert_eq!(verdict, PermissionVerdict::Allowed);
        assert_eq!(
            get_permission_request(&store, "req-2").unwrap(),
            Some(PermissionVerdict::Allowed)
        );
    }

    #[test]
    fn permission_request_can_be_denied() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        record_permission_request(&store, "req-3", "Bash", "rm -rf", "dangerous").unwrap();
        let verdict = resolve_permission_request(&store, "req-3", false).unwrap();
        assert_eq!(verdict, PermissionVerdict::Denied);
    }

    #[test]
    fn resolving_an_unknown_request_id_fails() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(resolve_permission_request(&store, "does-not-exist", true).is_err());
    }
}
