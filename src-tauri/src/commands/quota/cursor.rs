//! Cursor Agent plan quota adapter (#281).
//!
//! The Agent CLI has no usage subcommand. Interactive `/usage` in the Cursor
//! app loads `aiserver.v1.DashboardService/GetCurrentPeriodUsage` on
//! `api2.cursor.sh` — the same JSON the dashboard spending view uses. This
//! adapter calls that endpoint with the CLI login token (`agent login` writes
//! `~/.config/cursor/auth.json` on Linux; `CURSOR_AUTH_TOKEN` /
//! `CURSOR_API_KEY` override). It does not scrape `~/.cursor` cookies or HTML.
//!
//! Captured live, 2026-09-08, CLI `2026.09.02-c22c1a3` (numeric fields only):
//!
//! ```text
//! { "billingCycleEnd": "1791200599000",
//!   "planUsage": { "totalSpend": 7122, "includedSpend": 2000, "limit": 2000,
//!                  "autoPercentUsed": 12.5, "apiPercentUsed": 33.2,
//!                  "totalPercentUsed": 14.4 } }
//! ```
//!
//! Spend values are integer cents. Website `cursor.com/api/usage` still needs
//! a browser session cookie and is not used.

use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow};
use serde_json::Value;
use std::path::PathBuf;

const AGENT_ID: &str = "cursor";
const PROVIDER: &str = "cursor";
const HARNESS_TITLE: &str = "Cursor Agent";
const USAGE_URL: &str = "https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage";

fn unavailable(detail: impl Into<String>) -> ProviderQuota {
    unavailable_quota(AGENT_ID, PROVIDER, HARNESS_TITLE, detail)
}

fn json_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|n| n as f64))
        .or_else(|| value.as_u64().map(|n| n as f64))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn json_percent(value: &Value) -> Option<i32> {
    json_f64(value).map(|used| used.clamp(0.0, 100.0).round() as i32)
}

fn json_cents(value: &Value) -> Option<i64> {
    json_f64(value).map(|n| n.round() as i64)
}

/// Connect JSON may send proto timestamps as a millisecond string or number.
fn json_unix_seconds(value: &Value) -> Option<i64> {
    let n = json_f64(value)? as i64;
    Some(if n > 10_000_000_000 { n / 1000 } else { n })
}

fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.unsigned_abs();
    format!("{sign}${}.{:02}", cents / 100, cents % 100)
}

fn object_field<'a>(value: &'a Value, camel: &str, snake: &str) -> Option<&'a Value> {
    value.get(camel).or_else(|| value.get(snake))
}

fn push_percent_window(
    windows: &mut Vec<ProviderQuotaWindow>,
    label: &str,
    used: i32,
    resets_at: Option<i64>,
) {
    windows.push(ProviderQuotaWindow {
        label: label.into(),
        family: Some("Cursor Agent".into()),
        used_percent: used,
        remaining_percent: (100 - used).clamp(0, 100),
        resets_at,
        window_minutes: None,
    });
}

/// Pure parser so tests never touch the network or the login file.
pub(crate) fn windows_from_period_usage(value: &Value) -> Vec<ProviderQuotaWindow> {
    let plan = object_field(value, "planUsage", "plan_usage");
    let resets_at = object_field(value, "billingCycleEnd", "billing_cycle_end")
        .or_else(|| {
            plan.and_then(|plan| object_field(plan, "billingCycleEnd", "billing_cycle_end"))
        })
        .and_then(json_unix_seconds);

    let mut windows = Vec::new();
    if let Some(plan) = plan {
        if let Some(used) =
            object_field(plan, "autoPercentUsed", "auto_percent_used").and_then(json_percent)
        {
            push_percent_window(&mut windows, "Auto / Composer", used, resets_at);
        }
        if let Some(used) =
            object_field(plan, "apiPercentUsed", "api_percent_used").and_then(json_percent)
        {
            push_percent_window(&mut windows, "API", used, resets_at);
        }
        if windows.is_empty() {
            if let Some(used) =
                object_field(plan, "totalPercentUsed", "total_percent_used").and_then(json_percent)
            {
                push_percent_window(&mut windows, "Plan", used, resets_at);
            } else if let (Some(used), Some(limit)) = (
                object_field(plan, "includedSpend", "included_spend").and_then(json_cents),
                object_field(plan, "limit", "limit").and_then(json_cents),
            ) {
                if limit > 0 {
                    let percent = ((used as f64 / limit as f64) * 100.0)
                        .clamp(0.0, 100.0)
                        .round() as i32;
                    push_percent_window(&mut windows, "Included allowance", percent, resets_at);
                }
            }
        }
    }
    windows
}

fn balance_from_period_usage(value: &Value) -> Option<String> {
    let plan = object_field(value, "planUsage", "plan_usage")?;
    let total = object_field(plan, "totalSpend", "total_spend").and_then(json_cents)?;
    let limit = object_field(plan, "limit", "limit").and_then(json_cents)?;
    Some(format!(
        "{} used of {} included this cycle",
        format_cents(total),
        format_cents(limit)
    ))
}

fn cursor_auth_file() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        return PathBuf::from(appdata).join("Cursor").join("auth.json");
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        return PathBuf::from(home).join(".cursor").join("auth.json");
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            format!("{home}/.config")
        });
        PathBuf::from(config).join("cursor").join("auth.json")
    }
}

fn token_from_auth_file(raw: &str) -> Option<String> {
    let value: Value = serde_json::from_str(raw).ok()?;
    value
        .get("accessToken")
        .or_else(|| value.get("access_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

fn cursor_auth_token() -> Result<String, String> {
    for var in ["CURSOR_AUTH_TOKEN", "CURSOR_API_KEY"] {
        if let Ok(token) = std::env::var(var) {
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }
    }
    let path = cursor_auth_file();
    let raw = std::fs::read_to_string(&path).map_err(|_| {
        "Not logged in to Cursor Agent (no CLI auth file). Run `agent login`, or set CURSOR_AUTH_TOKEN."
            .to_string()
    })?;
    token_from_auth_file(&raw).ok_or_else(|| {
        "Cursor Agent is signed in but the CLI auth file has no access token. Run `agent login`."
            .into()
    })
}

fn fetch_period_usage(token: &str) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("HTTP client error: {error}"))?;
    let response = client
        .post(USAGE_URL)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .bearer_auth(token)
        .body("{}")
        .send()
        .map_err(|error| format!("Cursor usage request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Cursor usage endpoint returned {status}"));
    }
    response
        .json()
        .map_err(|error| format!("Unexpected response shape from Cursor usage: {error}"))
}

fn cursor_quota_from_period(period: &Value) -> ProviderQuota {
    let windows = windows_from_period_usage(period);
    let balance = balance_from_period_usage(period);
    ProviderQuota {
        agent_id: AGENT_ID.into(),
        provider: PROVIDER.into(),
        harness_title: HARNESS_TITLE.into(),
        status: if windows.is_empty() && balance.is_none() {
            "unavailable"
        } else {
            "ok"
        }
        .into(),
        detail: if windows.is_empty() && balance.is_none() {
            Some("Cursor usage endpoint returned no recognizable plan windows".into())
        } else {
            None
        },
        windows,
        fetched_at: now_unix(),
        balance,
    }
}

pub(crate) fn cursor_quota() -> ProviderQuota {
    let token = match cursor_auth_token() {
        Ok(token) => token,
        Err(detail) => return unavailable(detail),
    };
    match fetch_period_usage(&token) {
        Ok(period) => cursor_quota_from_period(&period),
        Err(detail) => unavailable(detail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIVE_SAMPLE: &str = r#"{
        "billingCycleStart": "1788608599000",
        "billingCycleEnd": "1791200599000",
        "planUsage": {
            "totalSpend": 7122,
            "includedSpend": 2000,
            "bonusSpend": 5122,
            "limit": 2000,
            "autoPercentUsed": 12.506666666666666,
            "apiPercentUsed": 33.2,
            "totalPercentUsed": 14.387878787878789
        },
        "spendLimitUsage": { "limitType": "user" },
        "enabled": true
    }"#;

    #[test]
    fn parses_live_dashboard_period_usage() {
        let value: Value = serde_json::from_str(LIVE_SAMPLE).unwrap();
        let windows = windows_from_period_usage(&value);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].label, "Auto / Composer");
        assert_eq!(windows[0].used_percent, 13);
        assert_eq!(windows[0].remaining_percent, 87);
        assert_eq!(windows[1].label, "API");
        assert_eq!(windows[1].used_percent, 33);
        assert_eq!(windows[0].resets_at, Some(1_791_200_599));
        assert_eq!(
            balance_from_period_usage(&value).as_deref(),
            Some("$71.22 used of $20.00 included this cycle")
        );
        let quota = cursor_quota_from_period(&value);
        assert_eq!(quota.agent_id, "cursor");
        assert_eq!(quota.status, "ok");
        assert!(quota.detail.is_none());
    }

    #[test]
    fn snake_case_payload_and_included_fallback_parse() {
        let value: Value = serde_json::from_str(
            r#"{ "billing_cycle_end": 1791200599,
                 "plan_usage": { "included_spend": 500, "limit": 2000 } }"#,
        )
        .unwrap();
        let windows = windows_from_period_usage(&value);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].label, "Included allowance");
        assert_eq!(windows[0].used_percent, 25);
        assert_eq!(windows[0].resets_at, Some(1_791_200_599));
    }

    #[test]
    fn missing_auth_file_is_unavailable_without_panic() {
        let quota = unavailable(
            "Not logged in to Cursor Agent (no CLI auth file). Run `agent login`, or set CURSOR_AUTH_TOKEN.",
        );
        assert_eq!(quota.status, "unavailable");
        assert!(quota.windows.is_empty());
        assert!(quota.balance.is_none());
        assert!(quota
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("agent login"));
    }

    #[test]
    fn auth_file_without_token_is_rejected() {
        assert!(token_from_auth_file("{}").is_none());
        assert!(token_from_auth_file(r#"{"accessToken":"  "}"#).is_none());
        assert_eq!(
            token_from_auth_file(r#"{"accessToken":"tok_live"}"#).as_deref(),
            Some("tok_live")
        );
        assert!(token_from_auth_file("not-json").is_none());
    }

    #[test]
    fn unauthenticated_cli_never_leaks_a_token_into_details() {
        let quota = unavailable("Cursor usage endpoint returned 401 Unauthorized");
        assert_eq!(quota.status, "unavailable");
        let detail = quota.detail.unwrap();
        assert!(!detail.contains("tok_"));
        assert!(!detail.contains("Bearer"));
    }
}
