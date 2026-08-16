//! Generic kill -> resume-in-a-real-terminal flow for any harness's
//! *interactive* session, backing the Orchestrate "Harness Interfaces"
//! panel. Unlike the headless task/wake `harness::*_spawn_args` (one
//! prompt in, the process exits), this opens a real terminal running the
//! harness's interactive CLI — the same shape as
//! `bridge::channels::claude::terminal::launch_claude_channel_session` —
//! because the human wants to sit in the resumed session, not have the app
//! deliver a single message and exit.

mod managed;
pub use managed::start_managed_harness;

use crate::harness::HarnessId;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Terminal emulators to try, in order. Duplicated from
/// `bridge::channels::claude::terminal` rather than shared — that module is
/// Claude-Channel-specific and reserved to Claude's ownership; this one is
/// generic across all four harnesses.
const TERMINAL_CANDIDATES: &[&str] = &["x-terminal-emulator", "konsole", "gnome-terminal", "xterm"];

fn terminal_exec_prefix(terminal: &str) -> &'static [&'static str] {
    if terminal == "gnome-terminal" {
        &["--"]
    } else {
        &["-e"]
    }
}

/// Wraps `program`/`args` so the launched terminal stays open and shows
/// the exit status after the harness process exits, whatever that status
/// is. Without this, a command that fails immediately — confirmed live,
/// 2026-08-14: `codex resume <a stale/invalid thread id>` prints a real
/// error and exits in well under a second — makes the terminal window
/// flash and close before its error is ever readable, indistinguishable
/// from the terminal itself crashing. Terminal emulators disagree on a
/// "hold open" flag (`konsole --hold`, `xterm -hold`, no equivalent for
/// `gnome-terminal` or whatever `x-terminal-emulator` resolves to), so
/// this wraps the command in a small `sh` script instead, uniform across
/// all four.
///
/// `program`/`args` are passed to `sh` as trailing positional arguments
/// (`$0`/`"$@"`), never interpolated into the script string itself — `sh`
/// treats them as opaque values, not re-parsed shell syntax, so this stays
/// exactly as injection-safe as passing them directly to `Command::args`.
fn hold_open_after_exit(program: &str, args: &[String]) -> (&'static str, Vec<String>) {
    const SCRIPT: &str = r#"
"$0" "$@"
status=$?
echo
echo "[$0 exited $status] Press Enter to close this terminal."
read -r _ignored
exit "$status"
"#;
    let mut sh_args = vec!["-c".to_string(), SCRIPT.to_string(), program.to_string()];
    sh_args.extend(args.iter().cloned());
    ("sh", sh_args)
}

/// True if a process with this pid currently exists and is not a zombie.
pub fn is_pid_running(pid: u32) -> bool {
    if pid == 0 || pid > (i32::MAX as u32) {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string(format!("/proc/{pid}/status")) {
            for line in status.lines() {
                if let Some(state) = line.strip_prefix("State:") {
                    let s = state.trim();
                    return !s.starts_with('Z') && !s.starts_with('X');
                }
            }
            true
        } else {
            false
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
}

/// SIGTERM, then SIGKILL if it's still alive after a short grace period.
/// Returns `false` when the pid was already gone (nothing to kill).
pub fn kill_pid(pid: u32) -> bool {
    if !is_pid_running(pid) {
        return false;
    }
    let _ = Command::new("kill")
        .args(["-15", &pid.to_string()])
        .output();
    std::thread::sleep(std::time::Duration::from_millis(200));
    if is_pid_running(pid) {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    true
}

/// The most recent on-disk session/thread/conversation id this harness has
/// for `workspace`, read from each harness's own C12 capture bridge —
/// never guessed from a killed process's stdout. Only Antigravity's exit
/// message has actually been reverse-engineered in this codebase (see
/// `bridge::channels::gemini::relaunch::parse_agy_resume_conversation_id`);
/// the other three harnesses don't have a verified stdout resume-hint
/// format, so this deliberately reads durable on-disk state instead of
/// guessing one.
pub fn latest_session_id(harness: HarnessId, workspace: &Path) -> Option<String> {
    match harness {
        HarnessId::Claude => {
            let sessions = crate::bridge::claude::list_active_claude_sessions().ok()?;
            crate::bridge::claude::find_active_claude_session(&sessions, workspace)
                .map(|session| session.session_id)
        }
        HarnessId::Grok => crate::bridge::grok::latest_grok_session_id(workspace),
        HarnessId::Chat => crate::bridge::channels::chat::latest_codex_thread_id(workspace),
        HarnessId::Gemini => crate::bridge::gemini::latest_gemini_session_id(workspace),
        HarnessId::OpenCode | HarnessId::Vibe => None,
    }
}

/// Bound on session discovery during a resume launch. Discovery can be
/// slow — `claude agents --json` is a subprocess, the Codex lookup walks
/// the whole `~/.codex/sessions` tree — and an unbounded wait makes the
/// relaunch command hang with no feedback, the #161 "clicked, nothing
/// happened" class. On timeout the caller proceeds with a fresh session
/// and says so truthfully.
const DISCOVERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// Runs `latest_session_id` on a worker thread, returning the id or a
/// timeout flag. On timeout the worker keeps running (a wedged provider
/// subprocess cannot be aborted from here) but the launch no longer
/// depends on it.
fn discover_session_id_bounded(
    harness: HarnessId,
    workspace: &Path,
    timeout: std::time::Duration,
) -> (Option<String>, bool) {
    let workspace = workspace.to_path_buf();
    discover_session_id_bounded_with(harness, workspace, timeout, |h, w| latest_session_id(h, &w))
}

/// Injectable-lookup form of `discover_session_id_bounded` so tests can
/// exercise the timeout path without touching a real provider CLI.
fn discover_session_id_bounded_with(
    harness: HarnessId,
    workspace: PathBuf,
    timeout: std::time::Duration,
    lookup: impl FnOnce(HarnessId, PathBuf) -> Option<String> + Send + 'static,
) -> (Option<String>, bool) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(lookup(harness, workspace));
    });
    match rx.recv_timeout(timeout) {
        Ok(session_id) => (session_id, false),
        Err(_) => (None, true),
    }
}

/// Interactive argv for resuming `harness` at `session_id`, or a fresh
/// session with no resume flag when `session_id` is `None`.
///
/// `--leader`/`--resume` (Grok) and `--conversation` (Gemini/`agy`) are
/// flags already relied on elsewhere in this codebase (`GrokLeaderCard`,
/// `gemini_managed_spawn_args`); `claude --resume` and `codex resume` are
/// each CLI's own documented resume convention but have not been
/// independently re-verified against a live process here the way `agy`'s
/// was.
pub fn interactive_resume_args(harness: HarnessId, session_id: Option<&str>) -> Vec<String> {
    match (harness, session_id) {
        (HarnessId::Grok, Some(id)) => vec!["--leader".into(), "--resume".into(), id.into()],
        (HarnessId::Grok, None) => vec!["--leader".into()],
        (HarnessId::Claude, Some(id)) => vec!["--resume".into(), id.into()],
        (HarnessId::Claude, None) => vec![],
        (HarnessId::Chat, Some(id)) => vec!["resume".into(), id.into()],
        (HarnessId::Chat, None) => vec![],
        (HarnessId::Gemini, Some(id)) => vec!["--conversation".into(), id.into()],
        (HarnessId::Gemini, None) => vec![],
        (HarnessId::OpenCode, _) | (HarnessId::Vibe, _) => vec![],
    }
}

/// Documented `grok` flags for an **in-app PTY**. Fullscreen `--leader`
/// uses the alternate screen (`\x1b[?1049h`) and typically mouse-tracking
/// (`DECSET 1000/1002/1003/1006`). xterm.js then has no local scrollback
/// and forwards wheel events as unused mouse CSI — wheel appears dead.
/// Claude's card is a primary-buffer CLI, so it is unaffected.
///
/// `--no-alt-screen` + `--minimal` keep the primary buffer and print
/// finalized turns into native scrollback (Grok Build TUI help text).
pub fn apply_grok_embedded_scroll_flags(args: &mut Vec<String>) {
    if !args.iter().any(|a| a == "--no-alt-screen") {
        args.push("--no-alt-screen".into());
    }
    if !args.iter().any(|a| a == "--minimal") {
        args.push("--minimal".into());
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelaunchOutcome {
    pub harness: String,
    pub killed_pid: Option<u32>,
    pub resumed_session_id: Option<String>,
    pub detail: String,
}

/// Result of the kill/resolve step shared by both relaunch paths.
pub struct ResolvedRelaunch {
    pub harness: HarnessId,
    pub killed_pid: Option<u32>,
    pub resumed_session_id: Option<String>,
    pub program: &'static str,
    pub args: Vec<String>,
    /// True when session discovery hit `DISCOVERY_TIMEOUT`; callers
    /// should say so instead of pretending a fresh session is a resume.
    pub discovery_timed_out: bool,
}

/// Kill/resolve step shared by both relaunch paths: kills `existing_pid` if
/// given and still alive, then resolves a resume session id from disk and
/// the resulting interactive argv. Neither spawns anything — the external-
/// terminal path (`relaunch_harness_in_terminal`) and the embedded-PTY path
/// (`src-tauri`'s `hub_relaunch_harness_embedded`) each do their own spawn
/// on top of this, since only one of them wants the `hold_open_after_exit`
/// wrapper (a PTY session doesn't flash-close the way a terminal-emulator
/// window does — the frontend keeps it open and shows the exit itself).
pub fn resolve_interactive_relaunch(
    harness_id: &str,
    workspace: &Path,
    existing_pid: Option<u32>,
) -> Result<ResolvedRelaunch, String> {
    let harness = HarnessId::parse(harness_id).map_err(|error| error.to_string())?;
    if !workspace.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }

    let killed_pid = existing_pid.filter(|&pid| kill_pid(pid));
    // Give the process a moment to flush any exit-time state to disk
    // (transcript files) before we go looking for it.
    if killed_pid.is_some() {
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    let (resumed_session_id, discovery_timed_out) =
        discover_session_id_bounded(harness, workspace, DISCOVERY_TIMEOUT);
    let mut args = interactive_resume_args(harness, resumed_session_id.as_deref());
    // #165: a resumed Claude session must reconnect the Channel MCP bridge
    // (same flags `launch_claude_channel_session` relies on) when the
    // workspace is set up for it — otherwise the "resume" opens a plain
    // session with no Hub round-trip, and the live Channel conversation the
    // user is continuing stays disconnected from the app.
    if harness == HarnessId::Claude
        && resumed_session_id.is_some()
        && workspace_has_channel_mcp(workspace)
    {
        args.extend([
            "--dangerously-load-development-channels".into(),
            "server:coding-assistants-channel".into(),
        ]);
    }
    let program = harness.executable();
    Ok(ResolvedRelaunch {
        harness,
        killed_pid,
        resumed_session_id,
        program,
        args,
        discovery_timed_out,
    })
}

const CHANNEL_SERVER_KEY: &str = "coding-assistants-channel";

/// Whether `<workspace>/.mcp.json` already has the Channel server entry —
/// the same condition `launch_claude_channel_session` checks before it
/// opens a terminal. Kept as a small self-contained copy in this generic
/// module rather than reaching into the Claude-reserved channel module.
fn workspace_has_channel_mcp(workspace: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(workspace.join(".mcp.json")) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    value
        .get("mcpServers")
        .and_then(|servers| servers.get(CHANNEL_SERVER_KEY))
        .is_some()
}

/// Kills `existing_pid` if given and still alive, resolves a resume
/// session id from disk, then opens a real terminal running the harness's
/// interactive CLI — resumed if an id was found, fresh otherwise. Never a
/// detached headless `Command`; the human is meant to sit in this session,
/// same as the Claude Channel "Connect" terminal.
pub fn relaunch_harness_in_terminal(
    harness_id: &str,
    workspace: &Path,
    existing_pid: Option<u32>,
) -> Result<RelaunchOutcome, String> {
    let ResolvedRelaunch {
        harness,
        killed_pid,
        resumed_session_id,
        program,
        args,
        discovery_timed_out,
    } = resolve_interactive_relaunch(harness_id, workspace, existing_pid)?;
    let (wrapped_program, wrapped_args) = hold_open_after_exit(program, &args);

    let mut errors = Vec::new();
    for terminal in TERMINAL_CANDIDATES {
        let mut command = Command::new(terminal);
        command
            .current_dir(workspace)
            .args(terminal_exec_prefix(terminal))
            .arg(wrapped_program)
            .args(&wrapped_args);
        match command.spawn() {
            Ok(_) => {
                let mut detail = match &resumed_session_id {
                    Some(id) => {
                        format!("Relaunched {program} in a new terminal, resuming session {id}")
                    }
                    None => format!(
                        "Launched a fresh {program} session in a new terminal (no prior session found to resume)"
                    ),
                };
                if discovery_timed_out {
                    detail.push_str(
                        " Session discovery timed out, so this is a fresh session, not a resume.",
                    );
                }
                return Ok(RelaunchOutcome {
                    harness: harness.as_str().into(),
                    killed_pid,
                    resumed_session_id,
                    detail,
                });
            }
            Err(error) => errors.push(format!("{terminal}: {error}")),
        }
    }
    Err(format!(
        "could not find a terminal emulator to launch (tried {}): {}",
        TERMINAL_CANDIDATES.join(", "),
        errors.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_open_after_exit_wraps_in_sh_with_program_and_args_as_positionals() {
        let (program, args) = hold_open_after_exit("codex", &["resume".into(), "abc".into()]);
        assert_eq!(program, "sh");
        assert_eq!(args[0], "-c");
        assert!(
            args[1].contains("\"$0\" \"$@\""),
            "script must exec via positionals, not interpolate"
        );
        assert_eq!(&args[2..], &["codex", "resume", "abc"]);
    }

    #[test]
    fn hold_open_after_exit_script_actually_propagates_the_exit_status() {
        // Regression: a wrapper that "holds the window open" but silently
        // swallows the real exit code would break anything reading
        // RelaunchOutcome's success/failure the same way a flashing
        // window broke reading the error message.
        let (program, args) = hold_open_after_exit("/bin/false", &[]);
        let status = std::process::Command::new(program)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(1));
    }

    #[test]
    fn interactive_resume_args_match_each_harness_documented_flag() {
        assert_eq!(
            interactive_resume_args(HarnessId::Grok, Some("abc")),
            vec!["--leader", "--resume", "abc"]
        );
        assert_eq!(
            interactive_resume_args(HarnessId::Grok, None),
            vec!["--leader"]
        );
        assert_eq!(
            interactive_resume_args(HarnessId::Claude, Some("abc")),
            vec!["--resume", "abc"]
        );
        assert!(interactive_resume_args(HarnessId::Claude, None).is_empty());
        assert_eq!(
            interactive_resume_args(HarnessId::Chat, Some("abc")),
            vec!["resume", "abc"]
        );
        assert_eq!(
            interactive_resume_args(HarnessId::Gemini, Some("abc")),
            vec!["--conversation", "abc"]
        );
    }

    #[test]
    fn grok_embedded_scroll_flags_are_documented_and_idempotent() {
        let mut args = interactive_resume_args(HarnessId::Grok, Some("abc"));
        apply_grok_embedded_scroll_flags(&mut args);
        apply_grok_embedded_scroll_flags(&mut args);
        assert!(args.windows(2).any(|pair| pair == ["--resume", "abc"]));
        assert_eq!(args.iter().filter(|a| *a == "--no-alt-screen").count(), 1);
        assert_eq!(args.iter().filter(|a| *a == "--minimal").count(), 1);
        let claude = interactive_resume_args(HarnessId::Claude, Some("abc"));
        assert!(!claude.iter().any(|a| a == "--no-alt-screen"));
    }

    #[test]
    fn is_pid_running_is_false_for_an_implausible_pid() {
        assert!(!is_pid_running(u32::MAX));
    }

    #[test]
    fn kill_pid_is_a_harmless_noop_for_an_already_dead_pid() {
        assert!(!kill_pid(u32::MAX));
    }

    #[test]
    fn session_discovery_returns_a_fast_id_without_timing_out() {
        let (id, timed_out) = discover_session_id_bounded_with(
            HarnessId::Grok,
            PathBuf::from("/nonexistent-workspace"),
            std::time::Duration::from_millis(500),
            |_harness, _workspace| Some("session-fast".into()),
        );
        assert_eq!(id.as_deref(), Some("session-fast"));
        assert!(!timed_out);
    }

    #[test]
    fn session_discovery_that_never_returns_times_out_truthfully() {
        // The #161 "clicked, nothing happened" class: a wedged provider
        // discovery (e.g. a hung `claude agents --json`) must not block the
        // launch forever. The lingering worker thread dies with the test.
        let (id, timed_out) = discover_session_id_bounded_with(
            HarnessId::Grok,
            PathBuf::from("/nonexistent-workspace"),
            std::time::Duration::from_millis(150),
            |_harness, _workspace| {
                std::thread::sleep(std::time::Duration::from_secs(30));
                Some("too-late".into())
            },
        );
        assert_eq!(id, None);
        assert!(timed_out);
    }

    #[test]
    fn workspace_has_channel_mcp_detects_the_channel_server_entry() {
        // #165: a resumed Claude session only gets the Channel-reconnect
        // flags when the workspace's own .mcp.json actually has the server.
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        assert!(
            !workspace_has_channel_mcp(&ws),
            "no .mcp.json means no channel configured"
        );
        std::fs::write(
            ws.join(".mcp.json"),
            r#"{"mcpServers":{"coding-assistants-channel":{"command":"x"},"other":{"command":"y"}}}"#,
        )
        .unwrap();
        assert!(workspace_has_channel_mcp(&ws));
        std::fs::write(
            ws.join(".mcp.json"),
            r#"{"mcpServers":{"other":{"command":"y"}}}"#,
        )
        .unwrap();
        assert!(!workspace_has_channel_mcp(&ws));
        std::fs::write(ws.join(".mcp.json"), "not json").unwrap();
        assert!(!workspace_has_channel_mcp(&ws));
    }

    #[test]
    fn relaunch_rejects_a_relative_workspace() {
        let err =
            relaunch_harness_in_terminal("claude", Path::new("relative/path"), None).unwrap_err();
        assert!(err.contains("absolute"), "{err}");
    }

    #[test]
    fn relaunch_rejects_an_unknown_harness() {
        let err = relaunch_harness_in_terminal("not-a-harness", Path::new("/abs/repo"), None)
            .unwrap_err();
        assert!(err.contains("unknown harness"), "{err}");
    }
}
