//! Explicit per-harness spawn argv plus the one-shot start path
//! (the spawn half of the old harness/mod.rs, split for the
//! 500-LoC cap, #158). Task/wake injection lives in [inject].

use crate::HubError;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

use super::{HarnessId, HarnessStartRequest, HarnessStartResult};

/// Explicit argv for a Grok wake/task spawn. Never concatenated into a shell.
pub fn grok_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Grok spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Grok spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![
        OsString::from("--cwd"),
        workspace.as_os_str().to_os_string(),
        OsString::from(prompt),
    ])
}

/// Explicit argv for an OpenAI Codex / Chat wake/task spawn.
pub fn codex_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Codex spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Codex spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![
        OsString::from("exec"),
        OsString::from("--cwd"),
        workspace.as_os_str().to_os_string(),
        OsString::from(prompt),
    ])
}

/// Explicit argv for an Anthropic Claude Code wake/task spawn.
pub fn claude_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Claude spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Claude spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![OsString::from("-p"), OsString::from(prompt)])
}

/// Explicit argv for a Google Antigravity CLI (agy) wake/task spawn.
///
/// `agy` uses the process working directory as its workspace; unlike Codex it
/// does not support a `--cwd` option.  Its documented non-interactive
/// contract is `--print` with an optional machine-readable output format.
/// A future managed-session adapter may add `--conversation <id>` when it
/// owns that session, but a wake must never guess an existing conversation.
pub fn gemini_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    gemini_managed_spawn_args(workspace, prompt, None)
}

/// Explicit argv for an app-managed Antigravity (`agy`) worker run.
/// Adds `--conversation <id>` when continuing a managed session owned by the app.
pub fn gemini_managed_spawn_args(
    workspace: &Path,
    prompt: &str,
    conversation_id: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Gemini spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Gemini spawn workspace must be an absolute path".into(),
        ));
    }
    // Order matters here in a way `agy --help` does not document: verified
    // directly against a live `agy` invocation (2026-08-14) that
    // `--print --output-format stream-json <prompt>` (the previous order)
    // makes agy misparse the prompt and reply with an off-topic explanation
    // of the `--output-format` flag instead of answering it — the exact
    // symptom #155 originally reported. `--output-format stream-json
    // [--conversation <id>] --print <prompt>` (this order) reliably works;
    // confirmed with both a fresh conversation and a `--conversation`-resumed
    // one. Do not reorder without re-verifying against a real `agy` call.
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
    args.push(OsString::from("--print"));
    args.push(OsString::from(prompt));
    Ok(args)
}

/// Explicit argv for an OpenCode (including DeepSeek) wake spawn.
pub fn opencode_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("OpenCode spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "OpenCode spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![
        OsString::from("run"),
        OsString::from(prompt),
        OsString::from("--dir"),
        workspace.as_os_str().to_os_string(),
    ])
}

/// Explicit argv for a Mistral Vibe wake spawn.
pub fn vibe_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Vibe spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Vibe spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![
        OsString::from("-p"),
        OsString::from(prompt),
        OsString::from("--workdir"),
        workspace.as_os_str().to_os_string(),
        OsString::from("--trust"),
        OsString::from("--output"),
        OsString::from("text"),
        OsString::from("--auto-approve"),
    ])
}

pub(super) fn spawn_explicit(
    program: &str,
    workspace: &Path,
    args: &[OsString],
) -> Result<HarnessStartResult, HubError> {
    match Command::new(program)
        .current_dir(workspace)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => Ok(HarnessStartResult {
            harness: program.into(),
            pid: Some(child.id()),
            status: "started".into(),
            detail: format!("spawned {program} pid {}", child.id()),
        }),
        Err(error) => Ok(HarnessStartResult {
            harness: program.into(),
            pid: None,
            status: "unavailable".into(),
            detail: format!("{program} unavailable: {error}"),
        }),
    }
}

pub fn start_harness(request: &HarnessStartRequest) -> Result<HarnessStartResult, HubError> {
    let harness = HarnessId::parse(&request.harness)?;
    let args = match harness {
        HarnessId::Grok => grok_spawn_args(&request.workspace, &request.prompt)?,
        HarnessId::Chat => codex_spawn_args(&request.workspace, &request.prompt)?,
        HarnessId::Claude => claude_spawn_args(&request.workspace, &request.prompt)?,
        HarnessId::Gemini => gemini_spawn_args(&request.workspace, &request.prompt)?,
        HarnessId::OpenCode => opencode_spawn_args(&request.workspace, &request.prompt)?,
        HarnessId::Vibe => vibe_spawn_args(&request.workspace, &request.prompt)?,
    };
    spawn_explicit(harness.executable(), &request.workspace, &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn grok_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = grok_spawn_args(&ws, "review the hub").unwrap();
        assert_eq!(args[0], "--cwd");
        assert_eq!(args[1], ws.as_os_str());
        assert_eq!(args[2], "review the hub");
        assert!(grok_spawn_args(Path::new("relative"), "x").is_err());
        assert!(grok_spawn_args(&ws, "   ").is_err());
    }

    #[test]
    fn codex_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = codex_spawn_args(&ws, "run task").unwrap();
        assert_eq!(args[0], "exec");
        assert_eq!(args[1], "--cwd");
        assert_eq!(args[2], ws.as_os_str());
        assert_eq!(args[3], "run task");
        assert!(codex_spawn_args(Path::new("relative"), "x").is_err());
        assert!(codex_spawn_args(&ws, "   ").is_err());
    }

    #[test]
    fn claude_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = claude_spawn_args(&ws, "fix bug").unwrap();
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "fix bug");
        assert!(claude_spawn_args(Path::new("relative"), "x").is_err());
        assert!(claude_spawn_args(&ws, "   ").is_err());
    }

    #[test]
    fn gemini_argv_is_explicit_and_rejects_relative_workspace() {
        // Order verified against a live `agy` call (2026-08-14, #155):
        // --output-format before --print, prompt immediately after --print.
        // Putting --print first makes agy misparse the prompt entirely.
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = gemini_spawn_args(&ws, "build feature").unwrap();
        assert_eq!(args[0], "--output-format");
        assert_eq!(args[1], "stream-json");
        assert_eq!(args[2], "--print");
        assert_eq!(args[3], "build feature");
        assert_eq!(args.len(), 4);
        assert!(gemini_spawn_args(Path::new("relative"), "x").is_err());
        assert!(gemini_spawn_args(&ws, "   ").is_err());
    }

    #[test]
    fn gemini_managed_argv_places_conversation_before_print() {
        // Also verified live: --conversation must sit between --output-format
        // and --print, not after --print — same ordering sensitivity as above.
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = gemini_managed_spawn_args(&ws, "continue", Some("conv-123")).unwrap();
        assert_eq!(args[0], "--output-format");
        assert_eq!(args[1], "stream-json");
        assert_eq!(args[2], "--conversation");
        assert_eq!(args[3], "conv-123");
        assert_eq!(args[4], "--print");
        assert_eq!(args[5], "continue");
        assert_eq!(args.len(), 6);
    }

    #[test]
    fn harness_ids_cover_the_four_v1_identities() {
        assert_eq!(HarnessId::parse("grok").unwrap(), HarnessId::Grok);
        assert_eq!(HarnessId::parse("codex").unwrap(), HarnessId::Chat);
        assert_eq!(HarnessId::parse("claude").unwrap(), HarnessId::Claude);
        assert_eq!(HarnessId::parse("agy").unwrap(), HarnessId::Gemini);
        assert_eq!(HarnessId::parse("deepseek").unwrap(), HarnessId::OpenCode);
        assert_eq!(HarnessId::parse("mistral").unwrap(), HarnessId::Vibe);
        assert!(HarnessId::parse("ollama").is_err());
    }

    #[test]
    fn opencode_and_vibe_argv_are_explicit() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let oc = opencode_spawn_args(&ws, "review").unwrap();
        assert_eq!(oc[0], "run");
        assert_eq!(oc[1], "review");
        assert_eq!(oc[2], "--dir");
        assert_eq!(oc[3], ws.as_os_str());
        let vibe = vibe_spawn_args(&ws, "review").unwrap();
        assert_eq!(vibe[0], "-p");
        assert_eq!(vibe[1], "review");
        assert_eq!(HarnessId::OpenCode.executable(), "opencode");
        assert_eq!(HarnessId::Vibe.executable(), "vibe");
        assert!(opencode_spawn_args(Path::new("relative"), "x").is_err());
        assert!(vibe_spawn_args(Path::new("relative"), "x").is_err());
    }

    #[test]
    fn gemini_uses_the_installed_antigravity_executable() {
        assert_eq!(HarnessId::Gemini.executable(), "agy");
    }
}
