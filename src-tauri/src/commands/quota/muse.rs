//! Muse Code (Meta) subscription-usage quota adapter.
//!
//! # What changed vs. #280
//!
//! #280 concluded there was "no key-authenticated usage surface": true for
//! the `MODEL_API_KEY` *dev-portal inference* credential, which this adapter
//! used to key off. The **harness login** is a different credential. `muse
//! login` (OAuth device flow) writes `~/.config/muse/auth.json` with
//! `providers.meta.api_key` — a Meta app token (`LLM|…`, distinct from
//! `MODEL_API_KEY`) — plus `providers.meta.api_base_url`
//! (`https://api.meta.ai/v1`).
//!
//! That `LLM|…` token is a plain Bearer for `POST {base}/responses` (an
//! OpenAI-Responses-shaped endpoint). When the request streams, the SSE
//! tail carries a dedicated frame:
//!
//! ```text
//! event: response.subscription_usage
//! data: {"subscription":{"tier":"…",
//!        "weekly":{"resets_at":1789344000,"used_percent":35},
//!        "window":{"resets_at":1788962220,"used_percent":5,
//!                  "window_duration_mins":300}},
//!        "type":"response.subscription_usage"}
//! ```
//!
//! `weekly` and the rolling `window` map straight onto `ProviderQuotaWindow`
//! (same shape as the Codex / Antigravity adapters). This is a Bearer API
//! call against a first-party endpoint — the same category as the Cursor
//! (#281) and DeepSeek adapters — not the cookie-authenticated `dev.meta.ai`
//! SPA that #280 correctly refused to scrape.
//!
//! # Cost
//!
//! There is no read-only usage route, so a snapshot costs one **minimal**
//! completion: `max_output_tokens: 16` (the API floor), `reasoning.effort:
//! "minimal"`, `store: false` — ~24 tokens. Negligible against a weekly
//! quota measured in whole percent, but non-zero, so this stays
//! refresh-scoped: `muse` is not in `QuotaStatusStrip`'s background poll.
//!
//! # Safety
//!
//! 12s timeout, redirects disabled. Any transport / auth / shape failure
//! degrades to `status: "unavailable"` with an actionable detail — never a
//! hard error. Secret hygiene: the key crosses IPC only as the
//! `Authorization` header value; it never enters a detail, a log, or an
//! error string.

use super::quota_codex::{
    now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow,
    METERED_PROBE_DISABLED_DETAIL,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

const AGENT_ID: &str = "muse";
const PROVIDER: &str = "muse";
const HARNESS_TITLE: &str = "Muse";
const DEFAULT_BASE_URL: &str = "https://api.meta.ai/v1";
const PROBE_MODEL: &str = "muse-spark-1.3";
const USER_AGENT: &str = "muse-code/launcher-2";
const WEEKLY_MINUTES: i64 = 7 * 24 * 60;
const FAMILY: &str = "Muse subscription";

fn unavailable(detail: impl Into<String>) -> ProviderQuota {
    unavailable_quota(AGENT_ID, PROVIDER, HARNESS_TITLE, detail)
}

fn muse_auth_path() -> PathBuf {
    let config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.config")
    });
    PathBuf::from(config).join("muse").join("auth.json")
}

/// `(api_key, base_url)` from a `muse` `auth.json` body. `None` unless a
/// non-empty `providers.meta.api_key` is present; `base_url` falls back to
/// the documented default and is only honoured when it is an `https` URL.
fn muse_api_creds_from(raw: &str) -> Option<(String, String)> {
    let meta = serde_json::from_str::<Value>(raw)
        .ok()?
        .get("providers")?
        .get("meta")?
        .clone();
    let key = meta
        .get("api_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|k| !k.is_empty())?
        .to_string();
    let base_url = meta
        .get("api_base_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|u| u.starts_with("https://"))
        .unwrap_or(DEFAULT_BASE_URL)
        .trim_end_matches('/')
        .to_string();
    Some((key, base_url))
}

/// Pull the `subscription` object out of a `response.subscription_usage`
/// SSE frame. Scans every `data:` line so frame ordering does not matter.
fn subscription_from_stream(body: &str) -> Option<Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data:").map(str::trim))
        .filter_map(|payload| serde_json::from_str::<Value>(payload).ok())
        .find(|value| {
            value.get("type").and_then(Value::as_str) == Some("response.subscription_usage")
        })
        .and_then(|value| value.get("subscription").cloned())
}

fn clamp_percent(value: &Value) -> Option<i32> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|n| n as f64))
        .map(|pct| pct.clamp(0.0, 100.0).round() as i32)
}

fn epoch_secs(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|n| n as i64))
        .filter(|n| *n > 0)
}

fn rolling_label(minutes: Option<i64>) -> String {
    match minutes {
        Some(mins) if mins > 0 && mins % 60 == 0 => format!("{}-hour rolling limit", mins / 60),
        Some(mins) if mins > 0 => format!("{mins}-minute rolling limit"),
        _ => "Rolling limit".to_string(),
    }
}

fn window(
    label: String,
    used: i32,
    resets_at: Option<i64>,
    minutes: Option<i64>,
) -> ProviderQuotaWindow {
    ProviderQuotaWindow {
        label,
        family: Some(FAMILY.to_string()),
        used_percent: used,
        remaining_percent: (100 - used).clamp(0, 100),
        resets_at,
        window_minutes: minutes,
    }
}

/// Pure parser: `{ "weekly": {...}, "window": {...}, "tier": "…" }` → windows.
fn windows_from_subscription(subscription: &Value) -> Vec<ProviderQuotaWindow> {
    let mut windows = Vec::new();
    if let Some(weekly) = subscription.get("weekly") {
        if let Some(used) = weekly.get("used_percent").and_then(clamp_percent) {
            windows.push(window(
                "Weekly limit".to_string(),
                used,
                weekly.get("resets_at").and_then(epoch_secs),
                Some(WEEKLY_MINUTES),
            ));
        }
    }
    if let Some(rolling) = subscription.get("window") {
        if let Some(used) = rolling.get("used_percent").and_then(clamp_percent) {
            let minutes = rolling.get("window_duration_mins").and_then(epoch_secs);
            windows.push(window(
                rolling_label(minutes),
                used,
                rolling.get("resets_at").and_then(epoch_secs),
                minutes,
            ));
        }
    }
    windows
}

fn muse_quota_from_subscription(subscription: &Value) -> ProviderQuota {
    let windows = windows_from_subscription(subscription);
    if windows.is_empty() {
        return unavailable(
            "Muse returned a subscription_usage frame with no weekly or rolling window",
        );
    }
    ProviderQuota {
        agent_id: AGENT_ID.into(),
        provider: PROVIDER.into(),
        harness_title: HARNESS_TITLE.into(),
        status: "ok".into(),
        detail: None,
        windows,
        fetched_at: now_unix(),
        balance: None,
    }
}

/// One minimal streamed completion; return the `subscription` object or a
/// sanitized reason. The response body is never echoed into the reason.
fn fetch_subscription(base_url: &str, api_key: &str) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("HTTP client error: {error}"))?;
    let response = client
        .post(format!("{base_url}/responses"))
        .header("Accept", "text/event-stream")
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .bearer_auth(api_key)
        .json(&json!({
            "model": PROBE_MODEL,
            "input": ".",
            "max_output_tokens": 16,
            "stream": true,
            "store": false,
            "reasoning": { "effort": "minimal" },
        }))
        .send()
        .map_err(|error| format!("Muse usage request failed: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => {
                "Muse harness login was rejected by api.meta.ai; run `muse` and sign in again"
                    .to_string()
            }
            429 => "Muse rate-limited the usage probe (HTTP 429); try again shortly".to_string(),
            code => format!("Muse usage endpoint returned HTTP {code}"),
        });
    }
    let body = response
        .text()
        .map_err(|error| format!("could not read Muse usage stream: {error}"))?;
    subscription_from_stream(&body)
        .ok_or_else(|| "Muse response carried no subscription_usage frame".to_string())
}

pub(crate) fn muse_quota(allow_metered: bool) -> ProviderQuota {
    // No read-only usage route exists; a snapshot costs one minimal
    // completion (~24 tokens). See the module header.
    if !allow_metered {
        return unavailable(METERED_PROBE_DISABLED_DETAIL);
    }
    let raw =
        match std::fs::read_to_string(muse_auth_path()) {
            Ok(raw) => raw,
            Err(_) => return unavailable(
                "No Muse harness login found (~/.config/muse/auth.json); run `muse` and sign in",
            ),
        };
    let Some((api_key, base_url)) = muse_api_creds_from(&raw) else {
        return unavailable(
            "Muse login at ~/.config/muse/auth.json has no Meta api_key; run `muse` and sign in again",
        );
    };
    match fetch_subscription(&base_url, &api_key) {
        Ok(subscription) => muse_quota_from_subscription(&subscription),
        Err(detail) => unavailable(detail),
    }
}

#[cfg(test)]
#[path = "muse_tests.rs"]
mod tests;
