//! Cheap OpenRouter gateway health probe (`platform.md` P13 / #318).
//!
//! HTTP-only: no binary. `authenticated` follows whether `OPENROUTER_API_KEY`
//! resolves through the vault or the environment. The key value is never
//! touched here, only its presence.

use super::{health, Ids, ProviderHealth};

pub(crate) const OPENROUTER: Ids = Ids {
    agent_id: "openrouter",
    provider: "openrouter",
    title: "OpenRouter",
};

pub(crate) fn openrouter_health_with(has_key: bool) -> ProviderHealth {
    health(
        &OPENROUTER,
        true,
        Some(has_key),
        None,
        if has_key {
            "OPENROUTER_API_KEY is configured"
        } else {
            "OPENROUTER_API_KEY is not set; add it in Settings → Credentials or export it"
        },
    )
}

pub(crate) fn openrouter_health() -> ProviderHealth {
    openrouter_health_with(hub::secret::resolve("OPENROUTER_API_KEY").is_some())
}
