//! Detecting and launching a live Channel-connected Claude Code session:
//! process-table inspection to tell whether the bridge is already running
//! for a workspace, and opening a real terminal when it isn't.

use std::path::Path;

const CHANNEL_BRIDGE_EXECUTABLE: &str = "coding-assistants-claude-channel";
const CHANNEL_SERVER_KEY: &str = "coding-assistants-channel";

/// Whether `<workspace>/.mcp.json` already has the Channel server entry
/// `--setup` writes. Any read/parse failure (file missing, invalid JSON)
/// is treated as "not set up yet" rather than an error — the setup step
/// itself is safe to (re-)run and will just write it.
fn workspace_mcp_json_has_channel_server(workspace: &Path) -> bool {
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

/// Ensures the one-time `--setup --workspace <path>` step has run for
/// `workspace` before a terminal is opened that assumes it already has.
/// Without it, Claude Code fails immediately with "server:
/// coding-assistants-channel · no MCP server configured with that name" —
/// confirmed live, 2026-08-14: nothing in the desktop app ever called
/// `setup_claude_channel` (only a test did), so "Connect"/"Start managed"
/// launched a terminal that was *guaranteed* to fail for any workspace
/// that had never been set up through the separate `--setup` CLI path.
///
/// Idempotent — skipped once the workspace's own `.mcp.json` already has
/// the `coding-assistants-channel` server entry. Deliberately checks the
/// real target file, not just this app's internal canonical copy under
/// `workspace_servers_path`: confirmed live that the two can disagree —
/// a prior setup attempt left the internal copy behind without the
/// workspace's `.mcp.json` ever actually getting written (an interrupted
/// or since-deleted prior run), which would have made a weaker check
/// wrongly skip re-running setup here too. Shells out to the sibling
/// `coding-assistants-claude-channel --setup` binary (built into the same
/// `target/{debug,release}` directory as this process's own executable in
/// this Cargo workspace) rather than reimplementing its `.mcp.json` merge
/// logic a second time here.
fn ensure_claude_channel_setup(workspace: &Path) -> Result<(), String> {
    if workspace_mcp_json_has_channel_server(workspace) {
        return Ok(());
    }
    let bridge_binary = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(CHANNEL_BRIDGE_EXECUTABLE)))
        .ok_or_else(|| "could not resolve the Claude Channel bridge binary's own directory".to_string())?;
    if !bridge_binary.is_file() {
        return Err(format!(
            "Claude Channel bridge binary not found at {} — build it with `cargo build -p claude`",
            bridge_binary.display()
        ));
    }
    let output = std::process::Command::new(&bridge_binary)
        .args(["--setup", "--workspace"])
        .arg(workspace)
        .output()
        .map_err(|error| format!("failed to run Claude Channel setup: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Claude Channel setup failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

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

/// Pids of `coding-assistants-claude-channel --workspace <workspace>`
/// processes. Same match as [`is_channel_session_live`] — the Channel
/// bridge is the liveness signal, not a terminal-emulator pid.
pub fn channel_bridge_pids(workspace: &Path) -> Result<Vec<u32>, String> {
    let output = std::process::Command::new("ps")
        .args(["-eo", "pid=,args="])
        .output()
        .map_err(|error| format!("failed to inspect local processes: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let (pid_str, rest) = trimmed.split_once(char::is_whitespace)?;
            let pid = pid_str.parse::<u32>().ok()?;
            command_is_channel_bridge_for(rest.trim(), workspace).then_some(pid)
        })
        .collect())
}

/// Whether a live Claude Code session already has the Channel bridge
/// loaded for `workspace` — i.e. some running `claude` process spawned
/// `coding-assistants-claude-channel --workspace <workspace>` as its MCP
/// server. Process-table inspection only (same shape as
/// `src-tauri::core::process_detector`): never touches the process's
/// stdin/stdout, and this crate has no way to attach to it even if it
/// wanted to — the bridge itself is what proves a session is connected.
///
/// The bridge subprocess existing is *not* sufficient on its own: it can
/// outlive its parent Claude Code process as an orphan (confirmed live —
/// a bridge subprocess kept running, reparented, for over an hour after
/// its Claude Code TUI had already exited), which made this return `true`
/// while `claude agents --json` — what actual delivery checks — correctly
/// found nothing, so a "Managed · Ready" badge could sit there
/// indefinitely while every delivery attempt failed as unavailable. Cross-
/// checking both means a bare orphaned subprocess can no longer fake
/// liveness by itself.
pub fn is_channel_session_live(workspace: &Path) -> Result<bool, String> {
    if channel_bridge_pids(workspace)?.is_empty() {
        return Ok(false);
    }
    let sessions = crate::bridge::claude::list_active_claude_sessions()?;
    Ok(crate::bridge::claude::find_active_claude_session(&sessions, workspace).is_some())
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
///
/// Runs [`ensure_claude_channel_setup`] first (a no-op once already set
/// up) so callers never open a terminal that's guaranteed to fail with
/// "no MCP server configured with that name".
pub fn launch_claude_channel_session(workspace: &Path) -> Result<u32, String> {
    ensure_claude_channel_setup(workspace)?;
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
    fn workspace_mcp_json_has_channel_server_is_false_when_the_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!workspace_mcp_json_has_channel_server(dir.path()));
    }

    #[test]
    fn workspace_mcp_json_has_channel_server_is_false_for_unrelated_servers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mcp.json"),
            r#"{"mcpServers":{"filesystem":{"command":"npx"}}}"#,
        )
        .unwrap();
        assert!(!workspace_mcp_json_has_channel_server(dir.path()));
    }

    #[test]
    fn workspace_mcp_json_has_channel_server_is_true_once_setup_has_written_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".mcp.json"),
            r#"{"mcpServers":{"coding-assistants-channel":{"command":"/path/to/bridge"}}}"#,
        )
        .unwrap();
        assert!(workspace_mcp_json_has_channel_server(dir.path()));
    }

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
