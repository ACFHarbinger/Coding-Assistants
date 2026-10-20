//! Shared provider-quota probing for Coding-Assistants (extracted from
//! `src-tauri` in S5c / #358).
//!
//! This crate is the single home of the per-provider quota adapters
//! (session 5h / weekly 7d percent windows, balances, and local on-disk
//! usage) so both quota consumers depend on it:
//!
//! * `src-tauri` — the Usage tab IPC commands, harness health probes, and
//!   the S6 in-app schedule poller.
//! * `crates/cli` — headless `ca schedule tick`, which now gets the same
//!   full budget fidelity instead of credit-allowance-only.
//!
//! `hub` stays the lower layer: this crate depends on `hub` (secret
//! resolution, settings, schedule store), never the reverse.
//!
//! Boundaries preserved by construction:
//!
//! * **Secrets** — adapters resolve credentials exclusively through
//!   [`hub::secret::resolve`] (vault first, environment fallback) and
//!   receive a redacted `SecretString`; no secret value is ever logged or
//!   placed in an error/detail string.
//! * **Provenance** — every [`ProviderQuota`] carries `fetched_at` (UNIX
//!   seconds) and a `status`/`detail` pair saying exactly where the
//!   measurement came from (live endpoint, harness subprocess, on-disk
//!   accounting, or "unavailable" with the reason).
//! * **Provider API cost controls** — the `gemini` / `opencode` / `muse`
//!   probes cost a model turn, so they honor
//!   `orchestration.allow_metered_quota_probes` and short-circuit to
//!   "unavailable" when it is off; every other probe hits a free endpoint
//!   or reads local files. The schedule bridge probes each targeted agent
//!   at most once per tick.
//! * **Hard-stop semantics** — only a measurable session/weekly window
//!   with `remaining_percent <= 0` pauses a task; see
//!   [`schedule_probe`] and [`schedule_bridge`].

mod claude;
mod codex;
mod cursor;
mod deepseek;
mod gemini;
mod grok;
mod hermes;
mod kimi_usage;
mod mistral;
mod muse;
mod opencode;
mod probes;
mod qwen;
mod schedule_bridge;
mod schedule_probe;
mod vibe_usage;

pub use codex::{
    now_unix, ProviderQuota, ProviderQuotaBalance, ProviderQuotaLocalUsage, ProviderQuotaWindow,
};
pub use cursor::{cursor_auth_details, CursorAuthDetails};
pub use probes::{
    all_provider_quotas, all_provider_quotas_at, probe_agent_quota, probe_agent_quota_at,
};
pub use schedule_bridge::tick_with_quota_guard;
pub use schedule_probe::{classify_quota, target_quota_state, QuotaProbeState};
pub use vibe_usage::{vibe_whoami_facts, VibeWhoamiFacts};

#[cfg(test)]
#[path = "aggregate_tests.rs"]
mod aggregate_tests;
