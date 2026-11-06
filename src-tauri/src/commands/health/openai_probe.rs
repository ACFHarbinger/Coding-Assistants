//! Cheap OpenAI direct-HTTP health probe (`platform.md` P4 / #327).
//!
//! HTTP-only: no binary. `authenticated` follows whether `OPENAI_API_KEY`
//! resolves through the vault or the environment. The key value is never
//! touched here, only its presence.

use super::{health, Ids, ProviderHealth};

pub(crate) const OPENAI: Ids = Ids {
    agent_id: "openai",
    provider: "openai",
    title: "OpenAI",
};

pub(crate) fn openai_health_with(has_key: bool) -> ProviderHealth {
    health(
        &OPENAI,
        true,
        Some(has_key),
        None,
        if has_key {
            "OPENAI_API_KEY is configured"
        } else {
            "OPENAI_API_KEY is not set; add it in Settings → Credentials or export it"
        },
    )
}

pub(crate) fn openai_health() -> ProviderHealth {
    openai_health_with(hub::secret::resolve("OPENAI_API_KEY").is_some())
}
