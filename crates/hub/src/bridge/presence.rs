//! Workspace-scoped harness liveness for Chat & Memory presence dots.
//!
//! Never derived from a global process-name scan. Claude uses the
//! Channel-bridge check for this workspace. Chat / Gemini / Grok claim
//! live only for a Hub-registered *managed* session whose `managed_pid`
//! is still running. Observed registrations stay unclaimed. Grok may
//! also claim live when a workspace-scoped `--leader` TUI is up *and*
//! the leader socket exists — that is the documented Hub inject path.

use crate::bridge::channels::claude::is_channel_session_live;
use crate::bridge::grok::grok_leader_status;
use crate::bridge::relaunch::is_pid_running;
use crate::{HarnessSessionMode, HarnessSessionRegistration, HarnessSessionState, HubStore};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct WorkspaceAgentPresence {
    pub claude: bool,
    pub chat: bool,
    pub gemini: bool,
    pub grok: bool,
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn same_workspace(registered: &str, requested: &Path) -> bool {
    canonical(Path::new(registered)) == canonical(requested)
}

const CHAT_ALIASES: &[&str] = &["chat", "codex"];
const GEMINI_ALIASES: &[&str] = &["gemini", "agy"];
const GROK_ALIASES: &[&str] = &["grok"];

pub fn managed_session_is_live(session: &HarnessSessionRegistration) -> bool {
    session.mode == HarnessSessionMode::Managed
        && matches!(
            session.state,
            HarnessSessionState::Ready | HarnessSessionState::Busy | HarnessSessionState::Queued
        )
        && session.managed_pid.is_some_and(is_pid_running)
}

fn any_managed_live(
    sessions: &[HarnessSessionRegistration],
    workspace: &Path,
    aliases: &[&str],
) -> bool {
    sessions.iter().any(|session| {
        aliases.contains(&session.harness.as_str())
            && same_workspace(&session.workspace, workspace)
            && managed_session_is_live(session)
    })
}

pub fn workspace_agent_presence(
    store: &HubStore,
    workspace: &Path,
) -> Result<WorkspaceAgentPresence, String> {
    let sessions = store
        .list_harness_sessions()
        .map_err(|error| error.to_string())?;
    let grok_leader = grok_leader_status(Some(workspace));
    Ok(WorkspaceAgentPresence {
        claude: is_channel_session_live(workspace).unwrap_or(false),
        chat: any_managed_live(&sessions, workspace, CHAT_ALIASES),
        gemini: any_managed_live(&sessions, workspace, GEMINI_ALIASES),
        grok: any_managed_live(&sessions, workspace, GROK_ALIASES)
            || (grok_leader.leader_live && grok_leader.live_standalone.is_some()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(
        harness: &str,
        mode: HarnessSessionMode,
        state: HarnessSessionState,
        pid: Option<u32>,
    ) -> HarnessSessionRegistration {
        HarnessSessionRegistration {
            harness: harness.into(),
            workspace: "/abs/repo".into(),
            disk_session_id: "sess".into(),
            leader_socket: None,
            registered_at: String::new(),
            mode,
            state,
            managed_pid: pid,
            writer_owner: None,
            writer_acquired_at: None,
        }
    }

    #[test]
    fn observed_registration_is_never_live() {
        let observed = session(
            "chat",
            HarnessSessionMode::Observed,
            HarnessSessionState::Ready,
            None,
        );
        assert!(!managed_session_is_live(&observed));
    }

    #[test]
    fn managed_with_a_dead_pid_is_not_live() {
        let dead = session(
            "gemini",
            HarnessSessionMode::Managed,
            HarnessSessionState::Ready,
            Some(u32::MAX),
        );
        assert!(!managed_session_is_live(&dead));
    }

    #[test]
    fn stopped_or_unavailable_managed_is_not_live() {
        let stopped = session(
            "chat",
            HarnessSessionMode::Managed,
            HarnessSessionState::Stopped,
            Some(u32::MAX),
        );
        assert!(!managed_session_is_live(&stopped));
    }
}
