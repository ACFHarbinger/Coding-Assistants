//! Documented Grok leader connect/spawn. Opens a real TUI with `--leader`
//! and/or starts `grok agent leader`. Never writes a PTY or invented socket.
#![allow(dead_code)]

use crate::bridge::grok::{
    active_grok_session_for, default_leader_socket, latest_grok_session_id,
    leader_socket_available, ActiveGrokSession,
};
use crate::{HubError, HubStore};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TERMINAL_CANDIDATES: &[&str] = &["x-terminal-emulator", "konsole", "gnome-terminal", "xterm"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokConnectResult {
    pub leader_socket: String,
    pub leader_live: bool,
    pub started_leader: bool,
    pub started_terminal: bool,
    pub session_id: Option<String>,
    pub live_standalone: Option<ActiveGrokSession>,
    pub detail: String,
}

pub fn grok_leader_daemon_args(socket: &Path) -> Vec<String> {
    vec![
        "agent".into(),
        "leader".into(),
        "--no-exit-on-disconnect".into(),
        "--relay-on-demand".into(),
        "--leader-socket".into(),
        socket.display().to_string(),
    ]
}

pub fn grok_leader_tui_args(workspace: &Path, resume_id: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "--leader".into(),
        "--no-alt-screen".into(),
        "--minimal".into(),
        "--cwd".into(),
        workspace.display().to_string(),
        "--leader-socket".into(),
        default_leader_socket().display().to_string(),
    ];
    if let Some(session_id) = resume_id {
        args.push("--resume".into());
        args.push(session_id.to_string());
    }
    args
}

fn terminal_exec_prefix(terminal: &str) -> &'static [&'static str] {
    if terminal == "gnome-terminal" {
        &["--"]
    } else {
        &["-e"]
    }
}

fn missing_leader_detail(socket: &Path) -> String {
    format!(
        "no leader socket at {} — start Grok with --leader (or [cli] use_leader = true) to enable Hub delivery. Task stays queued.",
        socket.display()
    )
}

pub fn start_grok_leader_daemon(socket: &Path) -> Result<u32, String> {
    let args = grok_leader_daemon_args(socket);
    let child = Command::new("grok")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not start `grok agent leader`: {error}"))?;
    Ok(child.id())
}

pub fn wait_for_leader_socket(socket: &Path, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if leader_socket_available(socket) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(missing_leader_detail(socket))
}

pub fn launch_grok_leader_tui(workspace: &Path, resume_id: Option<&str>) -> Result<(), String> {
    let grok_args = grok_leader_tui_args(workspace, resume_id);
    let mut errors = Vec::new();
    for terminal in TERMINAL_CANDIDATES {
        let mut command = Command::new(terminal);
        command
            .current_dir(workspace)
            .args(terminal_exec_prefix(terminal))
            .arg("grok")
            .args(&grok_args);
        match command.spawn() {
            Ok(_) => return Ok(()),
            Err(error) => errors.push(format!("{terminal}: {error}")),
        }
    }
    Err(format!(
        "could not find a terminal emulator to launch (tried {}): {}",
        TERMINAL_CANDIDATES.join(", "),
        errors.join("; ")
    ))
}

/// Ensure a documented leader is listening, then open (or reuse) a
/// leader-mode TUI for `workspace`. A standalone live TUI cannot be
/// attached; Connect resumes it in a new `--leader` window.
pub fn connect_grok_leader_session(
    store: &HubStore,
    workspace: &Path,
    resume: bool,
) -> Result<GrokConnectResult, HubError> {
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Grok leader connect requires an absolute workspace".into(),
        ));
    }
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let socket = default_leader_socket();
    let live = active_grok_session_for(&workspace);
    let mut started_leader = false;
    let mut started_terminal = false;
    let mut leader_pid: Option<u32> = live.as_ref().map(|row| row.pid);

    if !leader_socket_available(&socket) {
        let pid = start_grok_leader_daemon(&socket).map_err(HubError::Invalid)?;
        wait_for_leader_socket(&socket, Duration::from_secs(8)).map_err(HubError::Invalid)?;
        started_leader = true;
        leader_pid = Some(pid);
    }

    let resume_id = if resume {
        live.as_ref()
            .map(|row| row.session_id.clone())
            .or_else(|| latest_grok_session_id(&workspace))
    } else {
        None
    };

    // A live standalone TUI does not share the new leader. Always open a
    // `--leader` TUI unless a leader was already up (socket existed).
    if started_leader || live.is_none() {
        launch_grok_leader_tui(&workspace, resume_id.as_deref()).map_err(HubError::Invalid)?;
        started_terminal = true;
    }

    let session_id = resume_id.clone().or_else(|| {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(id) = latest_grok_session_id(&workspace) {
                return Some(id);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });

    if let (Some(id), Some(pid)) = (session_id.as_deref(), leader_pid) {
        store.register_managed_harness_session("grok", &workspace.to_string_lossy(), id, pid)?;
    } else if let Some(id) = session_id.as_deref() {
        store.register_harness_session(
            "grok",
            &workspace.to_string_lossy(),
            id,
            Some(&socket.to_string_lossy()),
        )?;
    }

    let detail = if !leader_socket_available(&socket) {
        missing_leader_detail(&socket)
    } else if started_terminal && resume_id.is_some() {
        format!(
            "Leader is up at {}. Opened a --leader TUI resuming {}. Close any standalone Grok window for this workspace so only the leader-mode session is live.",
            socket.display(),
            resume_id.as_deref().unwrap_or("")
        )
    } else if started_terminal {
        format!(
            "Leader is up at {}. Opened a new `grok --leader` TUI in this workspace.",
            socket.display()
        )
    } else {
        format!(
            "Leader socket already live at {}. Hub will inject into session {}.",
            socket.display(),
            session_id.as_deref().unwrap_or("(pending)")
        )
    };

    Ok(GrokConnectResult {
        leader_socket: socket.display().to_string(),
        leader_live: leader_socket_available(&socket),
        started_leader,
        started_terminal,
        session_id,
        live_standalone: live,
        detail,
    })
}

pub fn grok_leader_status(workspace: Option<&Path>) -> GrokConnectResult {
    let socket = default_leader_socket();
    let live = workspace.and_then(active_grok_session_for);
    let session_id = workspace.and_then(latest_grok_session_id);
    let live_now = leader_socket_available(&socket);
    let detail = if live_now {
        format!("leader socket ready at {}", socket.display())
    } else {
        missing_leader_detail(&socket)
    };
    GrokConnectResult {
        leader_socket: socket.display().to_string(),
        leader_live: live_now,
        started_leader: false,
        started_terminal: false,
        session_id,
        live_standalone: live,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_argv_is_documented_leader() {
        let args = grok_leader_daemon_args(Path::new("/tmp/leader.sock"));
        assert_eq!(args[0], "agent");
        assert_eq!(args[1], "leader");
        assert!(args.contains(&"--no-exit-on-disconnect".into()));
        assert!(args.contains(&"--leader-socket".into()));
    }

    #[test]
    fn tui_argv_uses_leader_and_optional_resume() {
        let fresh = grok_leader_tui_args(Path::new("/abs/repo"), None);
        assert_eq!(fresh[0], "--leader");
        assert!(fresh.contains(&"--cwd".into()));
        assert!(fresh.contains(&"--no-alt-screen".into()));
        assert!(fresh.contains(&"--minimal".into()));
        assert!(!fresh.contains(&"--resume".into()));

        let resume = grok_leader_tui_args(Path::new("/abs/repo"), Some("sess-1"));
        assert!(resume.windows(2).any(|pair| pair == ["--resume", "sess-1"]));
    }

    #[test]
    fn terminal_prefix_matches_each_emulator() {
        assert_eq!(terminal_exec_prefix("gnome-terminal"), &["--"]);
        assert_eq!(terminal_exec_prefix("konsole"), &["-e"]);
        assert_eq!(terminal_exec_prefix("xterm"), &["-e"]);
    }

    #[test]
    fn connect_rejects_a_relative_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let error = connect_grok_leader_session(&store, Path::new("relative/repo"), false)
            .expect_err("relative workspace");
        assert!(error.to_string().contains("absolute"));
    }
}
