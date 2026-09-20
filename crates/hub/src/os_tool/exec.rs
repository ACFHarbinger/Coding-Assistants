//! Spawn an OS tool with explicit argv (never a shell) and a timeout.

use super::policy::OsToolRequest;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const PREVIEW_CHARS: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsToolOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub duration_ms: u128,
}

fn preview(text: &str) -> String {
    if text.chars().count() <= PREVIEW_CHARS {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(PREVIEW_CHARS).collect();
        format!("{truncated}…")
    }
}

/// Run `request` without a shell. Callers must have already `Allow`ed it
/// (or received an explicit human approval for a `NeedsApproval` tool).
pub fn run(request: &OsToolRequest, timeout: Duration) -> Result<OsToolOutput, String> {
    let mut command = Command::new(&request.program);
    command
        .args(&request.args)
        .current_dir(&request.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let started = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn `{}`: {error}", request.program))?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_string(&mut stdout);
                }
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                return Ok(OsToolOutput {
                    exit_code: status.code(),
                    stdout: preview(&stdout),
                    stderr: preview(&stderr),
                    timed_out: false,
                    duration_ms: started.elapsed().as_millis(),
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(OsToolOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "timed out".into(),
                    timed_out: true,
                    duration_ms: started.elapsed().as_millis(),
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => return Err(format!("wait failed: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn echo_request(arg: &str) -> OsToolRequest {
        OsToolRequest {
            program: "echo".into(),
            args: vec![arg.to_string()],
            cwd: env::temp_dir().to_string_lossy().into_owned(),
            workspace: None,
        }
    }

    #[test]
    fn echo_keeps_metacharacters_as_data() {
        let dangerous = "; rm -rf / && echo pwned $(whoami)";
        let output = run(&echo_request(dangerous), Duration::from_secs(5)).unwrap();
        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout.contains(dangerous), "{}", output.stdout);
        assert!(!output.timed_out);
    }

    #[test]
    fn timeout_kills_a_sleeping_child() {
        let request = OsToolRequest {
            program: "sleep".into(),
            args: vec!["30".into()],
            cwd: env::temp_dir().to_string_lossy().into_owned(),
            workspace: None,
        };
        let started = Instant::now();
        let output = run(&request, Duration::from_millis(250)).unwrap();
        assert!(output.timed_out);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
