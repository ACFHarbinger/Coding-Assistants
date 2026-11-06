//! Explicit per-harness spawn argv plus the one-shot start path
//! (the spawn half of the old harness/mod.rs, split for the
//! 500-LoC cap, #158). Task/wake injection lives in [inject].

use crate::HubError;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use super::HarnessStartResult;

pub const DEFAULT_OPENCODE_MODEL: &str = "opencode-go/glm-5.3";
pub const DEFAULT_DEEPSEEK_MODEL: &str = "deepseek/deepseek-v4-flash";

fn codex_config_string(key: &str, value: &str) -> Result<OsString, HubError> {
    if value.chars().any(char::is_control) {
        return Err(HubError::Invalid(format!(
            "Codex {key} must not contain control characters"
        )));
    }
    // Codex parses `-c` as TOML. This remains one process argument, but the
    // value still needs TOML escaping so a quote cannot alter the setting.
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(OsString::from(format!("{key}=\"{escaped}\"")))
}

/// Explicit argv for a Grok wake/task spawn. Never concatenated into a shell.
pub fn grok_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Grok spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Grok spawn workspace must be an absolute path".into(),
        ));
    }
    let mut args = vec![
        OsString::from("--cwd"),
        workspace.as_os_str().to_os_string(),
    ];
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    if let Some(effort) = effort.map(str::trim).filter(|e| !e.is_empty()) {
        args.push(OsString::from("--reasoning-effort"));
        args.push(OsString::from(effort));
    }
    args.push(OsString::from(prompt));
    Ok(args)
}

/// Explicit argv for an OpenAI Codex / Chat wake/task spawn.
///
/// `codex exec` has no `--cwd` flag — that name is Grok's convention, not
/// Codex's. Codex's actual flag is `-C`/`--cd <DIR>` ("use the specified
/// directory as its working root"), confirmed live against the installed
/// `codex` CLI (v0.147.0): `--cwd` fails immediately with "unexpected
/// argument '--cwd' found", which made every Codex spawn exit right after
/// argv parsing — fast enough that callers only ever saw a zombie process,
/// never a visible error.
pub fn codex_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Codex spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Codex spawn workspace must be an absolute path".into(),
        ));
    }
    let mut args = vec![
        OsString::from("exec"),
        OsString::from("--cd"),
        workspace.as_os_str().to_os_string(),
    ];
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    if let Some(effort) = effort.map(str::trim).filter(|e| !e.is_empty()) {
        args.push(OsString::from("-c"));
        args.push(codex_config_string("model_reasoning_effort", effort)?);
    }
    args.push(OsString::from(prompt));
    Ok(args)
}

/// Explicit argv for an Anthropic Claude Code wake/task spawn.
pub fn claude_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Claude spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Claude spawn workspace must be an absolute path".into(),
        ));
    }
    let mut args = Vec::new();
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    if let Some(effort) = effort.map(str::trim).filter(|e| !e.is_empty()) {
        args.push(OsString::from("--effort"));
        args.push(OsString::from(effort));
    }
    args.push(OsString::from("-p"));
    args.push(OsString::from(prompt));
    Ok(args)
}

/// Explicit argv for a Google Antigravity CLI (agy) wake/task spawn.
///
/// `agy` uses the process working directory as its workspace; unlike Codex it
/// does not support a `--cwd` option.  Its documented non-interactive
/// contract is `--print` with an optional machine-readable output format.
/// A future managed-session adapter may add `--conversation <id>` when it
/// owns that session, but a wake must never guess an existing conversation.
pub fn gemini_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    gemini_managed_spawn_args(workspace, prompt, None, model, effort)
}

/// Explicit argv for an app-managed Antigravity (`agy`) worker run.
/// Adds `--conversation <id>` when continuing a managed session owned by the app.
pub fn gemini_managed_spawn_args(
    workspace: &Path,
    prompt: &str,
    conversation_id: Option<&str>,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Gemini spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Gemini spawn workspace must be an absolute path".into(),
        ));
    }
    let mut args = vec![
        OsString::from("--output-format"),
        OsString::from("stream-json"),
    ];
    if let Some(conv_id) = conversation_id {
        let conv_id = conv_id.trim();
        if !conv_id.is_empty() {
            args.push(OsString::from("--conversation"));
            args.push(OsString::from(conv_id));
        }
    }
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    if let Some(effort) = effort.map(str::trim).filter(|e| !e.is_empty()) {
        args.push(OsString::from("--effort"));
        args.push(OsString::from(effort));
    }
    args.push(OsString::from("--print"));
    args.push(OsString::from(prompt));
    Ok(args)
}

/// Explicit argv for an OpenCode (including DeepSeek) wake spawn.
pub fn opencode_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("OpenCode spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "OpenCode spawn workspace must be an absolute path".into(),
        ));
    }
    let model = model.unwrap_or(DEFAULT_OPENCODE_MODEL);
    let mut args = vec![
        OsString::from("run"),
        OsString::from(prompt),
        OsString::from("--model"),
        OsString::from(model),
    ];
    if let Some(effort) = effort.map(str::trim).filter(|e| !e.is_empty()) {
        args.push(OsString::from("--variant"));
        args.push(OsString::from(effort));
    }
    args.push(OsString::from("--dir"));
    args.push(workspace.as_os_str().to_os_string());
    Ok(args)
}

/// The on-disk Muse session id for a Hub `(harness, workspace)` session id.
///
/// `muse exec --session-id` requires a UUID (`managed-…` is rejected with
/// "invalid --session-id", verified live against muse 1.0.3), while the Hub
/// registers managed workers as `managed-<uuid>` (see
/// `bridge::relaunch::managed`). This maps a Hub session id to the UUID the
/// CLI accepts: a bare UUID passes through, and `managed-<uuid>` strips to
/// its trailing UUID. Anything else returns `None` so the caller fails
/// truthfully instead of spawning a fresh session the Hub believes is a
/// resume.
pub fn muse_disk_session_id(session_id: &str) -> Option<String> {
    use std::str::FromStr;
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

/// Explicit argv for a Meta Muse Code wake/task spawn, verified live
/// against the installed `muse` CLI (#273 spike):
/// `muse exec [--model <id>] [--reasoning-effort <effort>]
/// [--session-id <uuid>] [--workspace <abs>] <prompt>`.
///
/// Managed runs pass their session id through [`muse_managed_spawn_args`]
/// so the event-log capture adapter can find the transcript afterwards;
/// one-shot wakes omit it and get a fresh CLI session. No approval-bypass
/// flags (`--yolo`, `--disable-approval`, …) are passed by default.
pub fn muse_spawn_args(
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    muse_managed_spawn_args(workspace, prompt, None, model, effort)
}

/// Explicit argv for an app-managed Muse Code worker run. Adds
/// `--session-id <uuid>` (normalized via [`muse_disk_session_id`]) when
/// continuing a Hub-owned session; a present-but-unusable id is an error,
/// never a silent fresh session.
pub fn muse_managed_spawn_args(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Muse spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Muse spawn workspace must be an absolute path".into(),
        ));
    }
    let mut args = vec![OsString::from("exec")];
    if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    if let Some(effort) = effort.map(str::trim).filter(|e| !e.is_empty()) {
        args.push(OsString::from("--reasoning-effort"));
        args.push(OsString::from(effort));
    }
    if let Some(session_id) = session_id {
        let session_id = session_id.trim();
        if !session_id.is_empty() {
            let Some(disk_id) = muse_disk_session_id(session_id) else {
                return Err(HubError::Invalid(format!(
                    "Muse session id {session_id:?} is not a UUID and cannot be passed to `muse exec --session-id`"
                )));
            };
            args.push(OsString::from("--session-id"));
            args.push(OsString::from(disk_id));
        }
    }
    args.push(OsString::from("--workspace"));
    args.push(workspace.as_os_str().to_os_string());
    args.push(OsString::from(prompt));
    Ok(args)
}

pub(super) fn spawn_explicit_owned(
    program: &str,
    workspace: &Path,
    args: &[OsString],
) -> Result<Child, std::io::Error> {
    Command::new(program)
        .current_dir(workspace)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
}

pub(super) fn spawn_explicit(
    program: &str,
    workspace: &Path,
    args: &[OsString],
) -> Result<HarnessStartResult, HubError> {
    match spawn_explicit_owned(program, workspace, args) {
        Ok(child) => {
            let pid = child.id();
            std::thread::spawn(move || {
                let _ = child.wait_with_output();
            });
            Ok(HarnessStartResult {
                harness: program.into(),
                pid: Some(pid),
                status: "started".into(),
                detail: format!("spawned {program} pid {pid}"),
            })
        }
        Err(error) => Ok(HarnessStartResult {
            harness: program.into(),
            pid: None,
            status: "unavailable".into(),
            detail: format!("{program} unavailable: {error}"),
        }),
    }
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
