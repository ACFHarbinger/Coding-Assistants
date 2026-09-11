//! C12 typed harness boundary: start, inject, and capture.
//!
//! Adapters must pass explicit argv (no shell strings) and must not attach
//! to an already-running interactive TUI. Only an explicit wake may spawn a
//! new process. Task delivery remains in the durable Hub inbox until an
//! active-harness adapter consumes it; it must never silently create a second
//! agent instance.
//!
//! Split for the 500-LoC cap (#158): per-harness spawn argv + the one-shot
//! start path live in [spawn]; task/wake injection dispatch in [inject].

use crate::HubError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod cursor_spawn;
mod inject;
mod kimi_spawn;
mod spawn;
mod start;
mod vibe_spawn;
pub use cursor_spawn::{cursor_executable, cursor_managed_spawn_args, cursor_spawn_args};
pub use inject::{inject_harness, inject_harness_with_store};
pub use kimi_spawn::{
    kimi_disk_session_id, kimi_executable, kimi_managed_spawn_args, kimi_spawn_args,
};
pub use spawn::{
    claude_spawn_args, codex_spawn_args, gemini_managed_spawn_args, gemini_spawn_args,
    grok_spawn_args, muse_disk_session_id, muse_managed_spawn_args, muse_spawn_args,
    opencode_spawn_args, DEFAULT_DEEPSEEK_MODEL, DEFAULT_OPENCODE_MODEL,
};
pub use start::start_harness;
pub(crate) use start::start_harness_owned;
pub use vibe_spawn::{
    start_vibe_managed_harness, vibe_disk_session_id, vibe_managed_spawn_args, vibe_spawn_args,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessId {
    Grok,
    Chat,
    Claude,
    Gemini,
    OpenCode,
    DeepSeek,
    Vibe,
    /// Meta Muse Code terminal coding-agent. Real spawn/capture/resume is #273.
    Muse,
    /// Cursor local CLI agent (`agent`, some installs `cursor-agent`). Real
    /// spawn/capture/resume is #275.
    Cursor,
    /// Moonshot Kimi Code CLI. Real spawn/capture/resume is #309.
    Kimi,
}

impl HarnessId {
    pub fn parse(s: &str) -> Result<Self, HubError> {
        match s {
            "grok" | "xai" | "supergrok" => Ok(Self::Grok),
            "chat" | "codex" | "openai" => Ok(Self::Chat),
            "claude" | "anthropic" => Ok(Self::Claude),
            "gemini" | "agy" | "google" => Ok(Self::Gemini),
            "opencode" => Ok(Self::OpenCode),
            "deepseek" => Ok(Self::DeepSeek),
            "vibe" | "mistral" => Ok(Self::Vibe),
            // No bare "meta" alias: Meta ships both the Muse Code harness (#273)
            // and the Muse Spark model provider (#274); "meta" alone is ambiguous.
            "muse" | "muse-code" => Ok(Self::Muse),
            "cursor" | "cursor-agent" => Ok(Self::Cursor),
            "kimi" | "kimi-code" | "moonshot" => Ok(Self::Kimi),
            other => Err(HubError::Invalid(format!(
                "unknown harness: {other} (expected grok, chat, claude, gemini, opencode, deepseek, vibe, muse, cursor, or kimi)"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Grok => "grok",
            Self::Chat => "chat",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
            Self::OpenCode => "opencode",
            Self::DeepSeek => "deepseek",
            Self::Vibe => "vibe",
            Self::Muse => "muse",
            Self::Cursor => "cursor",
            Self::Kimi => "kimi",
        }
    }

    pub fn executable(self) -> &'static str {
        match self {
            Self::Grok => "grok",
            Self::Chat => "codex",
            Self::Claude => "claude",
            // Gemini is provided locally by the Antigravity CLI.
            Self::Gemini => "agy",
            Self::OpenCode | Self::DeepSeek => "opencode",
            Self::Vibe => "vibe",
            // TODO(#273): confirm `muse` vs `muse-code` against the installed CLI.
            Self::Muse => "muse",
            // TODO(#275): resolve `agent` vs `cursor-agent` once at setup.
            Self::Cursor => "agent",
            Self::Kimi => "kimi",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarnessStartRequest {
    pub harness: String,
    pub workspace: PathBuf,
    pub session_id: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessStartResult {
    pub harness: String,
    pub pid: Option<u32>,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarnessInjectRequest {
    pub harness: String,
    pub workspace: PathBuf,
    pub session_id: Option<String>,
    pub message_id: Option<String>,
    pub body: String,
    pub is_task: bool,
    pub is_wake: bool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessInjectResult {
    pub harness: String,
    pub pid: Option<u32>,
    pub status: String,
    pub detail: String,
}
