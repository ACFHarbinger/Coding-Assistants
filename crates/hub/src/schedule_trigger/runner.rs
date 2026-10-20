//! Thin explicit-argv command runner for `systemctl --user`.
//!
//! Commands are never executed through a shell. A bounded timeout keeps a
//! wedged user bus from hanging a Settings action; diagnostics are capped
//! by the lifecycle layer before they reach the UI.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(15);

/// Captured result of one command invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Injectable command boundary so lifecycle tests never touch real systemd.
pub trait CommandRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<CommandOutput, String>;
}

/// Production runner backed by the standard library.
pub struct RealRunner;

impl CommandRunner for RealRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<CommandOutput, String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to spawn `{program}`: {error}"))?;
        let started = Instant::now();
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
                    return Ok(CommandOutput {
                        success: status.success(),
                        stdout,
                        stderr,
                    });
                }
                Ok(None) if started.elapsed() >= SYSTEMCTL_TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "`{program}` did not finish within {}s",
                        SYSTEMCTL_TIMEOUT.as_secs()
                    ));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(error) => return Err(format!("failed to wait for `{program}`: {error}")),
            }
        }
    }
}
