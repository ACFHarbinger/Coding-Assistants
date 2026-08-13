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
use std::path::Path;

pub const CLAUDE_AGENT_ID: &str = "claude";

const PERMISSION_ROOT: &str = "claude_channel_permission";

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

/// Opt-in setup: registers `claude` as a Hub-managed harness session for
/// `workspace` (so the existing C14.1 single-writer lease applies to it
/// like any other managed provider), and returns the `.mcp.json` server
/// entry the caller should merge into that workspace's config. This never
/// touches an *existing*, non-opted-in Claude session — registration is a
/// separate, deliberate action the owner takes per workspace.
pub fn setup_claude_channel(
    store: &HubStore,
    workspace: &Path,
    bridge_binary: &Path,
) -> Result<serde_json::Value, HubError> {
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
    Ok(serde_json::json!({
        "mcpServers": {
            "coding-assistants-channel": {
                "command": bridge_binary.to_string_lossy(),
                "args": ["--workspace", workspace.to_string_lossy()],
            }
        }
    }))
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
