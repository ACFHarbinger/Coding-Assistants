//! Kimi Code CLI in-session slash-command task modes (#363).
//!
//! Kimi Code CLI documents two in-session slash commands the app can drive
//! for free from the existing one-shot delivery path
//! (`kimi -p <body> --session <id> --output-format stream-json`):
//!
//! - `/swarm <task>` — turns swarm mode on for one completion, dispatching
//!   onto Kimi's built-in coder/explore/plan subagents; auto-disables after.
//! - `/goal <objective>` — a persistent objective across auto-continuing
//!   turns (documented exit codes: 0 complete / 3 blocked / 6 paused).
//!
//! A mode only *prefixes a plain-language task body* before it reaches
//! [super::kimi::deliver_kimi_task_with]; the spawn shape is unchanged and
//! there is no new bridge function. Default is OFF (`None`).
//!
//! `/goal`'s control subcommands (`status` / `pause` / `resume` / `cancel` /
//! `replace`) are deliberately **not** a toggle surface: they are driven by
//! sending the explicit slash command as the task body (e.g. body
//! `/goal status`), and [apply_kimi_task_mode] never rewrites a body that
//! already starts with `/`.

use serde::{Deserialize, Serialize};

/// Per-task Kimi slash-command mode (#363).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KimiTaskMode {
    /// Prefix the body with `/swarm ` (one swarm-mode completion).
    Swarm,
    /// Prefix the body with `/goal ` (persistent objective).
    Goal,
}

impl KimiTaskMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Swarm => "swarm",
            Self::Goal => "goal",
        }
    }

    /// Parse a settings/CLI string (`swarm` or `goal`).
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim() {
            "swarm" => Ok(Self::Swarm),
            "goal" => Ok(Self::Goal),
            other => Err(format!(
                "unknown kimi mode {other:?} (expected \"swarm\" or \"goal\")"
            )),
        }
    }

    /// The slash command this mode prefixes onto a plain task body.
    pub fn slash_command(self) -> &'static str {
        match self {
            Self::Swarm => "/swarm",
            Self::Goal => "/goal",
        }
    }
}

/// Resolve the effective mode: an explicit per-task choice always wins over
/// the per-workspace default; `None` (the default) means no prefix.
pub fn resolve_kimi_task_mode(
    per_task: Option<KimiTaskMode>,
    workspace_default: Option<KimiTaskMode>,
) -> Option<KimiTaskMode> {
    per_task.or(workspace_default)
}

/// Apply the mode's slash-command prefix to a task body.
///
/// Pass-through rule: a body whose first non-whitespace character is `/`
/// is returned unchanged — the toggle never rewrites an explicit slash
/// command. That keeps `/goal status` (and any future slash command)
/// reachable per task even when a workspace default mode is set.
pub fn apply_kimi_task_mode(mode: Option<KimiTaskMode>, body: &str) -> String {
    let trimmed = body.trim();
    let Some(mode) = mode else {
        return body.to_string();
    };
    if trimmed.starts_with('/') {
        return body.to_string();
    }
    format!("{} {}", mode.slash_command(), trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_documented_modes_and_rejects_unknown() {
        assert_eq!(KimiTaskMode::parse("swarm"), Ok(KimiTaskMode::Swarm));
        assert_eq!(KimiTaskMode::parse("goal"), Ok(KimiTaskMode::Goal));
        assert!(KimiTaskMode::parse("flock").is_err());
        assert!(KimiTaskMode::parse("").is_err());
    }

    #[test]
    fn serde_roundtrip_uses_snake_case() {
        let json = serde_json::to_string(&KimiTaskMode::Swarm).unwrap();
        assert_eq!(json, "\"swarm\"");
        let back: KimiTaskMode = serde_json::from_str("\"goal\"").unwrap();
        assert_eq!(back, KimiTaskMode::Goal);
    }

    #[test]
    fn off_by_default_leaves_body_untouched() {
        assert_eq!(
            apply_kimi_task_mode(None, "refactor the parser"),
            "refactor the parser"
        );
    }

    #[test]
    fn modes_prefix_plain_bodies() {
        assert_eq!(
            apply_kimi_task_mode(Some(KimiTaskMode::Swarm), "map the repo"),
            "/swarm map the repo"
        );
        assert_eq!(
            apply_kimi_task_mode(Some(KimiTaskMode::Goal), "  keep CI green  "),
            "/goal keep CI green"
        );
    }

    #[test]
    fn explicit_slash_commands_pass_through_under_a_mode() {
        // `/goal` control subcommands stay reachable: the toggle never
        // rewrites an explicit slash command.
        assert_eq!(
            apply_kimi_task_mode(Some(KimiTaskMode::Swarm), "/goal status"),
            "/goal status"
        );
        assert_eq!(
            apply_kimi_task_mode(Some(KimiTaskMode::Goal), "/goal pause"),
            "/goal pause"
        );
        assert_eq!(
            apply_kimi_task_mode(Some(KimiTaskMode::Swarm), "  /compact"),
            "  /compact"
        );
    }

    #[test]
    fn delivery_prefixes_body_only_when_mode_is_set() {
        use crate::{HarnessInjectRequest, HarnessSessionState, HubStore};
        use std::path::Path;
        use std::sync::{Arc, Mutex};

        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = Path::new("/tmp/kimi-363-mode-test");
        let workspace_key = workspace.to_string_lossy().into_owned();
        store
            .register_managed_harness_session_with_state(
                "kimi",
                &workspace_key,
                "session_363",
                None,
                HarnessSessionState::Ready,
            )
            .unwrap();

        let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let deliver = |mode: Option<KimiTaskMode>, body: &str| {
            let seen = Arc::clone(&seen);
            let request = HarnessInjectRequest {
                harness: "kimi".into(),
                workspace: workspace.to_path_buf(),
                session_id: Some("session_363".into()),
                message_id: None,
                body: body.into(),
                is_task: true,
                is_wake: false,
                model: None,
                effort: None,
                kimi_mode: mode,
            };
            let result = crate::bridge::kimi::deliver_kimi_task_with(
                &store,
                &request,
                move |_ws, prompt, _session, _model, _effort| {
                    seen.lock().unwrap().push(prompt.to_string());
                    Ok(Some(7))
                },
            )
            .unwrap();
            assert_eq!(result.status, "delivered");
        };

        deliver(None, "map the repo");
        deliver(Some(KimiTaskMode::Swarm), "map the repo");
        deliver(Some(KimiTaskMode::Goal), "keep CI green");
        // Explicit slash commands pass through even under a mode.
        deliver(Some(KimiTaskMode::Swarm), "/goal status");

        assert_eq!(
            seen.lock().unwrap().as_slice(),
            [
                "map the repo",
                "/swarm map the repo",
                "/goal keep CI green",
                "/goal status"
            ]
        );
    }

    #[test]
    fn per_task_mode_wins_over_workspace_default() {
        assert_eq!(
            resolve_kimi_task_mode(Some(KimiTaskMode::Goal), Some(KimiTaskMode::Swarm)),
            Some(KimiTaskMode::Goal)
        );
        assert_eq!(
            resolve_kimi_task_mode(None, Some(KimiTaskMode::Swarm)),
            Some(KimiTaskMode::Swarm)
        );
        assert_eq!(resolve_kimi_task_mode(None, None), None);
    }
}
