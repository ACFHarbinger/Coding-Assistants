//! The never-auto-approved permission relay lifecycle: a request starts
//! `pending`, and only [`resolve_permission_request`] (an explicit human
//! action) can move it out of that state.

use crate::{HubError, HubStore};
use std::path::Path;

const PERMISSION_ROOT: &str = "claude_channel_permission";

/// Resolved (or still-pending) state of a relayed permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionVerdict {
    Pending,
    Allowed,
    Denied,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
