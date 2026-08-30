//! Workspace-scoped harness liveness for Chat & Memory presence dots.
//!
//! Never derived from a global process-name scan. Claude uses the
//! Channel-bridge check for this workspace. Chat / Gemini / Grok claim
//! live for a Hub-registered session that is in an active state, aligned
//! with the #165 capture-identity gate: a *managed* session must still be
//! running (`managed_pid` alive) so a dead app-spawned process never shows
//! live; an *observed* session is considered present because the capture
//! identity gate is already mirroring its turns into the app (the "inactive
//! but messaging" symptom — observed was previously never claimed). Grok may
//! also claim live when a workspace-scoped `--leader` TUI is up *and* the
//! leader socket exists — that is the documented Hub inject path.

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

pub fn registered_session_is_present(session: &HarnessSessionRegistration) -> bool {
    if !matches!(
        session.state,
        HarnessSessionState::Ready | HarnessSessionState::Busy | HarnessSessionState::Queued
    ) {
        return false;
    }
    match session.mode {
        // A managed session we launched must still be running to be present —
        // otherwise a dead app-spawned process would show a live dot.
        HarnessSessionMode::Managed => session.managed_pid.is_some_and(is_pid_running),
        // An observed session is being tracked/captured by the capture-identity
        // gate (that's why its turns appear in the app), so align presence with
        // that and show it present rather than "inactive but messaging".
        HarnessSessionMode::Observed => true,
    }
}

fn any_managed_live(
    sessions: &[HarnessSessionRegistration],
    workspace: &Path,
    aliases: &[&str],
) -> bool {
    sessions.iter().any(|session| {
        aliases.contains(&session.harness.as_str())
            && same_workspace(&session.workspace, workspace)
            && registered_session_is_present(session)
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
    fn observed_registration_is_present() {
        // #165 option 1: an observed session is tracked/captured by the
        // capture-identity gate, so it shows present rather than the
        // "inactive but messaging" mismatch. It has no managed pid, so its
        // liveness is the registration's active state, not a pid check.
        let observed = session(
            "chat",
            HarnessSessionMode::Observed,
            HarnessSessionState::Ready,
            None,
        );
        assert!(registered_session_is_present(&observed));
    }

    #[test]
    fn managed_with_a_dead_pid_is_not_present() {
        let dead = session(
            "gemini",
            HarnessSessionMode::Managed,
            HarnessSessionState::Ready,
            Some(u32::MAX),
        );
        assert!(!registered_session_is_present(&dead));
    }

    #[test]
    fn stopped_or_unavailable_managed_is_not_present() {
        let stopped = session(
            "chat",
            HarnessSessionMode::Managed,
            HarnessSessionState::Stopped,
            Some(u32::MAX),
        );
        assert!(!registered_session_is_present(&stopped));
    }
}
