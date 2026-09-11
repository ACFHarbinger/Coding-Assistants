//! Qwen Code CLI spawn argv (C14.13 / #308). Split from `spawn.rs` for the
//! 500-LoC cap.
//!
//! Verified live against `qwen` 0.23.2: positional prompt (not deprecated
//! `-p`), `-o/--output-format stream-json`, `--chat-recording` (required or
//! `-c`/`-r` are no-ops), `--session-id <uuid>` pre-assigns the transcript
//! id. Headless runs pass `-y` so the worker cannot hang on a TTY approval
//! prompt with stdin closed. Interactive resume is a separate argv in
//! `bridge::relaunch` (`--resume` + `--chat-recording`, no `-y`).

use crate::HubError;
use std::ffi::OsString;
use std::path::Path;
use std::str::FromStr;

/// The on-disk Qwen session id for a Hub `(harness, workspace)` session id.
///
/// `qwen --session-id` requires a UUID. The Hub registers managed workers as
/// `managed-<uuid>` (see `bridge::relaunch::managed`). A bare UUID passes
/// through; `managed-<uuid>` strips to the trailing UUID. Anything else
/// returns `None` so the caller fails truthfully instead of spawning a
/// fresh session the Hub believes is a resume.
pub fn qwen_disk_session_id(session_id: &str) -> Option<String> {
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

/// Explicit argv for a Qwen Code wake/task spawn (no `--session-id`).
pub fn qwen_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    qwen_managed_spawn_args(workspace, prompt, None, model, effort)
}

/// Explicit argv for an app-managed Qwen Code worker run.
///
/// `--session-id <uuid>` (normalized via [`qwen_disk_session_id`]) pins the
/// transcript the capture adapter later reads. A present-but-unusable id is
/// an error, never a silent fresh session.
pub fn qwen_managed_spawn_args(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
    model: Option<&str>,
    _effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Qwen spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Qwen spawn workspace must be an absolute path".into(),
        ));
    }
    let mut args = vec![
        OsString::from("--output-format"),
        OsString::from("stream-json"),
        OsString::from("--chat-recording"),
        OsString::from("-y"),
    ];
    if let Some(session_id) = session_id {
        let session_id = session_id.trim();
        if !session_id.is_empty() {
            let Some(disk_id) = qwen_disk_session_id(session_id) else {
                return Err(HubError::Invalid(format!(
                    "Qwen session id {session_id:?} is not a UUID and cannot be passed to `qwen --session-id`"
                )));
            };
            args.push(OsString::from("--session-id"));
            args.push(OsString::from(disk_id));
        }
    }
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    args.push(OsString::from(prompt));
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WS: &str = "/tmp/coding-assistants-c14-qwen";

    #[test]
    fn qwen_spawn_args_is_explicit_and_rejects_relative_workspace() {
        let args = qwen_spawn_args(Path::new(WS), "review", None, None).unwrap();
        assert_eq!(args[0], "--output-format");
        assert_eq!(args[1], "stream-json");
        assert_eq!(args[2], "--chat-recording");
        assert_eq!(args[3], "-y");
        assert_eq!(args[4], "review");
        assert!(!args.iter().any(|arg| arg == "--session-id"));
        assert!(!args.iter().any(|arg| arg == "-p"));

        let custom = qwen_spawn_args(Path::new(WS), "review", Some("qwen3-coder"), None).unwrap();
        assert_eq!(custom[4], "--model");
        assert_eq!(custom[5], "qwen3-coder");
        assert_eq!(custom[6], "review");

        assert!(qwen_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(qwen_spawn_args(Path::new(WS), "  ", None, None).is_err());
    }

    #[test]
    fn qwen_managed_spawn_args_pins_a_uuid_session_id() {
        let uuid = "123e4567-e89b-42d3-a456-426614174000";
        let args =
            qwen_managed_spawn_args(Path::new(WS), "continue", Some(uuid), None, None).unwrap();
        let at = args.iter().position(|arg| arg == "--session-id").unwrap();
        assert_eq!(args[at + 1], uuid);
        assert!(args.iter().any(|arg| arg == "--chat-recording"));
        assert!(args.iter().any(|arg| arg == "-y"));

        let managed = format!("managed-{uuid}");
        let args =
            qwen_managed_spawn_args(Path::new(WS), "continue", Some(&managed), None, None).unwrap();
        let at = args.iter().position(|arg| arg == "--session-id").unwrap();
        assert_eq!(args[at + 1], uuid);

        assert!(
            qwen_managed_spawn_args(Path::new(WS), "continue", Some("chat-1"), None, None).is_err()
        );
        let fresh =
            qwen_managed_spawn_args(Path::new(WS), "continue", Some("  "), None, None).unwrap();
        assert!(!fresh.iter().any(|arg| arg == "--session-id"));
    }

    #[test]
    fn qwen_disk_session_id_maps_hub_ids_to_cli_uuids() {
        let uuid = "123e4567-e89b-42d3-a456-426614174000";
        assert_eq!(qwen_disk_session_id(uuid).as_deref(), Some(uuid));
        assert_eq!(
            qwen_disk_session_id(&format!("managed-{uuid}")).as_deref(),
            Some(uuid)
        );
        assert_eq!(qwen_disk_session_id("chat-1"), None);
        assert_eq!(qwen_disk_session_id("managed-not-a-uuid"), None);
        assert_eq!(qwen_disk_session_id("  "), None);
    }
}
