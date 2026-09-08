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

/// Explicit argv for a Mistral Vibe wake spawn.
pub fn vibe_spawn_args(
    workspace: &Path,
    prompt: &str,
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

/// Placeholder spawn argv for the Cursor `agent` CLI. Real argv, stream-json
/// parsing, and chat-id persistence land in #275.
pub fn cursor_spawn_args(
    _workspace: &Path,
    _prompt: &str,
    _model: Option<&str>,
    _effort: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    Err(HubError::Invalid(
        "Cursor agent harness is not implemented yet (#275)".into(),
    ))
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
mod tests {
    use super::*;
    use crate::harness::HarnessId;
    use std::path::PathBuf;

    #[test]
    fn grok_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = grok_spawn_args(&ws, "review the hub", None, None).unwrap();
        assert_eq!(args[0], "--cwd");
        assert_eq!(args[1], ws.as_os_str());
        assert_eq!(args[2], "review the hub");

        let custom = grok_spawn_args(&ws, "review", Some("grok-4.6"), Some("high")).unwrap();
        assert_eq!(custom[0], "--cwd");
        assert_eq!(custom[1], ws.as_os_str());
        assert_eq!(custom[2], "--model");
        assert_eq!(custom[3], "grok-4.6");
        assert_eq!(custom[4], "--reasoning-effort");
        assert_eq!(custom[5], "high");
        assert_eq!(custom[6], "review");

        assert!(grok_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(grok_spawn_args(&ws, "   ", None, None).is_err());
    }

    #[test]
    fn codex_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = codex_spawn_args(&ws, "run task", None, None).unwrap();
        assert_eq!(args[0], "exec");
        assert_eq!(args[1], "--cd");
        assert_eq!(args[2], ws.as_os_str());
        assert_eq!(args[3], "run task");

        let custom = codex_spawn_args(&ws, "run", Some("o3"), Some("high")).unwrap();
        assert_eq!(custom[0], "exec");
        assert_eq!(custom[1], "--cd");
        assert_eq!(custom[2], ws.as_os_str());
        assert_eq!(custom[3], "--model");
        assert_eq!(custom[4], "o3");
        assert_eq!(custom[5], "-c");
        assert_eq!(custom[6], "model_reasoning_effort=\"high\"");
        assert_eq!(custom[7], "run");

        let escaped = codex_spawn_args(&ws, "run", None, Some("high\"; model=\"other")).unwrap();
        assert_eq!(
            escaped[4],
            "model_reasoning_effort=\"high\\\"; model=\\\"other\""
        );
        assert!(codex_spawn_args(&ws, "run", None, Some("high\nother")).is_err());

        assert!(codex_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(codex_spawn_args(&ws, "   ", None, None).is_err());
    }

    #[test]
    fn claude_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = claude_spawn_args(&ws, "fix bug", None, None).unwrap();
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "fix bug");

        let custom = claude_spawn_args(&ws, "fix bug", Some("sonnet"), Some("high")).unwrap();
        assert_eq!(custom[0], "--model");
        assert_eq!(custom[1], "sonnet");
        assert_eq!(custom[2], "--effort");
        assert_eq!(custom[3], "high");
        assert_eq!(custom[4], "-p");
        assert_eq!(custom[5], "fix bug");

        assert!(claude_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(claude_spawn_args(&ws, "   ", None, None).is_err());
    }

    #[test]
    fn gemini_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = gemini_spawn_args(&ws, "build feature", None, None).unwrap();
        assert_eq!(args[0], "--output-format");
        assert_eq!(args[1], "stream-json");
        assert_eq!(args[2], "--print");
        assert_eq!(args[3], "build feature");
        assert_eq!(args.len(), 4);

        let custom = gemini_spawn_args(
            &ws,
            "build feature",
            Some("gemini-3.7-flash-high"),
            Some("high"),
        )
        .unwrap();
        assert_eq!(custom[0], "--output-format");
        assert_eq!(custom[1], "stream-json");
        assert_eq!(custom[2], "--model");
        assert_eq!(custom[3], "gemini-3.7-flash-high");
        assert_eq!(custom[4], "--effort");
        assert_eq!(custom[5], "high");
        assert_eq!(custom[6], "--print");
        assert_eq!(custom[7], "build feature");

        assert!(gemini_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(gemini_spawn_args(&ws, "   ", None, None).is_err());
    }

    #[test]
    fn gemini_managed_argv_places_conversation_before_print() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args =
            gemini_managed_spawn_args(&ws, "continue", Some("conv-123"), None, None).unwrap();
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
        assert_eq!(HarnessId::parse("opencode").unwrap(), HarnessId::OpenCode);
        assert_eq!(HarnessId::parse("deepseek").unwrap(), HarnessId::DeepSeek);
        assert_eq!(HarnessId::parse("mistral").unwrap(), HarnessId::Vibe);
        assert!(HarnessId::parse("ollama").is_err());
    }

    #[test]
    fn muse_and_cursor_ids_parse_but_cursor_spawn_is_a_typed_unavailable() {
        // #271 scaffold: the identities exist; #275 implements the argv.
        // (#273 implements the Muse argv below; Cursor's slice reconciles
        // this test on merge.)
        assert_eq!(HarnessId::parse("muse").unwrap(), HarnessId::Muse);
        assert_eq!(HarnessId::parse("muse-code").unwrap(), HarnessId::Muse);
        assert_eq!(HarnessId::parse("cursor").unwrap(), HarnessId::Cursor);
        assert_eq!(HarnessId::parse("cursor-agent").unwrap(), HarnessId::Cursor);
        assert_eq!(HarnessId::Muse.as_str(), "muse");
        assert_eq!(HarnessId::Cursor.as_str(), "cursor");
        assert_eq!(HarnessId::Muse.executable(), "muse");
        assert_eq!(HarnessId::Cursor.executable(), "agent");

        let ws = PathBuf::from("/tmp/coding-assistants-c14");
        let cursor = cursor_spawn_args(&ws, "do the thing", None, None);
        assert!(cursor.is_err());
        assert!(cursor.unwrap_err().to_string().contains("#275"));
    }

    #[test]
    fn muse_argv_is_explicit_and_rejects_relative_workspace() {
        // #273, verified live against `muse exec --help` (muse 1.0.3).
        let ws = PathBuf::from("/tmp/coding-assistants-c14");
        let args = muse_spawn_args(&ws, "do the thing", None, None).unwrap();
        assert_eq!(args[0], "exec");
        assert_eq!(args[1], "--workspace");
        assert_eq!(args[2], ws.as_os_str());
        assert_eq!(args[3], "do the thing");

        // Model/effort are opaque passthroughs (`test-model` is not a real
        // id — the CLI publishes no model catalog; see #274).
        let custom = muse_spawn_args(&ws, "do the thing", Some("test-model"), Some("low")).unwrap();
        assert_eq!(custom[0], "exec");
        assert_eq!(custom[1], "--model");
        assert_eq!(custom[2], "test-model");
        assert_eq!(custom[3], "--reasoning-effort");
        assert_eq!(custom[4], "low");
        assert_eq!(custom[5], "--workspace");
        assert_eq!(custom[6], ws.as_os_str());
        assert_eq!(custom[7], "do the thing");

        assert!(muse_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(muse_spawn_args(&ws, "   ", None, None).is_err());
    }

    #[test]
    fn muse_managed_argv_pins_a_uuid_session_id() {
        let ws = PathBuf::from("/tmp/coding-assistants-c14");
        let uuid = "123e4567-e89b-42d3-a456-426614174000";
        let args = muse_managed_spawn_args(&ws, "continue", Some(uuid), None, None).unwrap();
        let at = args.iter().position(|arg| arg == "--session-id").unwrap();
        assert_eq!(args[at + 1], uuid);

        // Hub managed ids (`managed-<uuid>`) strip to the trailing UUID —
        // `muse exec` rejects the prefixed form ("invalid --session-id").
        let managed = format!("managed-{uuid}");
        let args = muse_managed_spawn_args(&ws, "continue", Some(&managed), None, None).unwrap();
        let at = args.iter().position(|arg| arg == "--session-id").unwrap();
        assert_eq!(args[at + 1], uuid);

        // A present-but-unusable id is an error, never a silent fresh run.
        assert!(muse_managed_spawn_args(&ws, "continue", Some("chat-1"), None, None).is_err());
        // Blank ids mean a fresh one-shot, like `muse_spawn_args`.
        let fresh = muse_managed_spawn_args(&ws, "continue", Some("  "), None, None).unwrap();
        assert!(!fresh.iter().any(|arg| arg == "--session-id"));
    }

    #[test]
    fn muse_disk_session_id_maps_hub_ids_to_cli_uuids() {
        let uuid = "123e4567-e89b-42d3-a456-426614174000";
        assert_eq!(muse_disk_session_id(uuid).as_deref(), Some(uuid));
        assert_eq!(
            muse_disk_session_id(&format!("managed-{uuid}")).as_deref(),
            Some(uuid)
        );
        assert_eq!(muse_disk_session_id("chat-1"), None);
        assert_eq!(muse_disk_session_id("managed-not-a-uuid"), None);
        assert_eq!(muse_disk_session_id("  "), None);
    }

    #[test]
    fn opencode_and_vibe_argv_are_explicit() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let oc = opencode_spawn_args(&ws, "review", None, None).unwrap();
        assert_eq!(oc[0], "run");
        assert_eq!(oc[1], "review");
        assert_eq!(oc[2], "--model");
        assert_eq!(oc[3], "opencode-go/glm-5.3");
        assert_eq!(oc[4], "--dir");
        assert_eq!(oc[5], ws.as_os_str());

        let ds =
            opencode_spawn_args(&ws, "review", Some(DEFAULT_DEEPSEEK_MODEL), Some("high")).unwrap();
        assert_eq!(ds[0], "run");
        assert_eq!(ds[1], "review");
        assert_eq!(ds[2], "--model");
        assert_eq!(ds[3], "deepseek/deepseek-v4-flash");
        assert_eq!(ds[4], "--variant");
        assert_eq!(ds[5], "high");
        assert_eq!(ds[6], "--dir");
        assert_eq!(ds[7], ws.as_os_str());

        let vibe = vibe_spawn_args(&ws, "review", None, None).unwrap();
        assert_eq!(vibe[0], "-p");
        assert_eq!(vibe[1], "review");
        assert_eq!(HarnessId::OpenCode.executable(), "opencode");
        assert_eq!(HarnessId::DeepSeek.executable(), "opencode");
        assert_eq!(HarnessId::Vibe.executable(), "vibe");
        assert!(opencode_spawn_args(Path::new("relative"), "x", None, None).is_err());
        assert!(vibe_spawn_args(Path::new("relative"), "x", None, None).is_err());
    }

    #[test]
    fn gemini_uses_the_installed_antigravity_executable() {
        assert_eq!(HarnessId::Gemini.executable(), "agy");
    }
}
