//! Relaunch and managed-harness Tauri commands: `hub_relaunch_harness_*`
//! (resume a harness in a real terminal or an in-app PTY) and
//! `hub_start_managed_harness`.
//!
//! Split out of `harness::commands` to keep every source unit within the
//! 500-line cap (#158). The shared kill/resolve logic lives in
//! `hub::bridge::relaunch`.

use super::sandbox_strictness_blocks;
use crate::commands::commands::store::open_store;
use crate::pty::{self, PtySessions};
use hub::{
    apply_grok_embedded_scroll_flags, cursor_executable, interactive_resume_args,
    relaunch_harness_in_terminal, resolve_interactive_relaunch, start_managed_claude_channel,
    start_managed_harness, HarnessId, HarnessSessionRegistration, HarnessStartResult,
    RelaunchOutcome, ResolvedRelaunch,
};
use std::path::Path;
use tauri::{AppHandle, State};

/// Validates that an optional instance key meets safety and identity requirements:
/// 1-64 characters, ASCII alphanumeric, hyphen, or underscore only.
pub fn validate_instance_key(key: &str) -> Result<(), &'static str> {
    if key.is_empty() {
        return Err("instance_key must not be empty");
    }
    if key.len() > 64 {
        return Err("instance_key must not exceed 64 characters");
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "instance_key must contain only ASCII alphanumeric, hyphen, or underscore characters",
        );
    }
    Ok(())
}

/// Kill an optional managed pid, then open a real terminal running this
/// harness's interactive CLI (resumed from the latest on-disk session
/// when one exists). This is the human-attended path, not the headless
/// one-shot spawn used by `hub_start_harness`.
///
/// Async: the resolve step can be slow (a `claude agents --json`
/// subprocess, a Codex session-tree scan, kill + settle sleeps) and must
/// not occupy a Tokio worker that other IPC commands need — the #161 /
/// #163 "clicked, nothing happened / UI froze" class.
#[tauri::command]
pub async fn hub_relaunch_harness_in_terminal(
    harness: String,
    workspace: String,
    existing_pid: Option<u32>,
) -> Result<RelaunchOutcome, String> {
    tokio::task::spawn_blocking(move || {
        relaunch_harness_in_terminal(&harness, Path::new(&workspace), existing_pid)
    })
    .await
    .map_err(|join| format!("relaunch worker panicked: {join}"))?
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedRelaunchOutcome {
    pub harness: String,
    pub killed_pid: Option<u32>,
    pub resumed_session_id: Option<String>,
    pub session_id: String,
    pub detail: String,
}

/// Same kill/resume-resolution as `hub_relaunch_harness_in_terminal`, but
/// spawns the harness's interactive CLI into an in-app PTY (see `pty.rs`)
/// instead of a separate terminal-emulator window. The frontend attaches
/// to `pty-output:<session_id>` / `pty-exit:<session_id>` to render it.
///
/// If `instance_key` is provided (#301), this pane represents an independent
/// terminal instance: the session ID is `harness-terminal:<harness>:<workspace>:<key>`,
/// prior PIDs are not killed, transcript discovery is bypassed, and a fresh CLI session
/// starts immediately without cross-session collision.
///
/// Async: see `hub_relaunch_harness_in_terminal` — the resolve step
/// (subprocess discovery, kill + settle sleeps) runs on the blocking pool,
/// never on a Tokio worker. The PTY spawn itself stays on the async
/// handler because it needs the Tauri State.
#[tauri::command]
pub async fn hub_relaunch_harness_embedded(
    app: AppHandle,
    pty_state: State<'_, PtySessions>,
    harness: String,
    workspace: String,
    existing_pid: Option<u32>,
    instance_key: Option<String>,
) -> Result<EmbeddedRelaunchOutcome, String> {
    let (session_id, resolved) = if let Some(ref key) = instance_key {
        validate_instance_key(key).map_err(|e| format!("invalid instance_key: {e}"))?;
        if !Path::new(&workspace).is_absolute() {
            return Err("workspace must be an absolute path".into());
        }
        let parsed = HarnessId::parse(&harness).map_err(|e| e.to_string())?;
        let program = if parsed == HarnessId::Cursor {
            cursor_executable()
        } else {
            parsed.executable()
        };
        let args = interactive_resume_args(parsed, None);
        let session_id = format!("harness-terminal:{}:{}:{}", harness, workspace, key);
        let resolved = ResolvedRelaunch {
            harness: parsed,
            killed_pid: None,
            resumed_session_id: None,
            program,
            args,
            discovery_timed_out: false,
        };
        (session_id, resolved)
    } else {
        let session_id = format!("harness-terminal:{}:{}", harness, workspace);
        let resolved = tokio::task::spawn_blocking({
            let harness = harness.clone();
            let workspace = workspace.clone();
            move || resolve_interactive_relaunch(&harness, Path::new(&workspace), existing_pid)
        })
        .await
        .map_err(|join| format!("relaunch worker panicked: {join}"))??;
        (session_id, resolved)
    };

    let mut spawn_args = resolved.args.clone();
    if resolved.harness == HarnessId::Grok {
        apply_grok_embedded_scroll_flags(&mut spawn_args);
    }

    pty::pty_spawn(
        app,
        pty_state,
        session_id.clone(),
        resolved.program.to_string(),
        spawn_args,
        workspace,
        24,
        80,
    )?;

    let ResolvedRelaunch {
        harness: harness_id,
        killed_pid,
        resumed_session_id,
        program,
        discovery_timed_out,
        ..
    } = resolved;

    let mut detail = match &resumed_session_id {
        Some(id) => format!("Resumed {program} in-app, session {id}"),
        None => {
            if instance_key.is_some() {
                format!("Started a fresh {program} instance session in-app")
            } else {
                format!(
                    "Started a fresh {program} session in-app (no prior session found to resume)"
                )
            }
        }
    };
    if discovery_timed_out {
        detail.push_str(" Session discovery timed out, so this is a fresh session, not a resume.");
    }
    Ok(EmbeddedRelaunchOutcome {
        harness: harness_id.as_str().into(),
        killed_pid,
        resumed_session_id,
        session_id,
        detail,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartManagedHarnessOutcome {
    pub start: HarnessStartResult,
    pub registration: HarnessSessionRegistration,
}

/// "Start managed": for Claude this opens a Channel-connected terminal
/// (kill-prior first) instead of the one-shot `claude -p` spawn, which
/// exits before any task can be delivered. Other harnesses still spawn a
/// headless worker and register it, killing any prior managed pid first.
/// Process spawn + discovery runs off the IPC thread (#163).
#[tauri::command]
pub async fn hub_start_managed_harness(
    harness: String,
    workspace: String,
    disk_session_id: String,
    prompt: String,
) -> Result<StartManagedHarnessOutcome, String> {
    crate::harness::blocking::run_blocking("hub_start_managed_harness", move || {
        if sandbox_strictness_blocks(&harness, &workspace) {
            return Err(format!(
                "{harness} requires bypassing approval and is blocked by this workspace's strict sandbox policy"
            ));
        }
        let parsed = hub::HarnessId::parse(&harness).map_err(|error| error.to_string())?;
        let store = open_store()?;
        let (start, registration) = if parsed == hub::HarnessId::Claude {
            start_managed_claude_channel(&store, Path::new(&workspace))?
        } else {
            start_managed_harness(
                &store,
                &harness,
                Path::new(&workspace),
                &disk_session_id,
                &prompt,
            )?
        };
        Ok(StartManagedHarnessOutcome {
            start,
            registration,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_instance_key_valid() {
        assert!(validate_instance_key("leaf-gemini-m7u8-1").is_ok());
        assert!(validate_instance_key("leaf_1").is_ok());
        assert!(validate_instance_key("a-b_c").is_ok());
        assert!(validate_instance_key(&"x".repeat(64)).is_ok());
    }

    #[test]
    fn test_validate_instance_key_invalid() {
        assert!(validate_instance_key("").is_err());
        assert!(validate_instance_key(&"x".repeat(65)).is_err());
        assert!(validate_instance_key("leaf with spaces").is_err());
        assert!(validate_instance_key("leaf/1").is_err());
        assert!(validate_instance_key("leaf:1").is_err());
        assert!(validate_instance_key("leaf..1").is_err());
        assert!(validate_instance_key("leaf@1").is_err());
    }
}
