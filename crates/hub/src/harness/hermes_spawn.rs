//! Nous Research Hermes Agent CLI spawn argv (C14.16, #322).
//! Split from `spawn.rs` for the 500-LoC cap.
//!
//! Managed runs use headless one-shot mode (`-z <prompt>`, final-text-only
//! stdout) inside the workspace (`--in <dir>`), resume with
//! `--resume <session-id>`, and always pass `--usage-file <path>` so the
//! quota adapter can accumulate the free per-call JSON reports. The
//! auto-approve flags (`--accept-hooks`, `--yolo`) are never passed —
//! strict-sandbox-blocked, same class as Vibe/Qwen (see the
//! `hermes_spawn_never_passes_auto_approve` test).

use crate::HubError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Directory holding the per-call `--usage-file` JSON reports the quota
/// adapter accumulates. Under the Hub data dir (honors `CA_HOME`), never in
/// the workspace or the CLI's own home.
pub fn hermes_usage_dir() -> PathBuf {
    crate::default_hub_home().join("hermes-usage")
}

/// Resolve path to the `hermes` executable: `hermes` on `$PATH` when it
/// answers `--version`, else the bare name so the spawn error names it.
pub fn resolve_hermes_path() -> String {
    if Command::new("hermes")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        return "hermes".into();
    }
    "hermes".into()
}

/// Normalize a Hub session id to a Hermes disk session id (`20260912_...`).
/// Strips the `managed-` prefix the Hub assigns; empty/`"pending"` means a
/// fresh session (None).
pub fn hermes_disk_session_id(session_id: &str) -> Option<String> {
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

/// Filesystem-safe usage-file name for a spawn: the session id when known,
/// else a timestamped one-shot name. Hermes ids are already safe
/// (`20260912_232548_916eae`); anything else is filtered to alnum/`_`/`-`.
fn usage_file_name(session_id: Option<&str>) -> String {
    match session_id.and_then(hermes_disk_session_id) {
        Some(id) => {
            let safe: String = id
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if safe.is_empty() {
                format!("oneshot-{}.json", now_millis())
            } else {
                format!("{safe}.json")
            }
        }
        None => format!("oneshot-{}.json", now_millis()),
    }
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Usage-file path for a spawn, ensuring the directory exists (the CLI
/// writes the file and will not create parents).
pub fn hermes_usage_file(session_id: Option<&str>) -> Result<PathBuf, HubError> {
    let dir = hermes_usage_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| HubError::Invalid(format!("cannot create hermes usage dir: {e}")))?;
    Ok(dir.join(usage_file_name(session_id)))
}

fn validate_workspace_prompt(workspace: &Path, prompt: &str) -> Result<(), HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Hermes spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Hermes spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(())
}

/// Explicit argv for a Hermes wake/task spawn (session_id = None).
pub fn hermes_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    hermes_managed_spawn_args(workspace, prompt, None, model, effort)
}

/// Explicit argv for an app-managed Hermes worker run.
///
/// `-z <prompt>` (final-text-only stdout), `--in <workspace>`,
/// `--resume <id>` when continuing, `-m <model>` when set, and always
/// `--usage-file <hub>/hermes-usage/<name>.json` for the quota adapter.
/// Never `--accept-hooks` / `--yolo`.
pub fn hermes_managed_spawn_args(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
    model: Option<&str>,
    _effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    validate_workspace_prompt(workspace, prompt)?;

    let mut args = Vec::new();
    if let Some(id) = session_id.and_then(hermes_disk_session_id) {
        args.push(OsString::from("--resume"));
        args.push(OsString::from(id));
    }
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("-m"));
        args.push(OsString::from(model));
    }
    args.push(OsString::from("--in"));
    args.push(OsString::from(workspace));
    let usage_file = hermes_usage_file(session_id)?;
    args.push(OsString::from("--usage-file"));
    args.push(OsString::from(usage_file));
    args.push(OsString::from("-z"));
    args.push(OsString::from(prompt));
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    /// Redirect `CA_HOME` so usage-dir creation never touches the real hub
    /// home. Returns the guard + tempdir (both must outlive the test).
    fn hermetic_home() -> (
        std::sync::MutexGuard<'static, ()>,
        tempfile::TempDir,
        Option<std::ffi::OsString>,
    ) {
        let guard = crate::CA_HOME_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let previous = std::env::var_os("CA_HOME");
        std::env::set_var("CA_HOME", dir.path());
        (guard, dir, previous)
    }

    fn restore_home(previous: Option<std::ffi::OsString>) {
        match previous {
            Some(value) => std::env::set_var("CA_HOME", value),
            None => std::env::remove_var("CA_HOME"),
        }
    }

    #[test]
    fn hermes_argv_is_explicit_and_rejects_bad_input() {
        let (_guard, _dir, previous) = hermetic_home();
        let ws = PathBuf::from("/tmp/coding-assistants-c14-hermes");
        let args = hermes_spawn_args(&ws, "test task", None, None).unwrap();
        let text: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(text.contains(&"--in".to_string()));
        assert!(text.contains(&"/tmp/coding-assistants-c14-hermes".to_string()));
        assert!(text.contains(&"-z".to_string()));
        assert!(text.contains(&"test task".to_string()));
        assert!(text.iter().any(|a| a == "--usage-file"));
        assert!(!text.iter().any(|a| a == "--resume"));

        assert!(hermes_spawn_args(Path::new("relative/path"), "test", None, None).is_err());
        assert!(hermes_spawn_args(&ws, "   ", None, None).is_err());
        restore_home(previous);
    }

    #[test]
    fn hermes_managed_argv_resumes_and_sets_model() {
        let (_guard, _dir, previous) = hermetic_home();
        let ws = PathBuf::from("/tmp/coding-assistants-c14-hermes");
        let args = hermes_managed_spawn_args(
            &ws,
            "resume work",
            Some("managed-20260912_232548_916eae"),
            Some("upstage/solar-pro4"),
            None,
        )
        .unwrap();
        let text: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(text[0], "--resume");
        assert_eq!(text[1], "20260912_232548_916eae");
        assert_eq!(text[2], "-m");
        assert_eq!(text[3], "upstage/solar-pro4");

        let pending = hermes_managed_spawn_args(&ws, "hello", Some("pending"), None, None).unwrap();
        assert!(!pending.iter().any(|arg| arg == "--resume"));
        restore_home(previous);
    }

    #[test]
    fn hermes_spawn_never_passes_auto_approve() {
        let (_guard, _dir, previous) = hermetic_home();
        let ws = PathBuf::from("/tmp/coding-assistants-c14-hermes");
        for session in [None, Some("20260912_232548_916eae")] {
            let args =
                hermes_managed_spawn_args(&ws, "do not automate unsafely", session, None, None)
                    .unwrap();
            assert!(!args.iter().any(|arg| {
                let s = arg.to_string_lossy();
                s == "--accept-hooks" || s == "--yolo"
            }));
        }
        restore_home(previous);
    }

    #[test]
    fn usage_file_lands_under_the_hub_home() {
        let (_guard, dir, previous) = hermetic_home();
        let path = hermes_usage_file(Some("20260912_232548_916eae")).unwrap();
        assert_eq!(
            path,
            dir.path()
                .join("hermes-usage")
                .join("20260912_232548_916eae.json")
        );
        assert!(path.parent().is_some_and(|d| d.is_dir()));
        restore_home(previous);
    }

    #[test]
    fn usage_file_names_are_session_scoped_and_safe() {
        assert_eq!(
            usage_file_name(Some("managed-20260912_232548_916eae")),
            "20260912_232548_916eae.json"
        );
        assert_eq!(usage_file_name(Some("../../evil")), "evil.json");
        assert!(usage_file_name(None).starts_with("oneshot-"));
        assert!(usage_file_name(Some("pending")).starts_with("oneshot-"));
    }
}
