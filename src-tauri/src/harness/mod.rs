//! Capture and supported delivery adapters for agent harnesses.

mod acceptance;
pub mod blocking;
pub mod capture_commands;
pub mod claude;
pub mod codex;
pub mod commands;
pub mod gemini;
pub mod grok;
pub mod presence;
pub mod stop;

use hub::HubStore;
use std::path::Path;

/// Resolve which disk session a capture poll should target.
///
/// #165 capture-identity gate: an explicit session id wins; otherwise the
/// registered (observed/managed) session for (harness, workspace) is used,
/// matched by raw then canonical workspace key so a symlink/trailing-slash
/// difference in the app's path cannot hide the registration. Returns
/// `None` when the app has not opted into capturing this (harness,
/// workspace) — the caller must then skip capture entirely rather than
/// grabbing the provider's newest external transcript and attributing it to
/// the active work session (the reroute symptom).
pub(crate) fn resolve_capture_session_id(
    store: &HubStore,
    harness: &str,
    workspace: &Path,
    explicit: Option<&str>,
) -> Result<Option<String>, String> {
    if let Some(id) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(Some(id.to_string()));
    }
    let Some(session_id) = registered_disk_session(store, harness, workspace)? else {
        return Ok(None);
    };
    let registration = store
        .get_harness_session(harness, &workspace.to_string_lossy())
        .map_err(|error| error.to_string())?;
    // A managed start first records only an opaque, freshly generated id.
    // Do not poll any transcript until a real task has acquired and released
    // the writer lease (state becomes ready). This drops CLI greetings and
    // makes a pre-existing global transcript unreachable from a new session.
    if registration.is_some_and(|row| {
        row.mode == hub::HarnessSessionMode::Managed
            && row.state == hub::HarnessSessionState::Queued
    }) {
        return Ok(None);
    }
    Ok(Some(session_id))
}

fn registered_disk_session(
    store: &HubStore,
    harness: &str,
    workspace: &Path,
) -> Result<Option<String>, String> {
    let raw = workspace.to_string_lossy();
    if let Some(row) = store
        .get_harness_session(harness, &raw)
        .map_err(|error| error.to_string())?
    {
        return Ok(Some(row.disk_session_id));
    }
    let canonical = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let canonical = canonical.to_string_lossy();
    if canonical != raw {
        if let Some(row) = store
            .get_harness_session(harness, &canonical)
            .map_err(|error| error.to_string())?
        {
            return Ok(Some(row.disk_session_id));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn explicit_session_id_wins_over_any_registration() {
        let dir = tempdir().unwrap();
        let store = hub::HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session("claude", "/abs/ws", "registered-1", None)
            .unwrap();
        let resolved =
            resolve_capture_session_id(&store, "claude", Path::new("/abs/ws"), Some("explicit-2"))
                .unwrap();
        assert_eq!(resolved.as_deref(), Some("explicit-2"));
    }

    #[test]
    fn registered_session_is_used_when_no_explicit_id() {
        let dir = tempdir().unwrap();
        let store = hub::HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session("claude", "/abs/ws", "registered-1", None)
            .unwrap();
        let resolved =
            resolve_capture_session_id(&store, "claude", Path::new("/abs/ws"), None).unwrap();
        assert_eq!(resolved.as_deref(), Some("registered-1"));
    }

    #[test]
    fn unregistered_workspace_resolves_to_none() {
        let dir = tempdir().unwrap();
        let store = hub::HubStore::open(dir.path()).unwrap();
        let resolved =
            resolve_capture_session_id(&store, "claude", Path::new("/abs/ws"), None).unwrap();
        assert_eq!(resolved, None);
        // A blank explicit id is treated the same as no id.
        let resolved =
            resolve_capture_session_id(&store, "claude", Path::new("/abs/ws"), Some("  ")).unwrap();
        assert_eq!(resolved, None);
    }

    #[test]
    fn queued_managed_session_does_not_arm_capture_of_a_prior_transcript() {
        let dir = tempdir().unwrap();
        let store = hub::HubStore::open(dir.path()).unwrap();
        store
            .register_managed_harness_session_with_state(
                "gemini",
                "/abs/fresh-workspace",
                "managed-fresh-id",
                None,
                hub::HarnessSessionState::Queued,
            )
            .unwrap();
        let resolved =
            resolve_capture_session_id(&store, "gemini", Path::new("/abs/fresh-workspace"), None)
                .unwrap();
        assert_eq!(resolved, None);
    }
}
