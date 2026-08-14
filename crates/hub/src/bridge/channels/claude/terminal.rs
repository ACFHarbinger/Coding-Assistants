//! Detecting and launching a live Channel-connected Claude Code session:
//! process-table inspection to tell whether the bridge is already running
//! for a workspace, and opening a real terminal when it isn't.

use std::path::Path;

const CHANNEL_BRIDGE_EXECUTABLE: &str = "coding-assistants-claude-channel";

/// Pure match on one `ps -eo pid=,args=` command line: is this a
/// `coding-assistants-claude-channel` bridge process for exactly this
/// workspace? Only the executable basename is checked (never an argument
/// or parent path), same discipline as `src-tauri`'s agent-process
/// detector, plus an exact `--workspace <path>` match so a workspace whose
/// path happens to be a substring of another's can't false-positive.
fn command_is_channel_bridge_for(command: &str, workspace: &Path) -> bool {
    let mut parts = command.split_whitespace();
    let Some(executable) = parts.next() else {
        return false;
    };
    let is_bridge = Path::new(executable)
        .file_name()
        .map(|name| name.to_string_lossy().trim_end_matches(".exe") == CHANNEL_BRIDGE_EXECUTABLE)
        .unwrap_or(false);
    if !is_bridge {
        return false;
    }
    let workspace = workspace.to_string_lossy();
    let shifted = parts.clone().skip(1);
    parts
        .zip(shifted)
        .any(|(flag, value)| flag == "--workspace" && value == workspace)
}

/// Whether a live Claude Code session already has the Channel bridge
/// loaded for `workspace` — i.e. some running `claude` process spawned
/// `coding-assistants-claude-channel --workspace <workspace>` as its MCP
/// server. Process-table inspection only (same shape as
/// `src-tauri::core::process_detector`): never touches the process's
/// stdin/stdout, and this crate has no way to attach to it even if it
/// wanted to — the bridge itself is what proves a session is connected.
pub fn is_channel_session_live(workspace: &Path) -> Result<bool, String> {
    let output = std::process::Command::new("ps")
        .args(["-eo", "pid=,args="])
        .output()
        .map_err(|error| format!("failed to inspect local processes: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).lines().any(|line| {
        let command = line
            .trim_start()
            .split_once(char::is_whitespace)
            .map(|(_, rest)| rest.trim())
            .unwrap_or("");
        command_is_channel_bridge_for(command, workspace)
    }))
}

/// Terminal emulators to try, in order — the system default alternative
/// first, then the common desktop-specific ones. Claude Code's Channel
/// research preview is an interactive TUI (no headless daemon mode like
/// Codex's `app-server` or Gemini's `agy`), so it needs a real TTY; unlike
/// those two harness adapters this can never be a detached background
/// `Command` with piped stdio.
const TERMINAL_CANDIDATES: &[&str] = &["x-terminal-emulator", "konsole", "gnome-terminal", "xterm"];

/// Per-terminal argv prefix before the program to actually run. All four
/// accept "run this program directly, no shell" — `gnome-terminal` via
/// `--`, the rest via `-e` — so `claude` is passed as an argv array with no
/// shell involved (`std::process::Command`'s args are never shell-parsed).
fn terminal_exec_prefix(terminal: &str) -> &'static [&'static str] {
    if terminal == "gnome-terminal" {
        &["--"]
    } else {
        &["-e"]
    }
}

/// Launches a real terminal running `claude` with the Channel bridge
/// loaded for `workspace`, for when [`is_channel_session_live`] found none
/// already connected. Tries each candidate terminal emulator in turn and
/// succeeds on the first one that's installed; the workspace becomes the
/// terminal's (and so `claude`'s) working directory via `current_dir`,
/// never a shell `cd`.
///
/// Returns the spawned terminal-emulator pid (not Claude Code's pid).
/// Some emulators (`gnome-terminal`) hand off to a server and that pid
/// may exit immediately — callers must not treat it as Channel liveness.
pub fn launch_claude_channel_session(workspace: &Path) -> Result<u32, String> {
    let claude_args = [
        "--dangerously-skip-permissions",
        "--dangerously-load-development-channels",
        "server:coding-assistants-channel",
    ];

    let mut errors = Vec::new();
    for terminal in TERMINAL_CANDIDATES {
        let mut command = std::process::Command::new(terminal);
        command
            .current_dir(workspace)
            .args(terminal_exec_prefix(terminal))
            .arg("claude")
            .args(claude_args);
        match command.spawn() {
            Ok(child) => return Ok(child.id()),
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
    fn command_is_channel_bridge_for_matches_the_bridge_and_exact_workspace() {
        let workspace = Path::new("/home/pkhunter/Repositories/Repos/Coding-Assistants");
        assert!(command_is_channel_bridge_for(
            "/home/pkhunter/Repositories/Repos/Coding-Assistants/target/debug/coding-assistants-claude-channel --workspace /home/pkhunter/Repositories/Repos/Coding-Assistants",
            workspace,
        ));
    }

    #[test]
    fn command_is_channel_bridge_for_rejects_other_executables() {
        let workspace = Path::new("/repo");
        assert!(!command_is_channel_bridge_for(
            "/usr/bin/claude --workspace /repo",
            workspace,
        ));
    }

    #[test]
    fn command_is_channel_bridge_for_rejects_a_workspace_that_is_only_a_prefix() {
        // "/repo" must not match a process running for "/repo-other" just
        // because the string happens to start the same way.
        let workspace = Path::new("/repo");
        assert!(!command_is_channel_bridge_for(
            "/x/coding-assistants-claude-channel --workspace /repo-other",
            workspace,
        ));
    }

    #[test]
    fn command_is_channel_bridge_for_rejects_a_different_workspace() {
        let workspace = Path::new("/repo-a");
        assert!(!command_is_channel_bridge_for(
            "/x/coding-assistants-claude-channel --workspace /repo-b",
            workspace,
        ));
    }

    #[test]
    fn terminal_exec_prefix_uses_the_flag_each_emulator_actually_accepts() {
        assert_eq!(terminal_exec_prefix("gnome-terminal"), &["--"]);
        assert_eq!(terminal_exec_prefix("konsole"), &["-e"]);
        assert_eq!(terminal_exec_prefix("xterm"), &["-e"]);
        assert_eq!(terminal_exec_prefix("x-terminal-emulator"), &["-e"]);
    }
}
