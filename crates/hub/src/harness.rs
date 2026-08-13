//! C12 typed harness boundary: start, inject, and capture.
//!
//! Adapters must pass explicit argv (no shell strings) and must not attach
//! to an already-running interactive TUI. Only an explicit wake may spawn a
//! new process. Task delivery remains in the durable Hub inbox until an
//! active-harness adapter consumes it; it must never silently create a second
//! agent instance.

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
            // Gemini is provided locally by the Antigravity CLI.
            Self::Gemini => "agy",
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

/// Explicit argv for an OpenAI Codex / Chat wake/task spawn.
pub fn codex_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Codex spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Codex spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![
        OsString::from("exec"),
        OsString::from("--cwd"),
        workspace.as_os_str().to_os_string(),
        OsString::from(prompt),
    ])
}

/// Explicit argv for an Anthropic Claude Code wake/task spawn.
pub fn claude_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Claude spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Claude spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![OsString::from("-p"), OsString::from(prompt)])
}

/// Explicit argv for a Google Antigravity CLI (agy) wake/task spawn.
pub fn gemini_spawn_args(workspace: &Path, prompt: &str) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid("Gemini spawn requires a prompt".into()));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Gemini spawn workspace must be an absolute path".into(),
        ));
    }
    Ok(vec![
        OsString::from("--cwd"),
        workspace.as_os_str().to_os_string(),
        OsString::from(prompt),
    ])
}

fn spawn_explicit(
    program: &str,
    workspace: &Path,
    args: &[OsString],
) -> Result<HarnessStartResult, HubError> {
    match Command::new(program)
        .current_dir(workspace)
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
    let args = match harness {
        HarnessId::Grok => grok_spawn_args(&request.workspace, &request.prompt)?,
        HarnessId::Chat => codex_spawn_args(&request.workspace, &request.prompt)?,
        HarnessId::Claude => claude_spawn_args(&request.workspace, &request.prompt)?,
        HarnessId::Gemini => gemini_spawn_args(&request.workspace, &request.prompt)?,
    };
    spawn_explicit(harness.executable(), &request.workspace, &args)
}

pub fn inject_harness(request: &HarnessInjectRequest) -> Result<HarnessInjectResult, HubError> {
    inject_harness_inner(None, request)
}

/// Same as [`inject_harness`], but Grok task delivery uses a registered
/// active session through the leader ACP bridge when a store is provided.
pub fn inject_harness_with_store(
    store: &crate::HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    inject_harness_inner(Some(store), request)
}

fn inject_harness_inner(
    store: Option<&crate::HubStore>,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    let harness = HarnessId::parse(&request.harness)?;
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid("workspace path must be absolute".into()));
    }
    if request.is_task && !request.is_wake {
        if harness == HarnessId::Grok {
            if let Some(store) = store {
                return crate::grok_bridge::deliver_grok_task(store, request);
            }
        }
        if harness == HarnessId::Gemini {
            if let Some(store) = store {
                return crate::gemini_bridge::deliver_gemini_task(store, request);
            }
        }
        if harness == HarnessId::Claude {
            if let Some(store) = store {
                return crate::claude_bridge::deliver_claude_task(store, request);
            }
        }
        return Ok(HarnessInjectResult {
            harness: harness.as_str().into(),
            pid: None,
            status: "queued".into(),
            detail: "task is recorded in the session inbox; it awaits the target's active harness adapter".into(),
        });
    }

    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid("workspace path must be absolute".into()));
    }
    let prompt = if request.is_task && request.is_wake {
        format!("[TASK] [WAKE] {}", request.body)
    } else if request.is_task {
        format!("[TASK] {}", request.body)
    } else if request.is_wake {
        format!("[WAKE] {}", request.body)
    } else {
        request.body.clone()
    };

    let args = match harness {
        HarnessId::Grok => grok_spawn_args(&request.workspace, &prompt)?,
        HarnessId::Chat => codex_spawn_args(&request.workspace, &prompt)?,
        HarnessId::Claude => claude_spawn_args(&request.workspace, &prompt)?,
        HarnessId::Gemini => gemini_spawn_args(&request.workspace, &prompt)?,
    };

    let started = spawn_explicit(harness.executable(), &request.workspace, &args)?;
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
    fn codex_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = codex_spawn_args(&ws, "run task").unwrap();
        assert_eq!(args[0], "exec");
        assert_eq!(args[1], "--cwd");
        assert_eq!(args[2], ws.as_os_str());
        assert_eq!(args[3], "run task");
        assert!(codex_spawn_args(Path::new("relative"), "x").is_err());
        assert!(codex_spawn_args(&ws, "   ").is_err());
    }

    #[test]
    fn claude_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = claude_spawn_args(&ws, "fix bug").unwrap();
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "fix bug");
        assert!(claude_spawn_args(Path::new("relative"), "x").is_err());
        assert!(claude_spawn_args(&ws, "   ").is_err());
    }

    #[test]
    fn gemini_argv_is_explicit_and_rejects_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-c12");
        let args = gemini_spawn_args(&ws, "build feature").unwrap();
        assert_eq!(args[0], "--cwd");
        assert_eq!(args[1], ws.as_os_str());
        assert_eq!(args[2], "build feature");
        assert!(gemini_spawn_args(Path::new("relative"), "x").is_err());
        assert!(gemini_spawn_args(&ws, "   ").is_err());
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
    fn gemini_uses_the_installed_antigravity_executable() {
        assert_eq!(HarnessId::Gemini.executable(), "agy");
    }

    #[test]
    fn task_injection_queues_without_spawning_a_new_harness() {
        let result = inject_harness(&HarnessInjectRequest {
            harness: "grok".into(),
            workspace: PathBuf::from("/tmp/workspace-for-task-queue"),
            session_id: Some("session-1".into()),
            message_id: Some("message-1".into()),
            body: "review this".into(),
            is_task: true,
            is_wake: false,
        })
        .unwrap();
        assert_eq!(result.status, "queued");
        assert_eq!(result.pid, None);
    }
}
