//! Muse Spark (Meta Model API) quota adapter.
//!
//! Spike outcome (#280): the Model API exposes **no usage, balance, or
//! limits query endpoint**. Usage and cost are static documented tiers
//! (per-token pricing, RPM/TPM tables — see the official pricing and rate
//! limits doc), and the `x-ratelimit-*` headers ride **successful
//! inference responses only**. A quota panel that burns a completion per
//! render to read those headers would consume the very budget it reports,
//! and the dashboard usage route is cookie-authenticated GraphQL —
//! off-limits (no scraping). So this adapter lands the explicitly
//! acceptable state: `unavailable` with an actionable detail, and it
//! performs **zero network calls** in every branch.
//!
//! Reads `MODEL_API_KEY` from the environment for now (#287 migrates
//! provider credentials onto the vault resolver). The key is
//! presence-checked only — never logged, never echoed into details.

use super::quota_codex::{unavailable_quota, ProviderQuota};

const HARNESS_TITLE: &str = "Muse";
const AGENT_ID: &str = "muse";
const PROVIDER: &str = "muse";

fn unavailable(detail: impl Into<String>) -> ProviderQuota {
    unavailable_quota(AGENT_ID, PROVIDER, HARNESS_TITLE, detail)
}

/// Pure quota decision, separated from the environment read so tests never
/// touch the network (neither branch performs any I/O at all).
fn muse_quota_with(model_api_key: Option<&str>) -> ProviderQuota {
    let authenticated = model_api_key
        .map(str::trim)
        .is_some_and(|key| !key.is_empty());
    if !authenticated {
        return unavailable(
            "MODEL_API_KEY is not set in the environment; set it to enable the Muse Spark provider (usage still has no API surface to report)",
        );
    }
    unavailable(
        "the Meta Model API exposes no usage or balance endpoint — quota rides inference responses as x-ratelimit-* headers (see the official pricing and rate limits doc); track spend in the Meta developer dashboard",
    )
}

pub(crate) fn muse_quota() -> ProviderQuota {
    // Secret hygiene: presence-checked only, never logged, never echoed
    // back into a detail string or sent anywhere.
    muse_quota_with(std::env::var("MODEL_API_KEY").ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_names_the_env_var() {
        for key in [None, Some(""), Some("   ")] {
            let quota = muse_quota_with(key);
            assert_eq!(quota.agent_id, "muse");
            assert_eq!(quota.provider, "muse");
            assert_eq!(quota.status, "unavailable");
            assert_eq!(quota.balance, None);
            assert!(quota.windows.is_empty());
            let detail = quota.detail.unwrap();
            assert!(
                detail.contains("MODEL_API_KEY"),
                "unauthenticated detail must name the fix: {detail}"
            );
        }
    }

    #[test]
    fn authenticated_key_still_reports_no_usage_surface() {
        // Presence-only: the value is never inspected, and no request is
        // made — there is no endpoint to call.
        let quota = muse_quota_with(Some("presence-flag-not-a-secret"));
        assert_eq!(quota.status, "unavailable");
        assert_eq!(quota.balance, None);
        let detail = quota.detail.unwrap();
        assert!(
            detail.contains("no usage or balance endpoint"),
            "detail must state the documented gap: {detail}"
        );
        assert!(
            !detail.contains("presence-flag-not-a-secret"),
            "key material must never leak into details"
        );
    }
}
