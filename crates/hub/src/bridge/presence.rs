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
    pub opencode: bool,
    pub deepseek: bool,
    /// Harness id `"vibe"`; the roster/agent id is `"mistral"` (see
    /// `HarnessId::parse`'s `"vibe" | "mistral"` alias) — the frontend maps
    /// both keys onto this one field.
    pub vibe: bool,
    pub muse: bool,
    pub cursor: bool,
    pub qwen: bool,
    pub kimi: bool,
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
const OPENCODE_ALIASES: &[&str] = &["opencode"];
const DEEPSEEK_ALIASES: &[&str] = &["deepseek"];
const VIBE_ALIASES: &[&str] = &["vibe"];
const MUSE_ALIASES: &[&str] = &["muse"];
const CURSOR_ALIASES: &[&str] = &["cursor"];
const QWEN_ALIASES: &[&str] = &["qwen"];
const KIMI_ALIASES: &[&str] = &["kimi"];

pub fn registered_session_is_present(session: &HarnessSessionRegistration) -> bool {
    if !matches!(
        session.state,
        HarnessSessionState::Ready | HarnessSessionState::Busy | HarnessSessionState::Queued
    ) {
        return false;
    }
    match session.mode {
        // A managed worker's pid is only its current task process. A clean
        // one-shot exit leaves a managed provider session queued for its next
        // task, so absence of a pid must not turn that live session grey.
        // Explicit stop/unavailable states above remain authoritative.
        HarnessSessionMode::Managed => {
            if let Some(alive) = session.pid_alive {
                alive
            } else {
                session.managed_pid.is_none_or(is_pid_running)
            }
        }
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
        opencode: any_managed_live(&sessions, workspace, OPENCODE_ALIASES),
        deepseek: any_managed_live(&sessions, workspace, DEEPSEEK_ALIASES),
        vibe: any_managed_live(&sessions, workspace, VIBE_ALIASES),
        muse: any_managed_live(&sessions, workspace, MUSE_ALIASES),
        cursor: any_managed_live(&sessions, workspace, CURSOR_ALIASES),
        qwen: any_managed_live(&sessions, workspace, QWEN_ALIASES),
        kimi: any_managed_live(&sessions, workspace, KIMI_ALIASES),
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
            pid_alive: None,
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

    #[test]
    fn queued_managed_session_without_a_current_worker_is_still_present() {
        let queued = session(
            "gemini",
            HarnessSessionMode::Managed,
            HarnessSessionState::Queued,
            None,
        );
        assert!(registered_session_is_present(&queued));
    }

    /// Regression: `WorkspaceAgentPresence` originally only carried the four
    /// original harnesses (claude/chat/gemini/grok), so every harness
    /// onboarded since (opencode/deepseek/vibe/muse/cursor/qwen/kimi) always
    /// showed a grey "not connected" dot regardless of real session state —
    /// `any_managed_live` was already fully generic, the struct just never
    /// called it for them. Qwen is the reported case; the alias wiring is
    /// identical for the rest, so one live-and-one-absent pair per new
    /// field is sufficient coverage.
    #[test]
    fn newer_harnesses_report_live_through_the_same_generic_path() {
        let workspace = Path::new("/abs/repo");
        let ready_qwen = session(
            "qwen",
            HarnessSessionMode::Managed,
            HarnessSessionState::Ready,
            Some(std::process::id()),
        );
        assert!(any_managed_live(
            std::slice::from_ref(&ready_qwen),
            workspace,
            QWEN_ALIASES
        ));
        // A qwen session must not satisfy an unrelated alias set.
        assert!(!any_managed_live(
            std::slice::from_ref(&ready_qwen),
            workspace,
            KIMI_ALIASES
        ));
        assert!(!any_managed_live(&[], workspace, QWEN_ALIASES));
    }
}
