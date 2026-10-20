//! Official DeepSeek Harness (`dsh`) one-shot completions.
//!
//! The attended surface is the web app (`dsh --profile web`), embedded by
//! `harness::web_app`. This module is the headless task path: one prompt in,
//! the final answer on stdout, then exit. There is no `--model` flag.

use super::llm::ModelConfig;
use super::stream::stream_cli_child;
use hub::bus::InProcessBus;
use std::ffi::OsString;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::process::Command;

/// `dsh --profile headless [--session-id <id>] <task>`.
pub fn deepseek_headless_args(
    prompt: &str,
    session_id: Option<&str>,
) -> Result<Vec<OsString>, String> {
    if prompt.trim().is_empty() {
        return Err("DeepSeek harness requires a prompt".into());
    }
    let mut args = vec![OsString::from("--profile"), OsString::from("headless")];
    if let Some(id) = session_id.map(str::trim).filter(|id| !id.is_empty()) {
        args.push(OsString::from("--session-id"));
        args.push(OsString::from(id));
    }
    args.push(OsString::from(prompt));
    Ok(args)
}

pub fn deepseek_unavailable(error: impl std::fmt::Display) -> String {
    format!(
        "DeepSeek unavailable: the official dsh harness is not installed or failed to start ({error}). Install @deepseek-ai/dsh (https://github.com/deepseek-ai/deepseek-harness) or build submodules/deepseek."
    )
}

pub async fn deepseek_completion(
    _config: &ModelConfig,
    prompt: &str,
    work_dir: Option<&str>,
    bus: &InProcessBus,
    source: &str,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    let args = deepseek_headless_args(prompt, None)?;
    let mut command = Command::new("dsh");
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(secret) = hub::secret::resolve("DEEPSEEK_API_KEY") {
        command.env("DEEPSEEK_API_KEY", secret.expose());
    }
    if let Some(dir) = work_dir {
        command.current_dir(dir);
    }
    let child = command
        .spawn()
        .map_err(|error| deepseek_unavailable(error))?;
    stream_cli_child(child, bus.clone(), source, token, "DeepSeek").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_argv_is_the_official_harness_not_opencode() {
        let args = deepseek_headless_args("review this", Some("session-9")).unwrap();
        assert_eq!(args[0], "--profile");
        assert_eq!(args[1], "headless");
        assert_eq!(args[2], "--session-id");
        assert_eq!(args[3], "session-9");
        assert_eq!(args[4], "review this");
        assert!(!args
            .iter()
            .any(|arg| arg == "opencode" || arg == "run" || arg == "-m"));
        assert!(deepseek_headless_args("  ", None).is_err());
        assert!(deepseek_unavailable("missing").contains("dsh"));
        assert!(!deepseek_unavailable("missing").contains("OpenCode"));
    }
}
