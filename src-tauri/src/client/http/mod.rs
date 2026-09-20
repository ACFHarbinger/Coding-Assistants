//! Direct HTTP model providers (`platform.md` P4) and gateways (P13).
//!
//! Uses the already-vendored `async-openai` crate against OpenAI-compatible
//! `chat/completions` endpoints. Distinct from CLI harnesses (P3/C14) and
//! from the P4a Muse Spark reqwest adapter.

mod complete;
mod error;
mod gateway;

pub use complete::{
    chat_stream, ChatResult, OPENAI_DEFAULT_BASE, OPENAI_DEFAULT_MODEL, OPENAI_FALLBACK_MODELS,
    OPENAI_REQUEST_TIMEOUT_SECS,
};
pub use gateway::{
    chat_with_fallback, is_openrouter_provider, openrouter_is_authenticated,
    openrouter_unavailable_unauthenticated, split_fallback_models, OPENROUTER_DEFAULT_BASE,
    OPENROUTER_FALLBACK_MODELS, OPENROUTER_REQUEST_TIMEOUT_SECS,
};

pub fn is_openai_provider(provider: &str) -> bool {
    matches!(provider.trim(), "openai")
}

pub fn is_lm_studio_provider(provider: &str) -> bool {
    matches!(provider.trim(), "lm_studio" | "lm-studio")
}

/// Default LM Studio OpenAI-compatible listen address.
pub const LM_STUDIO_DEFAULT_BASE: &str = "http://127.0.0.1:1234";

/// Auth presence only: a non-empty `OPENAI_API_KEY`. Never reads key
/// material beyond presence — the key travels only in the request's
/// `Authorization` header, built at call time in `complete.rs`.
pub fn openai_is_authenticated(api_key: Option<&str>) -> bool {
    api_key
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

pub fn openai_unavailable_unauthenticated() -> String {
    "OpenAI unavailable: not authenticated. Add OPENAI_API_KEY in Settings → Credentials or export it in the environment.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_provider_key_is_exact() {
        assert!(is_openai_provider("openai"));
        assert!(is_openai_provider(" openai "));
        assert!(!is_openai_provider("opencode"));
        assert!(!is_openai_provider(""));
        assert!(is_lm_studio_provider("lm_studio"));
        assert!(is_lm_studio_provider("lm-studio"));
    }

    #[test]
    fn openai_auth_is_presence_only() {
        assert!(!openai_is_authenticated(None));
        assert!(!openai_is_authenticated(Some("  ")));
        assert!(openai_is_authenticated(Some("presence-flag-not-a-secret")));
        assert_eq!(OPENAI_DEFAULT_MODEL, "gpt-4o");
        assert_eq!(OPENAI_FALLBACK_MODELS, &["gpt-4o", "gpt-4o-mini"]);
    }

    #[test]
    fn openrouter_is_not_openai() {
        assert!(!is_openai_provider("openrouter"));
        assert!(is_openrouter_provider("openrouter"));
    }
}
