//! C12-CLAUDE-BRIDGE: discover an already-running Claude Code session for a
//! task-delivery bridge, without spawning a replacement `claude -p` process
//! or writing to any PTY.
//!
//! Claude Code exposes a real, documented registry of active sessions:
//! `claude agents --json` (see `claude --help` → `agents [options]  Manage
//! background agents` → `--json  Print active sessions (interactive and
//! background) as a JSON array and exit`). It lists every running session —
//! interactive or background — with its pid, cwd, session id, and status.
//! Verified directly on a real machine running this very code: the command
//! lists the session this file was written in.
//!
//! Each active session also listens on a real Unix control socket at
//! `$XDG_RUNTIME_DIR/cc-socks/<pid>.sock` — confirmed with `lsof -U` against
//! that same live pid. Unlike Codex's `app-server` (a documented JSON-RPC
//! daemon) this socket's wire protocol is Claude Code's internal
//! implementation detail; nothing in `claude --help` documents it. Blindly
//! connecting to it and writing arbitrary bytes into a live interactive
//! session's control channel — with no way to verify the outcome — is not a
//! safe automated action, so this bridge never attempts it. Delivery always
//! resolves to a clearly explained `unavailable` (the task stays queued),
//! the same safety shape used when a bridge socket is missing entirely —
//! except every claim here is backed by something actually observed on a
//! live system, not an assumed/undocumented endpoint.

use crate::{HarnessInjectRequest, HarnessInjectResult, HubError, HubStore};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One entry from `claude agents --json`.
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeAgentSession {
    pub pid: u32,
    pub cwd: String,
    pub kind: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub status: String,
}

/// Real, documented discovery: `claude agents --json`. Never mutates
/// anything — a read-only listing of already-running sessions.
pub fn list_active_claude_sessions() -> Result<Vec<ClaudeAgentSession>, String> {
    let output = Command::new("claude")
        .args(["agents", "--json"])
        .output()
        .map_err(|error| format!("could not run `claude agents --json`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`claude agents --json` exited with {}",
            output.status
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("unexpected `claude agents --json` output: {error}"))
}

/// Matches a listed session to a workspace by exact `cwd`.
pub fn find_active_claude_session(
    sessions: &[ClaudeAgentSession],
    workspace: &Path,
) -> Option<ClaudeAgentSession> {
    let workspace = workspace.to_string_lossy();
    sessions
        .iter()
        .find(|session| session.cwd == workspace)
        .cloned()
}

/// The real control socket path every active Claude Code session listens
/// on, as observed via `lsof -U` on a live pid: `$XDG_RUNTIME_DIR/cc-socks/<pid>.sock`.
/// Returns `None` when `$XDG_RUNTIME_DIR` isn't set (no Linux user session
/// runtime directory to look under).
pub fn claude_control_socket_path(pid: u32) -> Option<PathBuf> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
    Some(
        PathBuf::from(runtime_dir)
            .join("cc-socks")
            .join(format!("{pid}.sock")),
    )
}

/// Deliver a task into a registered Claude Code session. Never writes a
/// TTY/PTY and never starts a replacement `claude -p` process — see the
/// module docs for why real delivery is not attempted even when a live
/// session and its control socket are both found.
pub fn deliver_claude_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_claude_task_with(list_active_claude_sessions, store, request)
}

/// Testable core: takes the session lister as a parameter so tests never
/// shell out to a real `claude` binary or depend on this machine's live
/// session state.
fn deliver_claude_task_with(
    list_sessions: impl Fn() -> Result<Vec<ClaudeAgentSession>, String>,
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Claude active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());

    let registration = store.get_harness_session("claude", &workspace.to_string_lossy())?;
    let live_session = list_sessions()
        .ok()
        .and_then(|sessions| find_active_claude_session(&sessions, &workspace));

    let Some(pid) = live_session.as_ref().map(|session| session.pid) else {
        let hint = if registration.is_some() {
            " (a prior registration exists but no live session currently matches it)"
        } else {
            ""
        };
        return Ok(unavailable(&format!(
            "no active Claude Code session found for {} via `claude agents --json`{hint}. Task stays queued.",
            workspace.display()
        )));
    };

    match claude_control_socket_path(pid) {
        Some(socket) if socket.exists() => Ok(unavailable(&format!(
            "registered Claude Code session (pid {pid}) has a live control socket at {} — its wire \
             protocol is undocumented Claude Code internals, so automated delivery is not attempted. \
             Task stays queued; deliver it in that session manually.",
            socket.display()
        ))),
        _ => Ok(unavailable(&format!(
            "registered Claude Code session (pid {pid}) has no reachable control socket. Task stays queued."
        ))),
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "claude".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn no_sessions() -> Result<Vec<ClaudeAgentSession>, String> {
        Ok(Vec::new())
    }

    #[test]
    fn find_active_claude_session_matches_by_exact_cwd() {
        let sessions = vec![ClaudeAgentSession {
            pid: 12345,
            cwd: "/tmp/ca-claude-bridge".into(),
            kind: "interactive".into(),
            session_id: "session-uuid-1".into(),
            status: "busy".into(),
        }];
        let found = find_active_claude_session(&sessions, Path::new("/tmp/ca-claude-bridge"));
        assert_eq!(found.map(|s| s.pid), Some(12345));
        assert!(find_active_claude_session(&sessions, Path::new("/tmp/other")).is_none());
    }

    #[test]
    fn no_live_session_is_unavailable_and_never_spawns_a_replacement() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_claude_task_with(
            no_sessions,
            &store,
            &HarnessInjectRequest {
                harness: "claude".into(),
                workspace: PathBuf::from("/tmp/ca-claude-bridge-no-session"),
                session_id: None,
                message_id: Some("msg-claude-1".into()),
                body: "review the hub".into(),
                is_task: true,
                is_wake: false,
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert!(result.detail.contains("no active Claude Code session"));
        assert_eq!(result.pid, None);
    }

    #[test]
    fn a_prior_registration_is_surfaced_when_no_live_session_matches() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session(
                "claude",
                "/tmp/ca-claude-bridge-stale",
                "old-disk-session",
                None,
            )
            .unwrap();
        let result = deliver_claude_task_with(
            no_sessions,
            &store,
            &HarnessInjectRequest {
                harness: "claude".into(),
                workspace: PathBuf::from("/tmp/ca-claude-bridge-stale"),
                session_id: None,
                message_id: None,
                body: "hello".into(),
                is_task: true,
                is_wake: false,
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert!(result.detail.contains("prior registration exists"));
    }

    #[test]
    fn live_session_with_no_reachable_socket_is_unavailable() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        // A pid this unlikely to have a real /run/user/<uid>/cc-socks entry.
        let fake_pid = 999_999;
        let list = move || {
            Ok(vec![ClaudeAgentSession {
                pid: fake_pid,
                cwd: "/tmp/ca-claude-bridge-live".into(),
                kind: "interactive".into(),
                session_id: "session-uuid-live".into(),
                status: "busy".into(),
            }])
        };
        let result = deliver_claude_task_with(
            list,
            &store,
            &HarnessInjectRequest {
                harness: "claude".into(),
                workspace: PathBuf::from("/tmp/ca-claude-bridge-live"),
                session_id: None,
                message_id: None,
                body: "do the task".into(),
                is_task: true,
                is_wake: false,
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.pid, None);
    }

    #[test]
    fn empty_body_and_relative_workspace_are_rejected() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let empty_body = deliver_claude_task_with(
            no_sessions,
            &store,
            &HarnessInjectRequest {
                harness: "claude".into(),
                workspace: PathBuf::from("/tmp/ca-claude-bridge"),
                session_id: None,
                message_id: None,
                body: "   ".into(),
                is_task: true,
                is_wake: false,
            },
        );
        assert!(empty_body.is_err());

        let relative_workspace = deliver_claude_task_with(
            no_sessions,
            &store,
            &HarnessInjectRequest {
                harness: "claude".into(),
                workspace: PathBuf::from("relative/workspace"),
                session_id: None,
                message_id: None,
                body: "do the task".into(),
                is_task: true,
                is_wake: false,
            },
        );
        assert!(relative_workspace.is_err());
    }
}
