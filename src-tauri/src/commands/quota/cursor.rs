//! Cursor Agent plan quota adapter (#281, hardened in #290).
//!
//! # Data Source Contract & Stability Window
//!
//! * **Subcommand status**: The Cursor `agent` CLI currently exposes no machine-readable
//!   `agent usage` or `agent quota` command.
//! * **Endpoint contract**: Interactive `/usage` in the Cursor app loads
//!   `aiserver.v1.DashboardService/GetCurrentPeriodUsage` on `api2.cursor.sh` (Connect JSON
//!   protocol version 1). This is the same backend service driving the interactive dashboard
//!   spending panel.
//! * **Authentication**: Reads `CURSOR_TOKEN` from vault/environment (via `hub::secret::resolve`),
//!   falling back to the CLI login token written by `agent login` (`~/.config/cursor/auth.json` on
//!   Linux, `%APPDATA%\Cursor\auth.json` on Windows, `~/.cursor/auth.json` on macOS).
//! * **Expected Response Schema**:
//!   - `billingCycleEnd` / `billing_cycle_end`: Unix millisecond timestamp string or number.
//!   - `planUsage` / `plan_usage` object containing:
//!     - `totalSpend` / `total_spend`: Integer cents.
//!     - `includedSpend` / `included_spend`: Integer cents.
//!     - `limit`: Integer cents.
//!     - `autoPercentUsed` / `auto_percent_used`: Float or integer percentage (0..100).
//!     - `apiPercentUsed` / `api_percent_used`: Float or integer percentage (0..100).
//!     - `totalPercentUsed` / `total_percent_used`: Float or integer percentage (0..100).
//! * **Hardening & Safe Degradation (#290)**:
//!   - The response shape is strictly validated against `check_usage_schema`.
//!   - If the endpoint response structure drifts or fields change types, a diagnostic warning
//!     is logged once (with structural details only, never leaking auth tokens or payload secrets)
//!     and the adapter degrades cleanly to `status: "unavailable"`.

use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::OnceLock;

const AGENT_ID: &str = "cursor";
const PROVIDER: &str = "cursor";
const HARNESS_TITLE: &str = "Cursor Agent";
const USAGE_URL: &str = "https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage";
static SCHEMA_DRIFT_REPORTED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SchemaDriftReason {
    NotAnObject,
    MissingPlanUsage,
    PlanUsageNotAnObject,
    MissingExpectedFields,
    InvalidMetricType,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CursorContractSummary {
    pub has_billing_cycle: bool,
    pub auto_percent: Option<i32>,
    pub api_percent: Option<i32>,
    pub total_percent: Option<i32>,
    pub total_spend_cents: Option<i64>,
    pub limit_cents: Option<i64>,
}

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

/// Validate that the returned JSON conforms to the expected Cursor usage schema.
pub(crate) fn check_usage_schema(value: &Value) -> Result<(), SchemaDriftReason> {
    let obj = value.as_object().ok_or(SchemaDriftReason::NotAnObject)?;
    let plan = obj
        .get("planUsage")
        .or_else(|| obj.get("plan_usage"))
        .ok_or(SchemaDriftReason::MissingPlanUsage)?;
    let plan_obj = plan
        .as_object()
        .ok_or(SchemaDriftReason::PlanUsageNotAnObject)?;

    let metric_pairs = [
        ("autoPercentUsed", "auto_percent_used"),
        ("apiPercentUsed", "api_percent_used"),
        ("totalPercentUsed", "total_percent_used"),
        ("includedSpend", "included_spend"),
        ("totalSpend", "total_spend"),
    ];
    let mut has_metric = false;
    for (camel, snake) in metric_pairs {
        if let Some(metric) = plan_obj.get(camel).or_else(|| plan_obj.get(snake)) {
            has_metric = true;
            if json_f64(metric).is_none() {
                return Err(SchemaDriftReason::InvalidMetricType);
            }
        }
    }
    if !has_metric {
        return Err(SchemaDriftReason::MissingExpectedFields);
    }
    Ok(())
}

/// Verify the usage contract and extract a typed summary of present fields.
#[cfg(test)]
pub(crate) fn verify_usage_contract(
    value: &Value,
) -> Result<CursorContractSummary, SchemaDriftReason> {
    check_usage_schema(value)?;
    let plan = object_field(value, "planUsage", "plan_usage").unwrap();
    let has_billing_cycle = object_field(value, "billingCycleEnd", "billing_cycle_end")
        .or_else(|| object_field(plan, "billingCycleEnd", "billing_cycle_end"))
        .is_some();
    Ok(CursorContractSummary {
        has_billing_cycle,
        auto_percent: object_field(plan, "autoPercentUsed", "auto_percent_used")
            .and_then(json_percent),
        api_percent: object_field(plan, "apiPercentUsed", "api_percent_used")
            .and_then(json_percent),
        total_percent: object_field(plan, "totalPercentUsed", "total_percent_used")
            .and_then(json_percent),
        total_spend_cents: object_field(plan, "totalSpend", "total_spend").and_then(json_cents),
        limit_cents: object_field(plan, "limit", "limit").and_then(json_cents),
    })
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

fn cursor_auth_token_from_file() -> Result<String, String> {
    let path = cursor_auth_file();
    let raw = std::fs::read_to_string(&path).map_err(|_| {
        "Not logged in to Cursor Agent (no CLI auth file). Run `agent login`, or add CURSOR_TOKEN in Settings → Credentials."
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
    if let Err(drift) = check_usage_schema(period) {
        if SCHEMA_DRIFT_REPORTED.set(()).is_ok() {
            eprintln!(
                "[ca:quota:cursor] detected Cursor usage response schema drift: {drift:?}; degrading safely to unavailable"
            );
        }
        return unavailable(format!(
            "Cursor usage response schema drift detected ({drift:?}); degrading safely to unavailable"
        ));
    }
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
    // Keep a vault-resolved token in SecretString through authorization-header
    // construction. The CLI-file fallback is already an ordinary String from
    // the external CLI and has no additional app-side copy.
    if let Some(token) = hub::secret::resolve("CURSOR_TOKEN") {
        return match fetch_period_usage(token.expose()) {
            Ok(period) => cursor_quota_from_period(&period),
            Err(detail) => unavailable(detail),
        };
    }
    let token = match cursor_auth_token_from_file() {
        Ok(token) => token,
        Err(detail) => return unavailable(detail),
    };
    match fetch_period_usage(&token) {
        Ok(period) => cursor_quota_from_period(&period),
        Err(detail) => unavailable(detail),
    }
}

#[cfg(test)]
#[path = "cursor_tests.rs"]
mod tests;
