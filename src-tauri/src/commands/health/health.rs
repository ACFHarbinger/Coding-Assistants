//! Typed provider **health** — the cheap `status` half of `platform.md` P3.
//!
//! The quota adapters (`commands/quota/*`) answer "how much budget is left",
//! which costs a real HTTP round-trip or a multi-second subprocess (`agy
//! --print "/usage"` alone allows 25s). That is too expensive to poll for a
//! readiness dot, and process discovery alone says nothing about whether the
//! CLI is *logged in*. This module answers the cheaper question: is the
//! provider's binary present, is a credential on disk, and — where the
//! credential carries an expiry we can read without a network call — is it
//! still valid.
//!
//! Rules, mirrored from the quota adapters:
//! - **No usage call, no long-timeout network.** `endpoint_reachable` stays
//!   `None` unless a sub-second check is explicitly wired for that provider
//!   (none are, today).
//! - **Secret hygiene.** A credential value never enters `detail`, a log, a
//!   `Debug` impl, or an error string. Only presence / expiry / reachability
//!   cross IPC.
//! - **Never panic.** Every path degrades to `installed: false` /
//!   `authenticated: None` with an actionable, secret-free `detail`.

use super::creative_tools::resolve_binary;
use chrono::{TimeZone, Utc};
use std::path::PathBuf;

/// One provider's cheap health snapshot.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderHealth {
    /// Stable agent id used across the app (`claude`, `chat`, `gemini`, …).
    pub agent_id: String,
    /// Provider/vendor slug (`anthropic`, `openai`, `google`, …).
    pub provider: String,
    /// Human-readable harness/provider title for the UI.
    pub harness_title: String,
    /// The CLI this provider needs resolves on `PATH`. `true` for pure-HTTP
    /// providers that need no local binary (e.g. DeepSeek).
    pub installed: bool,
    /// `Some(true)`  — a credential is present and (where cheaply checkable)
    ///                 unexpired.
    /// `Some(false)` — the credential is definitely missing or expired.
    /// `None`        — cannot tell without a network call / interactive login.
    pub authenticated: Option<bool>,
    /// ISO-8601 credential expiry, when the on-disk credential carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_expires_at: Option<String>,
    /// Only populated when a sub-second reachability probe is wired for the
    /// provider. `None` means "not checked", not "unreachable".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_reachable: Option<bool>,
    /// Actionable, secret-free one-liner for the UI.
    pub detail: String,
    /// ISO-8601 timestamp of this probe.
    pub checked_at: String,
}

pub(crate) fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub(crate) fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
}

/// Render a UNIX epoch (seconds) as ISO-8601, or `None` if out of range.
pub(crate) fn epoch_secs_to_iso(secs: i64) -> Option<String> {
    match Utc.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => Some(dt.to_rfc3339()),
        _ => None,
    }
}

pub(crate) struct Ids {
    agent_id: &'static str,
    provider: &'static str,
    title: &'static str,
}

/// Binary present but auth undeterminable without a network / interactive step.
pub(crate) fn binary_only(ids: &Ids, bin: &str) -> ProviderHealth {
    match resolve_binary(bin) {
        Some(_) => health(
            ids,
            true,
            None,
            None,
            format!("`{bin}` is installed; run it once to confirm login"),
        ),
        None => not_installed(ids, bin),
    }
}

pub(crate) fn not_installed(ids: &Ids, bin: &str) -> ProviderHealth {
    health(
        ids,
        false,
        Some(false),
        None,
        format!("`{bin}` was not found on PATH"),
    )
}

pub(crate) fn health(
    ids: &Ids,
    installed: bool,
    authenticated: Option<bool>,
    auth_expires_at: Option<String>,
    detail: impl Into<String>,
) -> ProviderHealth {
    ProviderHealth {
        agent_id: ids.agent_id.into(),
        provider: ids.provider.into(),
        harness_title: ids.title.into(),
        installed,
        authenticated,
        auth_expires_at,
        endpoint_reachable: None,
        detail: detail.into(),
        checked_at: now_iso(),
    }
}

mod probes;
pub(crate) use probes::*;

fn probe(agent_id: &str) -> ProviderHealth {
    match agent_id {
        "claude" => claude_health(),
        "chat" | "codex" => codex_health(),
        "gemini" => gemini_health(),
        "grok" => grok_health(),
        "opencode" => opencode_health(),
        "deepseek" => deepseek_health(),
        "mistral" | "vibe" => mistral_health(),
        "muse" => muse_health(),
        "cursor" => cursor_health(),
        "ollama" => ollama_health(),
        "llamacpp" => llamacpp_health(),
        other => ProviderHealth {
            agent_id: other.into(),
            provider: "unknown".into(),
            harness_title: other.into(),
            installed: false,
            authenticated: None,
            auth_expires_at: None,
            endpoint_reachable: None,
            detail: "Unknown provider agent id".into(),
            checked_at: now_iso(),
        },
    }
}

const ALL_AGENT_IDS: &[&str] = &[
    "claude", "grok", "chat", "cursor", "gemini", "opencode", "deepseek", "muse", "mistral",
    "llamacpp", "ollama",
];

/// Cheap by design (a few `stat`/`read` calls, no network), but still uses
/// `spawn_blocking` for parity with `hub_get_provider_quotas` and to keep
/// filesystem stalls (e.g. a slow `$HOME` on a network mount) off the IPC
/// thread.
#[tauri::command]
pub async fn hub_get_provider_health() -> Result<Vec<ProviderHealth>, String> {
    tauri::async_runtime::spawn_blocking(|| ALL_AGENT_IDS.iter().map(|id| probe(id)).collect())
        .await
        .map_err(|error| format!("provider health task panicked: {error}"))
}

#[tauri::command]
pub async fn hub_refresh_provider_health(agent_id: String) -> Result<ProviderHealth, String> {
    tauri::async_runtime::spawn_blocking(move || probe(&agent_id))
        .await
        .map_err(|error| format!("provider health refresh task panicked: {error}"))
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;
