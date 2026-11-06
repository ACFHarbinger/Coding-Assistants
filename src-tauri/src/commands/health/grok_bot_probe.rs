//! Cheap Grok Bot direct-HTTP health probe (`platform.md` P15 / #320).
//!
//! HTTP-only: no binary. Distinct from the Grok CLI health row (`grok`).
//! `authenticated` follows `GROK_BOT_API_KEY` or fallback `XAI_API_KEY`.

use super::{health, Ids, ProviderHealth};

pub(crate) const GROK_BOT: Ids = Ids {
    agent_id: "grok-bot",
    provider: "grok-bot",
    title: "Grok Bot",
};

pub(crate) fn grok_bot_health_with(has_key: bool) -> ProviderHealth {
    health(
        &GROK_BOT,
        true,
        Some(has_key),
        None,
        if has_key {
            "GROK_BOT_API_KEY or XAI_API_KEY is configured"
        } else {
            "GROK_BOT_API_KEY is not set (XAI_API_KEY also accepted); add it in Settings → Credentials"
        },
    )
}

pub(crate) fn grok_bot_health() -> ProviderHealth {
    grok_bot_health_with(
        hub::secret::resolve("GROK_BOT_API_KEY").is_some()
            || hub::secret::resolve("XAI_API_KEY").is_some(),
    )
}
