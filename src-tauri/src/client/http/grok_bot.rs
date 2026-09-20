//! xAI Grok Bot / consumer assistant API (`platform.md` P15 / #320).
//!
//! Distinct from `HarnessId::Grok` (the Grok CLI coding harness). This is a
//! direct-call OpenAI-compatible provider against `api.x.ai`. Auth is
//! `GROK_BOT_API_KEY`, falling back to `XAI_API_KEY`. Presence-only.

use super::complete::OPENAI_REQUEST_TIMEOUT_SECS;

pub const GROK_BOT_DEFAULT_BASE: &str = "https://api.x.ai/v1";
pub const GROK_BOT_DEFAULT_MODEL: &str = "grok-4.6";
pub const GROK_BOT_FALLBACK_MODELS: &[&str] = &["grok-4.6", "grok-4", "grok-3-mini"];
pub const GROK_BOT_REQUEST_TIMEOUT_SECS: u64 = OPENAI_REQUEST_TIMEOUT_SECS;

pub fn is_grok_bot_provider(provider: &str) -> bool {
    matches!(provider.trim(), "grok-bot" | "grok_bot")
}

pub fn grok_bot_is_authenticated(api_key: Option<&str>) -> bool {
    api_key
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

pub fn grok_bot_unavailable_unauthenticated() -> String {
    "Grok Bot unavailable: not authenticated. Add GROK_BOT_API_KEY or XAI_API_KEY in Settings → Credentials or export it.".into()
}

/// Prefer the dedicated grok-bot slot; reuse the existing xAI key if unset.
pub fn resolve_grok_bot_key() -> Option<hub::secret::SecretString> {
    hub::secret::resolve("GROK_BOT_API_KEY").or_else(|| hub::secret::resolve("XAI_API_KEY"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grok_bot_is_not_the_cli_harness() {
        assert!(is_grok_bot_provider("grok-bot"));
        assert!(is_grok_bot_provider("grok_bot"));
        assert!(is_grok_bot_provider(" grok-bot "));
        assert!(!is_grok_bot_provider("grok"));
        assert!(!is_grok_bot_provider("xai"));
        assert!(!is_grok_bot_provider("supergrok"));
        assert!(!is_grok_bot_provider("openrouter"));
        assert_eq!(GROK_BOT_DEFAULT_MODEL, "grok-4.6");
        assert_eq!(GROK_BOT_DEFAULT_BASE, "https://api.x.ai/v1");
        assert!(
            hub::HarnessId::parse("grok-bot").is_err(),
            "grok-bot must not collide with HarnessId::Grok"
        );
        assert_eq!(hub::HarnessId::parse("grok").unwrap(), hub::HarnessId::Grok);
    }

    #[test]
    fn grok_bot_auth_is_presence_only() {
        assert!(!grok_bot_is_authenticated(None));
        assert!(!grok_bot_is_authenticated(Some("  ")));
        assert!(grok_bot_is_authenticated(Some(
            "presence-flag-not-a-secret"
        )));
    }
}
