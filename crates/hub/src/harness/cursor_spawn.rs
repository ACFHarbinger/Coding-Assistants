//! Cursor `agent` CLI spawn argv (#275). Split from `spawn.rs` for the 500-LoC cap.

use crate::HubError;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// Resolve `agent` vs `cursor-agent` once per process (#275 spike: both exist on
/// some installs; `command -v` picks the first available).
pub fn cursor_executable() -> &'static str {
    static RESOLVED: OnceLock<String> = OnceLock::new();
    RESOLVED.get_or_init(|| {
        for candidate in ["agent", "cursor-agent"] {
            if Command::new(candidate)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
            {
                return candidate.to_string();
            }
        }
        "agent".into()
    })
}

fn validate_workspace_prompt(workspace: &Path, prompt: &str) -> Result<(), HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Cursor spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Cursor spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(())
}

/// Hub-managed placeholders (`managed-<uuid>`) are not Cursor chat ids — never
/// pass them to `--resume` or the CLI rejects the run before stream-json starts.
fn cursor_resume_session_id(session_id: Option<&str>) -> Option<&str> {
    session_id.filter(|id| {
        let id = id.trim();
        !id.is_empty() && !id.starts_with("managed-") && id != "pending"
    })
}

/// Explicit argv for a Cursor `agent` wake/task spawn.
pub fn cursor_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    cursor_managed_spawn_args(workspace, prompt, None, model, effort)
}

/// Explicit argv for an app-managed Cursor worker run. Working directory is the
/// child process cwd (no verified `--workspace` flag in the spike).
pub fn cursor_managed_spawn_args(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    validate_workspace_prompt(workspace, prompt)?;
    let _ = effort;
    let mut args = Vec::new();
    if let Some(id) = cursor_resume_session_id(session_id) {
        args.push(OsString::from("--resume"));
        args.push(OsString::from(id));
    }
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    args.push(OsString::from("-p"));
    args.push(OsString::from(prompt));
    args.push(OsString::from("--output-format"));
    args.push(OsString::from("stream-json"));
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn cursor_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c14-cursor");
        let args = cursor_spawn_args(&ws, "review the hub", None, None).unwrap();
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "review the hub");
        assert_eq!(args[2], "--output-format");
        assert_eq!(args[3], "stream-json");

        let custom = cursor_spawn_args(&ws, "review", Some("composer-2.5"), None).unwrap();
        assert_eq!(custom[0], "--model");
        assert_eq!(custom[1], "composer-2.5");
        assert_eq!(custom[2], "-p");
        assert_eq!(custom[3], "review");

        assert!(cursor_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(cursor_spawn_args(&ws, "   ", None, None).is_err());
    }

    #[test]
    fn cursor_managed_argv_uses_resume_for_real_session_ids_only() {
        let ws = PathBuf::from("/tmp/coding-assistants-c14-cursor");
        let fresh =
            cursor_managed_spawn_args(&ws, "continue", Some("managed-abc"), None, None).unwrap();
        assert_eq!(fresh[0], "-p");
        assert!(!fresh.iter().any(|arg| arg == "--resume"));
        let pending =
            cursor_managed_spawn_args(&ws, "continue", Some("pending"), None, None).unwrap();
        assert!(!pending.iter().any(|arg| arg == "--resume"));

        let resumed = cursor_managed_spawn_args(
            &ws,
            "continue",
            Some("ec48d856-c722-44b3-aa9a-99481f5334a4"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(resumed[0], "--resume");
        assert_eq!(resumed[1], "ec48d856-c722-44b3-aa9a-99481f5334a4");
        assert_eq!(resumed[2], "-p");
        assert_eq!(resumed[3], "continue");
    }

    #[test]
    fn cursor_spawn_never_passes_trust_or_yolo() {
        let ws = PathBuf::from("/tmp/coding-assistants-c14-cursor");
        let args = cursor_spawn_args(&ws, "do the thing", None, None).unwrap();
        assert!(!args.iter().any(|arg| arg == "--trust" || arg == "--yolo"));
    }
}
