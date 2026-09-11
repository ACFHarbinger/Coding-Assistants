//! Moonshot Kimi Code CLI spawn argv (C14.14, #309).
//! Split from `spawn.rs` for the 500-LoC cap.

use crate::HubError;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

/// Resolve path to the `kimi` executable.
///
/// Priority:
/// 1. `$KIMI_CODE_HOME/bin/kimi` if `$KIMI_CODE_HOME` is set.
/// 2. `kimi` on `$PATH` if it responds to `--version`.
/// 3. `~/.kimi-code/bin/kimi` fallback.
/// 4. Bare `"kimi"`.
pub fn resolve_kimi_path() -> String {
    if let Ok(home) = std::env::var("KIMI_CODE_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            let candidate = Path::new(trimmed).join("bin").join("kimi");
            if candidate.is_file() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    if Command::new("kimi")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        return "kimi".into();
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let candidate = Path::new(&home).join(".kimi-code").join("bin").join("kimi");
    if candidate.is_file() {
        return candidate.to_string_lossy().into_owned();
    }
    "kimi".into()
}

/// Static executable string for `spawn_program` and relaunch callers.
pub fn kimi_executable() -> &'static str {
    Box::leak(resolve_kimi_path().into_boxed_str())
}

/// Normalize session ID for `kimi --session <id>` resume.
///
/// Strips any `managed-` prefix that the Hub assigns to managed sessions.
/// Returns None if the session ID is empty or `"pending"`.
pub fn kimi_disk_session_id(session_id: &str) -> Option<String> {
    let trimmed = session_id.trim();
    if trimmed.is_empty() || trimmed == "pending" {
        return None;
    }
    let id = trimmed.strip_prefix("managed-").unwrap_or(trimmed);
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

fn validate_workspace_prompt(workspace: &Path, prompt: &str) -> Result<(), HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Kimi spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Kimi spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(())
}

/// Explicit argv for a Kimi wake/task spawn (session_id = None).
pub fn kimi_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    kimi_managed_spawn_args(workspace, prompt, None, model, effort)
}

/// Explicit argv for an app-managed Kimi worker run.
///
/// Running non-interactively with `-p <prompt>` and `--output-format stream-json`.
/// When resuming an existing session, passes `--session <id>`.
pub fn kimi_managed_spawn_args(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
    model: Option<&str>,
    _effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    validate_workspace_prompt(workspace, prompt)?;

    let mut args = Vec::new();
    if let Some(raw_id) = session_id {
        if let Some(id) = kimi_disk_session_id(raw_id) {
            args.push(OsString::from("--session"));
            args.push(OsString::from(id));
        }
    }
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("-m"));
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
    use tempfile::tempdir;

    #[test]
    fn kimi_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c14-kimi");
        let args = kimi_spawn_args(&ws, "test task", None, None).unwrap();
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "test task");
        assert_eq!(args[2], "--output-format");
        assert_eq!(args[3], "stream-json");

        assert!(kimi_spawn_args(Path::new("relative/path"), "test", None, None).is_err());
        assert!(kimi_spawn_args(&ws, "   ", None, None).is_err());
    }

    #[test]
    fn kimi_managed_argv_uses_session_flag_when_present() {
        let ws = PathBuf::from("/tmp/coding-assistants-c14-kimi");
        let fresh = kimi_managed_spawn_args(&ws, "hello", None, None, None).unwrap();
        assert!(!fresh.iter().any(|arg| arg == "--session"));

        let pending = kimi_managed_spawn_args(&ws, "hello", Some("pending"), None, None).unwrap();
        assert!(!pending.iter().any(|arg| arg == "--session"));

        let managed = kimi_managed_spawn_args(
            &ws,
            "resume work",
            Some("managed-session_c3856fb0"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(managed[0], "--session");
        assert_eq!(managed[1], "session_c3856fb0");
        assert_eq!(managed[2], "-p");
        assert_eq!(managed[3], "resume work");
        assert_eq!(managed[4], "--output-format");
        assert_eq!(managed[5], "stream-json");
    }

    #[test]
    fn kimi_managed_argv_includes_model_when_specified() {
        let ws = PathBuf::from("/tmp/coding-assistants-c14-kimi");
        let args =
            kimi_managed_spawn_args(&ws, "task with model", None, Some("kimi-code/k3"), None)
                .unwrap();
        assert_eq!(args[0], "-m");
        assert_eq!(args[1], "kimi-code/k3");
        assert_eq!(args[2], "-p");
        assert_eq!(args[3], "task with model");
    }

    #[test]
    fn kimi_spawn_never_passes_auto_or_yolo() {
        let ws = PathBuf::from("/tmp/coding-assistants-c14-kimi");
        let args = kimi_spawn_args(&ws, "do not automate unsafely", None, None).unwrap();
        assert!(!args
            .iter()
            .any(|arg| arg == "--auto" || arg == "--yolo" || arg == "-y"));
    }

    #[test]
    fn resolve_kimi_path_honors_kimi_code_home() {
        let dir = tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let fake_bin = bin_dir.join("kimi");
        std::fs::write(&fake_bin, "#!/bin/sh\nexit 0\n").unwrap();

        std::env::set_var("KIMI_CODE_HOME", dir.path());
        let resolved = resolve_kimi_path();
        std::env::remove_var("KIMI_CODE_HOME");

        assert_eq!(resolved, fake_bin.to_string_lossy());
    }
}
