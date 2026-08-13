//! C12 typed harness boundary: start, inject, and capture.
//!
//! Adapters must pass explicit argv (no shell strings) and must not attach
//! to an already-running interactive TUI. Wake may spawn a new process.
//! Task inject may also spawn a new process of an already-enrolled identity
//! when no attach API exists.

use crate::HubError;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessId {
    Grok,
    Chat,
    Claude,
    Gemini,
}

impl HarnessId {
    pub fn parse(s: &str) -> Result<Self, HubError> {
        match s {
            "grok" | "xai" | "supergrok" => Ok(Self::Grok),
            "chat" | "codex" | "openai" => Ok(Self::Chat),
            "claude" | "anthropic" => Ok(Self::Claude),
            "gemini" | "agy" | "google" => Ok(Self::Gemini),
            other => Err(HubError::Invalid(format!(
                "unknown harness: {other} (expected grok, chat, claude, or gemini)"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Grok => "grok",
            Self::Chat => "chat",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
        }
    }

    pub fn executable(self) -> &'static str {
        match self {
            Self::Grok => "grok",
            Self::Chat => "codex",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessStartRequest {
    pub harness: String,
    pub workspace: PathBuf,
    pub session_id: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessStartResult {
    pub harness: String,
    pub pid: Option<u32>,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessInjectRequest {
    pub harness: String,
    pub workspace: PathBuf,
    pub session_id: Option<String>,
    pub message_id: Option<String>,
    pub body: String,
    pub is_task: bool,
    pub is_wake: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessInjectResult {
    pub harness: String,
    pub pid: Option<u32>,
    pub status: String,
    pub detail: String,
}

/// Explicit argv for a Grok wake/task spawn. Never concatenated into a shell.
pub fn grok_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Grok spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Grok spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![
        OsString::from("--cwd"),
        workspace.as_os_str().to_os_string(),
        OsString::from(prompt),
    ])
}

fn spawn_explicit(program: &str, args: &[OsString]) -> Result<HarnessStartResult, HubError> {
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => Ok(HarnessStartResult {
            harness: program.into(),
            pid: Some(child.id()),
            status: "started".into(),
            detail: format!("spawned {program} pid {}", child.id()),
        }),
        Err(error) => Ok(HarnessStartResult {
            harness: program.into(),
            pid: None,
            status: "unavailable".into(),
            detail: format!("{program} unavailable: {error}"),
        }),
    }
}

pub fn start_harness(request: &HarnessStartRequest) -> Result<HarnessStartResult, HubError> {
    let harness = HarnessId::parse(&request.harness)?;
    match harness {
        HarnessId::Grok => {
            let args = grok_spawn_args(&request.workspace, &request.prompt)?;
            spawn_explicit(harness.executable(), &args)
        }
        HarnessId::Chat | HarnessId::Claude | HarnessId::Gemini => Ok(HarnessStartResult {
            harness: harness.as_str().into(),
            pid: None,
            status: "unsupported".into(),
            detail: format!(
                "{} adapter is assigned to another agent; see AGENT_BUS C12",
                harness.as_str()
            ),
        }),
    }
}

pub fn inject_harness(request: &HarnessInjectRequest) -> Result<HarnessInjectResult, HubError> {
    let harness = HarnessId::parse(&request.harness)?;
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    match harness {
        HarnessId::Grok => {
            let prompt = if request.is_task && request.is_wake {
                format!("[TASK] [WAKE] {}", request.body)
            } else if request.is_task {
                format!("[TASK] {}", request.body)
            } else if request.is_wake {
                format!("[WAKE] {}", request.body)
            } else {
                request.body.clone()
            };
            let args = grok_spawn_args(&request.workspace, &prompt)?;
            let started = spawn_explicit(harness.executable(), &args)?;
            Ok(HarnessInjectResult {
                harness: started.harness,
                pid: started.pid,
                status: if started.status == "started" {
                    "spawned".into()
                } else {
                    started.status
                },
                detail: started.detail,
            })
        }
        HarnessId::Chat | HarnessId::Claude | HarnessId::Gemini => Ok(HarnessInjectResult {
            harness: harness.as_str().into(),
            pid: None,
            status: "unsupported".into(),
            detail: format!(
                "{} inject is assigned to another agent; see AGENT_BUS C12",
                harness.as_str()
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn grok_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = grok_spawn_args(&ws, "review the hub").unwrap();
        assert_eq!(args[0], "--cwd");
        assert_eq!(args[1], ws.as_os_str());
        assert_eq!(args[2], "review the hub");
        assert!(grok_spawn_args(Path::new("relative"), "x").is_err());
        assert!(grok_spawn_args(&ws, "   ").is_err());
    }

    #[test]
    fn harness_ids_cover_the_four_v1_identities() {
        assert_eq!(HarnessId::parse("grok").unwrap(), HarnessId::Grok);
        assert_eq!(HarnessId::parse("codex").unwrap(), HarnessId::Chat);
        assert_eq!(HarnessId::parse("claude").unwrap(), HarnessId::Claude);
        assert_eq!(HarnessId::parse("agy").unwrap(), HarnessId::Gemini);
        assert!(HarnessId::parse("ollama").is_err());
    }

    #[test]
    fn other_harnesses_are_reserved_not_silently_run() {
        let request = HarnessStartRequest {
            harness: "claude".into(),
            workspace: PathBuf::from("/tmp"),
            session_id: None,
            prompt: "hi".into(),
        };
        let result = start_harness(&request).unwrap();
        assert_eq!(result.status, "unsupported");
        assert!(result.pid.is_none());
    }
}
