//! Per-harness process settings (split out of settings/model.rs for the
//! 500-LoC cap, #274).

use super::model::{FieldStatus, SettingsError};
use serde::{Deserialize, Serialize};

/// Global per-harness process settings. A workspace does not copy these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSettings {
    pub harness: String,
    pub executable: String,
    pub workdir: Option<String>,
    pub capture_polling: bool,
    pub inject_permission: bool,
    pub default_model: Option<String>,
    pub default_effort: Option<String>,
}

impl HarnessSettings {
    pub fn default_for(harness: &str) -> Result<Self, SettingsError> {
        let id = crate::HarnessId::parse(harness)
            .map_err(|err| SettingsError::Invalid(err.to_string()))?;
        let default_model = match id {
            crate::HarnessId::OpenCode => Some(crate::harness::DEFAULT_OPENCODE_MODEL.to_string()),
            crate::HarnessId::DeepSeek => Some(crate::harness::DEFAULT_DEEPSEEK_MODEL.to_string()),
            crate::HarnessId::Claude => Some("claude-3-7-sonnet-20250219".to_string()),
            crate::HarnessId::Chat => Some("gpt-4o".to_string()),
            crate::HarnessId::Gemini => Some("gemini-3.7-flash-medium".to_string()),
            crate::HarnessId::Grok => Some("grok-4.6".to_string()),
            crate::HarnessId::Vibe => Some("mistral-medium-3.5".to_string()),
            // Muse keeps no harness default: `muse-spark-1.3` is a Model API
            // (#274 provider) id, and passing it as the CLI's `--model`
            // flag would change the reviewed #273 spawn argv with an
            // unverified value. The provider default lives in the tauri
            // `MUSE_DEFAULT_MODEL` fallback + `get_available_models`
            // instead, where it cannot leak into harness spawns.
            // Cursor uses its own account model config.
            crate::HarnessId::Muse | crate::HarnessId::Cursor | crate::HarnessId::Qwen => None,
        };
        let default_effort = match id {
            crate::HarnessId::Claude
            | crate::HarnessId::Chat
            | crate::HarnessId::Gemini
            | crate::HarnessId::Grok
            | crate::HarnessId::OpenCode
            | crate::HarnessId::DeepSeek => Some("medium".to_string()),
            crate::HarnessId::Vibe
            | crate::HarnessId::Muse
            | crate::HarnessId::Cursor
            | crate::HarnessId::Qwen => None,
        };
        Ok(Self {
            harness: id.as_str().to_string(),
            executable: if id == crate::HarnessId::Cursor {
                crate::harness::cursor_executable().to_string()
            } else {
                id.executable().to_string()
            },
            workdir: None,
            capture_polling: true,
            inject_permission: true,
            default_model,
            default_effort,
        })
    }
}

/// Effective harness view: global process settings plus the workspace's
/// selected default profile, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveHarnessSettings {
    pub harness: String,
    pub executable: String,
    pub workdir: Option<String>,
    pub capture_polling: bool,
    pub inject_permission: bool,
    pub default_profile: Option<String>,
    pub default_profile_status: FieldStatus,
    pub default_profile_badge: Option<String>,
    pub selected_model: Option<String>,
    pub selected_model_status: FieldStatus,
    pub selected_effort: Option<String>,
    pub selected_effort_status: FieldStatus,
}
