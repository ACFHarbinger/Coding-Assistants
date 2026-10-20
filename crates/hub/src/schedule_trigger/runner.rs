//! Thin explicit-argv command runner for `systemctl --user`.
//!
//! Commands are never executed through a shell. Stdout/stderr are drained
//! concurrently while the child runs so a chatty command cannot fill a pipe
//! and stall before exit. Captured text is bounded (extra output is still
//! consumed, then discarded) and a bounded timeout keeps a wedged user bus
//! from hanging a Settings action.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(15);
const CAPTURE_GRACE: Duration = Duration::from_secs(2);
const CAPTURE_BYTES: usize = 16 * 1024;

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

        let stdout_reader = child.stdout.take().map(drain_in_background);
        let stderr_reader = child.stderr.take().map(drain_in_background);
        let started = Instant::now();

        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() >= SYSTEMCTL_TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "`{program}` did not finish within {}s",
                        SYSTEMCTL_TIMEOUT.as_secs()
                    ));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("failed to wait for `{program}`: {error}"));
                }
            }
        };

        let stdout = join_reader(stdout_reader);
        let stderr = join_reader(stderr_reader);
        Ok(CommandOutput {
            success: status.success(),
            stdout,
            stderr,
        })
    }
}

fn drain_in_background<R: Read + Send + 'static>(mut pipe: R) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    if captured.len() < CAPTURE_BYTES {
                        let remaining = CAPTURE_BYTES - captured.len();
                        captured.extend_from_slice(&chunk[..read.min(remaining)]);
                    }
                }
                Err(_) => break,
            }
        }
        let _ = sender.send(String::from_utf8_lossy(&captured).into_owned());
    });
    receiver
}

fn join_reader(receiver: Option<Receiver<String>>) -> String {
    receiver
        .and_then(|receiver| receiver.recv_timeout(CAPTURE_GRACE).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_output_is_drained_without_stalling() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("large.txt");
        std::fs::write(&file, vec![b'x'; 200_000]).unwrap();
        let started = Instant::now();
        let output = RealRunner
            .run("cat", &[file.to_string_lossy().into_owned()])
            .unwrap();
        assert!(output.success);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!output.stdout.is_empty());
        assert!(output.stdout.len() <= CAPTURE_BYTES);
    }

    #[test]
    fn nonzero_exit_keeps_stderr() {
        let output = RealRunner
            .run("ls", &["/definitely-missing-ca-test-path".to_string()])
            .unwrap();
        assert!(!output.success);
        assert!(!output.stderr.is_empty());
    }
}
