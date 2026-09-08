//! Cursor Agent plan quota adapter (#281).
//!
//! Live spike against Cursor Agent CLI `2026.09.02-c22c1a3`:
//!
//! - `agent status --format json` exposes authentication and user identity.
//! - `agent about --format json` exposes CLI version and subscription tier.
//! - `agent models` exposes the account's model catalog.
//! - The command catalog has no usage, quota, limits, billing, or balance
//!   subcommand, and neither JSON response contains usage counters.
//! - `agent -p "/usage" --output-format stream-json` treats `/usage` as an
//!   ordinary model prompt. Its result-level `usage` object contains only
//!   that request's token counts, not account plan limits.
//!
//! Therefore there is no documented machine-readable remaining-plan surface
//! to parse. This adapter deliberately performs no subprocess, network, or
//! filesystem I/O (especially no `~/.cursor` scraping) and returns the
//! explicitly accepted `unavailable` state.

use super::quota_codex::{unavailable_quota, ProviderQuota};

const AGENT_ID: &str = "cursor";
const PROVIDER: &str = "cursor";
const HARNESS_TITLE: &str = "Cursor Agent";
const DETAIL: &str = "Cursor Agent exposes no machine-readable plan usage or remaining request limits; view them with /usage in the Cursor app or Cursor Dashboard → Usage";

pub(crate) fn cursor_quota() -> ProviderQuota {
    unavailable_quota(AGENT_ID, PROVIDER, HARNESS_TITLE, DETAIL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_typed_unavailable_row() {
        let quota = cursor_quota();
        assert_eq!(quota.agent_id, "cursor");
        assert_eq!(quota.provider, "cursor");
        assert_eq!(quota.harness_title, "Cursor Agent");
        assert_eq!(quota.status, "unavailable");
        assert!(quota.windows.is_empty());
        assert!(quota.balance.is_none());
        assert!(quota.detail.as_deref().is_some_and(|detail| {
            detail.contains("no machine-readable plan usage") && detail.contains("/usage")
        }));
    }

    #[test]
    fn missing_or_unauthenticated_cli_never_panics_or_performs_io() {
        // The no-surface landing state is static by design: it neither
        // resolves nor executes the CLI, so missing/unauthenticated installs
        // produce the same truthful row without touching global PATH or auth.
        for _ in 0..2 {
            assert_eq!(cursor_quota().status, "unavailable");
        }
    }
}
