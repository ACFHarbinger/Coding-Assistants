//! Generic kill -> resume-in-a-real-terminal flow for any harness's
//! *interactive* session, backing the Orchestrate "Harness Interfaces"
//! panel. Unlike the headless task/wake `harness::*_spawn_args` (one
//! prompt in, the process exits), this opens a real terminal running the
//! harness's interactive CLI — the same shape as
//! `bridge::channels::claude::terminal::launch_claude_channel_session` —
//! because the human wants to sit in the resumed session, not have the app
//! deliver a single message and exit.

use crate::harness::{HarnessId, HarnessStartRequest, HarnessStartResult};
use crate::{HarnessSessionRegistration, HubStore};
use std::path::Path;
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

/// True if a process with this pid currently exists.
pub fn is_pid_running(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelaunchOutcome {
    pub harness: String,
    pub killed_pid: Option<u32>,
    pub resumed_session_id: Option<String>,
    pub detail: String,
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

    let resumed_session_id = latest_session_id(harness, workspace);
    let args = interactive_resume_args(harness, resumed_session_id.as_deref());
    let program = harness.executable();
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
                let detail = match &resumed_session_id {
                    Some(id) => {
                        format!("Relaunched {program} in a new terminal, resuming session {id}")
                    }
                    None => format!(
                        "Launched a fresh {program} session in a new terminal (no prior session found to resume)"
                    ),
                };
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

/// Starts a headless managed harness worker (`harness::start_harness`) and
/// registers it, first killing any prior managed pid already registered for
/// `(harness, workspace)`.
///
/// Without this, "Start managed" orphaned the previous process: each
/// registration unconditionally overwrites `managed_pid` on the existing
/// Hub row (`register_harness_session`'s documented conflict behavior), so
/// a second click swapped which pid the Hub tracked without anything ever
/// killing the one it stopped tracking. Same kill primitive as
/// `relaunch_harness_in_terminal`, just for the headless one-shot spawn
/// instead of an interactive terminal.
pub fn start_managed_harness(
    store: &HubStore,
    harness_id: &str,
    workspace: &Path,
    disk_session_id: &str,
    prompt: &str,
) -> Result<(HarnessStartResult, HarnessSessionRegistration), String> {
    let harness = HarnessId::parse(harness_id).map_err(|error| error.to_string())?;
    if harness == HarnessId::Claude {
        return Err(
            "Claude has no headless managed worker; Start managed must open a Channel-connected terminal"
                .into(),
        );
    }
    if !workspace.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    let disk_session_id = disk_session_id.trim();
    if disk_session_id.is_empty() {
        return Err(
            "start_managed_harness requires a real disk/thread/conversation id, not a placeholder"
                .into(),
        );
    }
    let workspace_key = workspace.to_string_lossy().to_string();

    if let Ok(Some(existing)) = store.get_harness_session(harness.as_str(), &workspace_key) {
        if let Some(pid) = existing.managed_pid {
            kill_pid(pid);
        }
    }

    let started = crate::harness::start_harness(&HarnessStartRequest {
        harness: harness.as_str().into(),
        workspace: workspace.to_path_buf(),
        session_id: None,
        prompt: prompt.into(),
    })
    .map_err(|error| error.to_string())?;

    let Some(pid) = started.pid else {
        return Err(started.detail);
    };

    let registration = store
        .register_managed_harness_session(harness.as_str(), &workspace_key, disk_session_id, pid)
        .map_err(|error| error.to_string())?;

    Ok((started, registration))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_open_after_exit_wraps_in_sh_with_program_and_args_as_positionals() {
        let (program, args) = hold_open_after_exit("codex", &["resume".into(), "abc".into()]);
        assert_eq!(program, "sh");
        assert_eq!(args[0], "-c");
        assert!(args[1].contains("\"$0\" \"$@\""), "script must exec via positionals, not interpolate");
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
    fn is_pid_running_is_false_for_an_implausible_pid() {
        assert!(!is_pid_running(u32::MAX));
    }

    #[test]
    fn kill_pid_is_a_harmless_noop_for_an_already_dead_pid() {
        assert!(!kill_pid(u32::MAX));
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

    #[test]
    fn start_managed_harness_rejects_a_relative_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err =
            start_managed_harness(&store, "grok", Path::new("relative/path"), "thread-1", "hi")
                .unwrap_err();
        assert!(err.contains("absolute"), "{err}");
    }

    #[test]
    fn start_managed_harness_rejects_an_empty_disk_session_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err =
            start_managed_harness(&store, "grok", Path::new("/abs/repo"), "   ", "hi").unwrap_err();
        assert!(err.contains("real disk"), "{err}");
    }

    #[test]
    fn start_managed_harness_rejects_claude_headless_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err =
            start_managed_harness(&store, "claude", Path::new("/abs/repo"), "session-1", "hi")
                .unwrap_err();
        assert!(err.contains("Channel-connected"), "{err}");
    }

    #[test]
    fn start_managed_harness_rejects_an_unknown_harness() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let err = start_managed_harness(
            &store,
            "not-a-harness",
            Path::new("/abs/repo"),
            "thread-1",
            "hi",
        )
        .unwrap_err();
        assert!(err.contains("unknown harness"), "{err}");
    }

    #[test]
    fn start_managed_harness_kills_a_prior_registered_managed_pid_before_replacing_it() {
        // A real, controllable long-lived process this test owns end-to-end
        // (not a harness CLI, which may not be installed wherever this runs).
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
        let prior_pid = sleeper.id();
        store
            .register_managed_harness_session("grok", "/abs/repo", "old-thread", prior_pid)
            .unwrap();
        assert!(
            is_pid_running(prior_pid),
            "precondition: sleeper must be alive"
        );

        // grok itself may not be installed in every environment this runs
        // in; either outcome (spawn succeeds and re-registers, or grok is
        // unavailable and this returns Err) is fine — the property under
        // test is only that the *prior* pid was killed either way, since
        // the kill happens before the new spawn is attempted.
        let _ = start_managed_harness(&store, "grok", Path::new("/abs/repo"), "new-thread", "hi");

        assert!(
            !is_pid_running(prior_pid),
            "the previously registered managed pid must be killed, not orphaned"
        );
        let _ = sleeper.kill();
        let _ = sleeper.wait();
    }
}
