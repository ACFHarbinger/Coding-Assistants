//! Mistral Vibe CLI spawn argv (S6/S8). Split from `spawn.rs` for the
//! 500-LoC cap.
//!
//! Vibe's `--resume <id>` flag accepts the `meta.json.session_id` UUID —
//! not the session directory name. Managed runs pass that UUID so the
//! CLI continues the same on-disk transcript. The `--output streaming`
//! format (not `text`) is used for managed runs so structured events
//! are available to downstream consumers.

use crate::harness::{start_harness_owned, HarnessStartRequest, HarnessStartResult};
use crate::{HarnessSessionRegistration, HarnessSessionState, HubError, HubStore};
use std::ffi::OsString;
use std::path::Path;
use std::str::FromStr;

/// The on-disk Vibe session id for a Hub `(harness, workspace)` session id.
///
/// Vibe's `--resume` flag requires the `meta.json.session_id` UUID. The Hub
/// registers managed workers as `managed-<uuid>` (see
/// `bridge::relaunch::managed`). This maps a Hub session id to the UUID the
/// CLI accepts: a bare UUID passes through, and `managed-<uuid>` strips to
/// its trailing UUID. Anything else returns `None` so the caller fails
/// truthfully instead of spawning a fresh session the Hub believes is a
/// resume.
pub fn vibe_disk_session_id(session_id: &str) -> Option<String> {
    let trimmed = session_id.trim();
    if uuid::Uuid::from_str(trimmed).is_ok() {
        return Some(trimmed.to_string());
    }
    if let Some(trailing) = trimmed.strip_prefix("managed-") {
        if uuid::Uuid::from_str(trailing).is_ok() {
            return Some(trailing.to_string());
        }
    }
    None
}

/// Explicit argv for a Mistral Vibe wake/task spawn (session_id = None).
///
/// Delegates to [`vibe_managed_spawn_args`] with no session id, producing
/// `vibe -p <prompt> --workdir <workspace> --trust --output streaming --auto-approve`.
pub fn vibe_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    vibe_managed_spawn_args(workspace, prompt, None, model, effort)
}

/// Explicit argv for an app-managed Vibe worker run.
///
/// Managed runs pass `--resume <session_id>` so the CLI continues the
/// registered on-disk transcript. `--output streaming` (not `text`)
/// provides structured events for downstream consumers.
///
/// Verified live against `vibe 2.25.1` (2026-09-09): `--resume <uuid>`
/// continues the session with that `meta.json.session_id`.
pub fn vibe_managed_spawn_args(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
    _model: Option<&str>,
    _effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Vibe spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Vibe spawn workspace must be an absolute path".into(),
        ));
    }
    let mut args = Vec::new();
    if let Some(session_id) = session_id {
        let session_id = session_id.trim();
        if !session_id.is_empty() {
            let Some(disk_id) = vibe_disk_session_id(session_id) else {
                return Err(HubError::Invalid(format!(
                    "Vibe session id {session_id:?} is not a UUID and cannot be passed to `vibe --resume`"
                )));
            };
            args.push(OsString::from("--resume"));
            args.push(OsString::from(disk_id));
        }
    }
    args.push(OsString::from("-p"));
    args.push(OsString::from(prompt));
    args.push(OsString::from("--workdir"));
    args.push(workspace.as_os_str().to_os_string());
    args.push(OsString::from("--trust"));
    args.push(OsString::from("--output"));
    args.push(OsString::from("streaming"));
    args.push(OsString::from("--auto-approve"));
    Ok(args)
}

fn run_vibe_worker(workspace: &Path, prompt: &str) -> Result<Option<u32>, String> {
    let (started, child) = start_harness_owned(&HarnessStartRequest {
        harness: "vibe".into(),
        workspace: workspace.to_path_buf(),
        session_id: None,
        prompt: prompt.into(),
        ..Default::default()
    })
    .map_err(|error| error.to_string())?;
    // The durable Vibe transcript is the capture source; do not double-capture stdout.
    let _ = child.wait_with_output();
    Ok(started.pid)
}

/// Start a fresh managed Vibe worker and register the UUID Vibe wrote to
/// `meta.json`. A caller cannot choose that UUID, so this intentionally
/// invokes Vibe without `--resume`.
pub fn start_vibe_managed_harness(
    store: &HubStore,
    workspace: &Path,
    prompt: &str,
) -> Result<(HarnessStartResult, HarnessSessionRegistration), String> {
    start_vibe_managed_harness_from(
        store,
        workspace,
        prompt,
        run_vibe_worker,
        &crate::bridge::vibe::vibe_logs_root(),
    )
}

pub(crate) fn start_vibe_managed_harness_from(
    store: &HubStore,
    workspace: &Path,
    prompt: &str,
    runner: impl FnOnce(&Path, &str) -> Result<Option<u32>, String>,
    sessions_root: &Path,
) -> Result<(HarnessStartResult, HarnessSessionRegistration), String> {
    if !workspace.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let workspace_key = workspace.to_string_lossy().into_owned();

    if let Ok(Some(existing)) = store.get_harness_session("vibe", &workspace_key) {
        if let Some(pid) = existing.managed_pid {
            crate::bridge::relaunch::kill_pid(pid);
        }
    }

    let pid = runner(&workspace, prompt)?;
    let disk_id = crate::bridge::vibe::latest_vibe_session_id_from(
        sessions_root,
        &workspace,
        200,
    )
    .ok_or_else(|| {
        "vibe started but no matching session transcript was found; managed session not registered"
            .to_string()
    })?;

    let registration = store
        .register_managed_harness_session_with_state(
            "vibe",
            &workspace_key,
            &disk_id,
            pid,
            HarnessSessionState::Ready,
        )
        .map_err(|error| error.to_string())?;

    Ok((
        HarnessStartResult {
            harness: "vibe".into(),
            pid,
            status: "started".into(),
            detail: format!("Vibe managed session started (session {disk_id})"),
        },
        registration,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    const WS: &str = "/tmp/coding-assistants-c14-vibe";

    #[test]
    fn vibe_spawn_args_is_explicit_and_rejects_relative_workspace() {
        let args = vibe_spawn_args(Path::new(WS), "review", None, None).unwrap();
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "review");
        assert_eq!(args[2], "--workdir");
        assert_eq!(args[3], WS);
        assert_eq!(args[4], "--trust");
        assert_eq!(args[5], "--output");
        assert_eq!(args[6], "streaming");
        assert_eq!(args[7], "--auto-approve");

        assert!(vibe_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(vibe_spawn_args(Path::new(WS), "  ", None, None).is_err());
    }

    #[test]
    fn vibe_managed_spawn_args_uses_resume_for_real_session_ids_only() {
        let uuid = "123e4567-e89b-42d3-a456-426614174000";
        let managed = format!("managed-{uuid}");

        // Bare UUID → --resume <uuid>
        let resumed =
            vibe_managed_spawn_args(Path::new(WS), "continue", Some(uuid), None, None).unwrap();
        assert_eq!(resumed[0], "--resume");
        assert_eq!(resumed[1], uuid);
        assert_eq!(resumed[2], "-p");

        // managed-<uuid> → strips prefix
        let resumed_managed =
            vibe_managed_spawn_args(Path::new(WS), "continue", Some(&managed), None, None).unwrap();
        assert_eq!(resumed_managed[0], "--resume");
        assert_eq!(resumed_managed[1], uuid);

        // Non-UUID → error
        assert!(
            vibe_managed_spawn_args(Path::new(WS), "continue", Some("chat-1"), None, None).is_err()
        );

        // None → no --resume
        let fresh = vibe_managed_spawn_args(Path::new(WS), "continue", None, None, None).unwrap();
        assert!(!fresh.iter().any(|arg| arg == "--resume"));
    }

    #[test]
    fn vibe_disk_session_id_strips_managed_prefix() {
        let uuid = "123e4567-e89b-42d3-a456-426614174000";
        assert_eq!(vibe_disk_session_id(uuid), Some(uuid.to_string()));
        assert_eq!(
            vibe_disk_session_id(&format!("managed-{uuid}")),
            Some(uuid.to_string())
        );
        assert_eq!(vibe_disk_session_id("not-a-uuid"), None);
        assert_eq!(vibe_disk_session_id(""), None);
        assert_eq!(vibe_disk_session_id("managed-not-a-uuid"), None);
    }

    #[test]
    fn managed_start_registers_discovered_vibe_uuid() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("sessions");
        let store = HubStore::open(tmp.path()).unwrap();
        let workspace = PathBuf::from(WS);
        let uuid = "123e4567-e89b-42d3-a456-426614174000";
        let (started, registration) = start_vibe_managed_harness_from(
            &store,
            &workspace,
            "hello",
            |_workspace, _prompt| {
                let session = root.join("session_test");
                fs::create_dir_all(&session).unwrap();
                fs::write(
                    session.join("meta.json"),
                    serde_json::json!({
                        "session_id": uuid,
                        "environment": { "working_directory": WS }
                    })
                    .to_string(),
                )
                .unwrap();
                fs::write(session.join("messages.jsonl"), "event\n").unwrap();
                Ok(Some(4242))
            },
            &root,
        )
        .unwrap();
        assert_eq!(started.pid, Some(4242));
        assert_eq!(registration.disk_session_id, uuid);
        assert_eq!(registration.mode, crate::HarnessSessionMode::Managed);
    }
}
