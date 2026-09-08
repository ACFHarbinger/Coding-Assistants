//! Muse Spark (Meta Model API) quota adapter.
//!
//! Live-dashboard spike outcome (#280, re-verified 2026-09-08): when
//! `MODEL_API_KEY` is present the adapter still reports `unavailable` —
//! there is no key-authenticated usage surface to read, so there is
//! nothing to build a `ProviderQuotaWindow` from:
//! * `https://dev.meta.ai/usage` is a browser-session SPA route. From a
//!   key-only host it 302-redirects to `/error/geo/` and serves a generic
//!   geo-error shell with no usage-specific XHR / GraphQL / Connect shape
//!   to document. Its underlying calls ride the Meta browser session
//!   (cookies), not the `MODEL_API_KEY` Bearer credential —
//!   cookie-authenticated, so per the task contract: stop, no scraping,
//!   keep `unavailable`.
//! * `https://api.meta.ai` 401s every path pre-routing (`/v1/models` with
//!   a bogus Bearer fails identically to `/v1/usage`), so no usage or
//!   balance route is discoverable or confirmable without a real key —
//!   and none is documented. The official cookbook's only rate-limit
//!   guidance is to watch the `x-ratelimit-remaining-*` response headers
//!   to throttle before hitting a `429`: those headers ride **successful
//!   inference responses only**, and a quota panel that burns a completion
//!   per render to read them would consume the very budget it reports.
//!
//! So this adapter lands the explicitly acceptable state: `unavailable`
//! with an actionable detail, and it performs **zero network calls** in
//! every branch. Any dashboard-shaped failure degrades to the existing
//! `unavailable` row — never a hard error. (Same undocumented-endpoint
//! caveat as the Cursor case #281/#290 — owner-accepted, `unavailable` is
//! the safety net.)
//!
//! Resolves `MODEL_API_KEY` from the vault with the environment as fallback.
//! The key is presence-checked only — never logged, never echoed into details.

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
            "MODEL_API_KEY is not set; add it in Settings → Credentials or export it in the environment (usage still has no API surface to report)",
        );
    }
    unavailable(
        "the Meta Model API exposes no usage or balance endpoint — quota rides inference responses as x-ratelimit-* headers (see the official pricing and rate limits doc); track spend in the Meta developer dashboard",
    )
}

pub(crate) fn muse_quota() -> ProviderQuota {
    // Secret hygiene: presence-checked only, never logged, never echoed
    // back into a detail string or sent anywhere.
    let key = hub::secret::resolve("MODEL_API_KEY");
    muse_quota_with(key.as_ref().map(|secret| secret.expose()))
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
