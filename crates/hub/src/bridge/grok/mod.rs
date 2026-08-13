//! C12/C14 Grok bridge: deliver a queued Hub task into a Grok session
//! through the documented ACP + leader path.
//!
//! Grok Build's supported attach is `grok agent --leader stdio` talking to
//! `~/.grok/leader.sock` (or `GROK_LEADER_SOCKET`), then `session/load` +
//! `session/prompt`. That process is an ACP *client* of the existing leader,
//! not a replacement TUI. If the leader socket is absent, delivery is
//! `unavailable` and the task stays queued.

mod acp;
mod leader;
mod sessions;

pub use acp::{acp_initialize, acp_session_load, acp_session_prompt};
pub use leader::{
    connect_grok_leader_session, default_leader_socket, grok_leader_status,
    leader_socket_available, GrokConnectResult,
};
pub use sessions::{
    active_grok_session_for, latest_grok_session_id, list_active_grok_sessions, ActiveGrokSession,
};

use crate::{HarnessInjectRequest, HarnessInjectResult, HubError, HubStore};
use acp::run_acp_prompt;
use std::path::PathBuf;

/// Deliver a task into a registered Grok session. Never writes a TTY/PTY and
/// never starts a replacement interactive `grok` TUI.
pub fn deliver_grok_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Grok active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let registration = store.get_harness_session("grok", &workspace.to_string_lossy())?;
    // `request.session_id` is the Hub work-session id, not Grok's disk/ACP
    // session id. Never pass it to `session/load`.
    let session_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .or_else(|| latest_grok_session_id(&workspace));
    let Some(session_id) = session_id else {
        return Ok(unavailable(
            "no registered or on-disk Grok session for this workspace; use Shared Hub → Channels → Connect Grok (leader mode), or register one with hub_register_harness_session",
        ));
    };
    let socket = registration
        .as_ref()
        .and_then(|row| row.leader_socket.as_deref())
        .map(PathBuf::from)
        .unwrap_or_else(default_leader_socket);
    if !leader_socket_available(&socket) {
        return Ok(unavailable(&format!(
            "no leader socket at {} — start Grok with --leader (or [cli] use_leader = true) to enable delivery. Task stays queued.",
            socket.display()
        )));
    }

    match run_acp_prompt(&socket, &workspace, &session_id, &request.body) {
        Ok(reply) => {
            if let Some(message_id) = request.message_id.as_deref() {
                let _ = store.set_message_status(message_id, crate::MessageStatus::Acked);
            }
            let _ = store.record_harness_capture(
                "grok",
                "grok",
                request.session_id.as_deref(),
                &reply,
                Some(&workspace.to_string_lossy()),
            );
            Ok(HarnessInjectResult {
                harness: "grok".into(),
                pid: None,
                status: "delivered".into(),
                detail: format!(
                    "forwarded to registered Grok session {session_id} via leader {}",
                    socket.display()
                ),
            })
        }
        Err(error) => Ok(unavailable(&error)),
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "grok".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;
    use tempfile::tempdir;

    #[test]
    fn missing_leader_is_unavailable_and_does_not_spawn_a_tui() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session(
                "grok",
                "/tmp/ca-grok-bridge",
                "019ff7ff-69a1-70e0-9d50-8e4544861f12",
                Some("/tmp/missing-ca-grok-leader.sock"),
            )
            .unwrap();
        let result = deliver_grok_task(
            &store,
            &HarnessInjectRequest {
                harness: "grok".into(),
                workspace: PathBuf::from("/tmp/ca-grok-bridge"),
                session_id: None,
                message_id: Some("msg-1".into()),
                body: "review the hub".into(),
                is_task: true,
                is_wake: false,
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert!(
            result.detail.contains("leader socket") || result.detail.contains("--leader"),
            "{}",
            result.detail
        );
        assert_eq!(result.pid, None);
    }

    #[test]
    fn registration_is_required_or_inferred_from_disk() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_grok_task(
            &store,
            &HarnessInjectRequest {
                harness: "grok".into(),
                workspace: PathBuf::from("/tmp/no-such-ca-workspace-xyz"),
                session_id: None,
                message_id: None,
                body: "hello".into(),
                is_task: true,
                is_wake: false,
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert!(
            result.detail.contains("no registered")
                || result.detail.contains("leader socket")
                || result.detail.contains("--leader")
        );
    }

    #[test]
    fn connect_rejects_a_relative_workspace() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let error =
            connect_grok_leader_session(&store, std::path::Path::new("relative/repo"), false)
                .expect_err("relative workspace");
        assert!(error.to_string().contains("absolute"));
    }
}
