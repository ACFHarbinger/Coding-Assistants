//! Low-level process and terminal-emulation helpers for the generic
//! kill -> resume-in-a-real-terminal flow. Split out of `bridge::relaunch`
//! to keep every source unit within the 500-line cap (I8).

use std::process::Command;

/// Terminal emulators to try, in order. Duplicated from
/// `bridge::channels::claude::terminal` rather than shared — that module is
/// Claude-Channel-specific and reserved to Claude's ownership; this one is
/// generic across all four harnesses.
pub(super) const TERMINAL_CANDIDATES: &[&str] =
    &["x-terminal-emulator", "konsole", "gnome-terminal", "xterm"];

pub(super) fn terminal_exec_prefix(terminal: &str) -> &'static [&'static str] {
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
pub(super) fn hold_open_after_exit(program: &str, args: &[String]) -> (&'static str, Vec<String>) {
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
    fn is_pid_running_is_false_for_an_implausible_pid() {
        assert!(!is_pid_running(u32::MAX));
    }

    #[test]
    fn kill_pid_is_a_harmless_noop_for_an_already_dead_pid() {
        assert!(!kill_pid(u32::MAX));
    }
}
