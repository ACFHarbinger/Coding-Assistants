//! Claude Code quota adapter.
use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow};
use std::path::PathBuf;
#[derive(serde::Deserialize)]
struct ClaudeCredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: ClaudeOauthTokens,
}

#[derive(serde::Deserialize)]
struct ClaudeOauthTokens {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: i64,
}

#[derive(serde::Deserialize, Default)]
struct ClaudeUtilizationWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct ClaudeExtraUsage {
    utilization: Option<f64>,
}

#[derive(serde::Deserialize, Default)]
struct ClaudeUsageResponse {
    five_hour: Option<ClaudeUtilizationWindow>,
    seven_day: Option<ClaudeUtilizationWindow>,
    extra_usage: Option<ClaudeExtraUsage>,
}

pub(crate) fn claude_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude")
}

/// The `extra_usage` ("Usage credits") window has no `resets_at` in the
/// response — it's a monthly cap that resets on the 1st of each calendar
/// month (matches the desktop `/usage` UI's "Resets Sep 1" wording), so
/// compute it locally instead of guessing at an undocumented field name.
fn next_month_first_utc() -> Option<i64> {
    use chrono::{Datelike, TimeZone, Utc};
    let now = Utc::now();
    let (year, month) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .map(|dt| dt.timestamp())
}

fn push_claude_window(
    windows: &mut Vec<ProviderQuotaWindow>,
    label: &str,
    window: ClaudeUtilizationWindow,
) {
    let Some(used) = window.utilization else {
        return;
    };
    let used = used.clamp(0.0, 100.0);
    let resets_at = window
        .resets_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp());
    windows.push(ProviderQuotaWindow {
        label: label.into(),
        family: Some("Claude Model Family".into()),
        used_percent: used.round() as i32,
        remaining_percent: (100.0 - used).round() as i32,
        resets_at,
        window_minutes: None,
    });
}

/// Anthropic publishes no stable API for a Claude Code subscription's
/// session/weekly message-limit percentages (distinct from the per-API-key
/// `anthropic-ratelimit-*` headers, which are a token-billing concept, not
/// a subscription-plan one). This calls the same endpoint the official
/// `claude` CLI itself calls to render `/usage` — found by driving an
/// interactive `claude --debug` session and reading the debug log
/// (`fetchUtilization: GET /api/oauth/usage`), then verified directly with
/// the OAuth token from `~/.claude/.credentials.json`. It is undocumented
/// and can change or disappear on any Claude Code update with no notice —
/// every failure path below degrades to `unavailable_quota`, never a panic
/// or a raw error surfaced to the UI.
pub(crate) fn claude_quota() -> ProviderQuota {
    let creds_path = claude_home().join(".credentials.json");
    let bytes = match std::fs::read(&creds_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                "Not logged in to Claude Code (no ~/.claude/.credentials.json)",
            )
        }
    };
    let creds: ClaudeCredentialsFile = match serde_json::from_slice(&bytes) {
        Ok(creds) => creds,
        Err(error) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                format!("Could not parse Claude Code credentials: {error}"),
            )
        }
    };
    if creds.claude_ai_oauth.expires_at <= now_unix() * 1000 {
        return unavailable_quota(
            "claude",
            "anthropic",
            "Anthropic Claude Code",
            "Claude Code OAuth token expired; run `claude` to refresh login",
        );
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                format!("HTTP client error: {error}"),
            )
        }
    };
    let response = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(&creds.claude_ai_oauth.access_token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .send();
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                format!("request failed: {error}"),
            )
        }
    };
    if !response.status().is_success() {
        let status = response.status();
        return unavailable_quota(
            "claude",
            "anthropic",
            "Anthropic Claude Code",
            format!("Claude usage endpoint returned {status}"),
        );
    }
    let usage: ClaudeUsageResponse = match response.json() {
        Ok(usage) => usage,
        Err(error) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                format!("Unexpected response shape from Claude usage endpoint: {error}"),
            )
        }
    };

    let mut windows = Vec::new();
    if let Some(window) = usage.five_hour {
        push_claude_window(&mut windows, "Session", window);
    }
    if let Some(window) = usage.seven_day {
        push_claude_window(&mut windows, "Weekly (all models)", window);
    }
    if let Some(used) = usage.extra_usage.and_then(|extra| extra.utilization) {
        let used = used.clamp(0.0, 100.0);
        windows.push(ProviderQuotaWindow {
            label: "Usage credits".into(),
            family: Some("Claude Model Family".into()),
            used_percent: used.round() as i32,
            remaining_percent: (100.0 - used).round() as i32,
            resets_at: next_month_first_utc(),
            window_minutes: None,
        });
    }

    ProviderQuota {
        agent_id: "claude".into(),
        provider: "anthropic".into(),
        harness_title: "Anthropic Claude Code".into(),
        status: if windows.is_empty() {
            "unavailable"
        } else {
            "ok"
        }
        .into(),
        detail: if windows.is_empty() {
            Some("Claude usage endpoint returned no recognizable windows".into())
        } else {
            None
        },
        windows,
        fetched_at: now_unix(),
        balance: None,
    }
}
