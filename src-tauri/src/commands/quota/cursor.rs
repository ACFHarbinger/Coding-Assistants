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
//! * **Expected Response Schema** (camelCase or snake_case): `billingCycleEnd`
//!   (Unix ms, string or number); `planUsage` with cent integers `limit`,
//!   `includedSpend`, `totalSpend` (notional = included + `bonusSpend`),
//!   optional `onDemandSpend` (billed overage); and float percentages
//!   `autoPercentUsed` / `apiPercentUsed` / `totalPercentUsed` (0..100).
//! * **Hardening & Safe Degradation (#290)**:
//!   - The response shape is strictly validated against `check_usage_schema`.
//!   - If the endpoint response structure drifts or fields change types, a diagnostic warning
//!     is logged once (with structural details only, never leaking auth tokens or payload secrets)
//!     and the adapter degrades cleanly to `status: "unavailable"`.

use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow};
use base64::{
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
    Engine,
};
use chrono::{TimeZone, Utc};
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
        // The three bars `/usage` shows: overall, then Auto/Composer and API.
        if let Some(used) =
            object_field(plan, "totalPercentUsed", "total_percent_used").and_then(json_percent)
        {
            push_percent_window(&mut windows, "Included allowance", used, resets_at);
        }
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
            // No percentage fields at all: derive one from the cent integers.
            if let (Some(used), Some(limit)) = (
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

/// Balance line for the Usage card. The payload has no trustworthy "dollars
/// used" value (`totalSpend` is notional = `includedSpend` + `bonusSpend`;
/// `includedSpend` saturates at `limit`), so "$X used of $Y" phrasing was
/// always wrong. Consumption is the percentage windows; this line states only
/// the allowance and any positive `onDemandSpend` overage.
fn balance_from_period_usage(value: &Value) -> Option<String> {
    let plan = object_field(value, "planUsage", "plan_usage")?;
    let limit = object_field(plan, "limit", "limit").and_then(json_cents)?;
    let base = format!("{} included this cycle", format_cents(limit));
    match object_field(plan, "onDemandSpend", "on_demand_spend")
        .and_then(json_cents)
        .filter(|cents| *cents > 0)
    {
        Some(on_demand) => Some(format!("{base} · {} on-demand", format_cents(on_demand))),
        None => Some(base),
    }
}

pub(crate) fn cursor_auth_file() -> PathBuf {
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

pub(crate) fn token_from_auth_file(raw: &str) -> Option<String> {
    let value: Value = serde_json::from_str(raw).ok()?;
    value
        .get("accessToken")
        .or_else(|| value.get("access_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

/// Parse an expiration timestamp (seconds since Unix epoch) from a JSON object.
/// Checks `exp`, `expiresAt`, `expires_at`, and `expiry`. Numbers greater than
/// 100,000,000,000 are assumed to be in milliseconds and converted to seconds.
pub(crate) fn parse_expiry_value(value: &Value) -> Option<i64> {
    fn to_secs(num: i64) -> i64 {
        if num > 100_000_000_000 {
            num / 1000
        } else {
            num
        }
    }
    for key in ["exp", "expiresAt", "expires_at", "expiry"] {
        if let Some(field) = value.get(key) {
            if let Some(num) = field.as_i64().or_else(|| field.as_f64().map(|f| f as i64)) {
                return Some(to_secs(num));
            }
            if let Some(s) = field.as_str() {
                if let Ok(num) = s.parse::<i64>() {
                    return Some(to_secs(num));
                }
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                    return Some(dt.timestamp());
                }
            }
        }
    }
    None
}

/// Extract expiration from a JWT token's claims payload.
/// Secret hygiene: the token is split and payload decoded strictly to read the `exp`
/// timestamp claim; no token or payload string is retained or logged.
pub(crate) fn parse_jwt_expiry(token: &str) -> Option<i64> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_str = parts[1].trim();
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_str.trim_end_matches('='))
        .or_else(|_| URL_SAFE.decode(payload_str))
        .ok()?;
    let value: Value = serde_json::from_slice(&payload_bytes).ok()?;
    parse_expiry_value(&value)
}

fn epoch_secs_to_iso(secs: i64) -> Option<String> {
    match Utc.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => Some(dt.to_rfc3339()),
        _ => None,
    }
}

/// Health snapshot details for Cursor authentication (#294).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CursorAuthDetails {
    pub token_present: bool,
    pub auth_expires_at: Option<String>,
    pub is_expired: bool,
    pub file_present: bool,
}

impl CursorAuthDetails {
    pub(crate) fn detail(&self, installed: bool) -> &'static str {
        match (installed, self.token_present, self.is_expired) {
            (true, true, false) => "Cursor Agent is logged in",
            (true, true, true) => {
                "Cursor Agent access token expired; run `agent login` to refresh login"
            }
            (true, false, _) if self.file_present => {
                "Cursor Agent is installed but CLI auth file has no access token; run `agent login`"
            }
            (true, false, _) => "Cursor Agent is installed but not logged in; run `agent login`",
            (false, true, false) => {
                "The Cursor `agent` CLI was not found on PATH (credentials are present)"
            }
            (false, true, true) => {
                "The Cursor `agent` CLI was not found on PATH (credentials are expired)"
            }
            (false, false, _) => "The Cursor `agent` CLI was not found on PATH",
        }
    }
}

pub(crate) fn cursor_auth_details_from(
    vault_token: Option<&str>,
    file_content: Option<&str>,
    now_epoch: i64,
) -> CursorAuthDetails {
    if let Some(token) = vault_token.map(str::trim).filter(|t| !t.is_empty()) {
        let expiry = parse_jwt_expiry(token);
        let is_expired = expiry.map(|exp| exp <= now_epoch).unwrap_or(false);
        let auth_expires_at = expiry.and_then(epoch_secs_to_iso);
        return CursorAuthDetails {
            token_present: true,
            auth_expires_at,
            is_expired,
            file_present: false,
        };
    }

    let Some(raw) = file_content else {
        return CursorAuthDetails {
            token_present: false,
            auth_expires_at: None,
            is_expired: false,
            file_present: false,
        };
    };

    let token = token_from_auth_file(raw);
    let parsed_json: Option<Value> = serde_json::from_str(raw).ok();

    if let Some(token_str) = token {
        let expiry = parse_jwt_expiry(&token_str)
            .or_else(|| parsed_json.as_ref().and_then(parse_expiry_value));
        let is_expired = expiry.map(|exp| exp <= now_epoch).unwrap_or(false);
        let auth_expires_at = expiry.and_then(epoch_secs_to_iso);
        CursorAuthDetails {
            token_present: true,
            auth_expires_at,
            is_expired,
            file_present: true,
        }
    } else {
        CursorAuthDetails {
            token_present: false,
            auth_expires_at: None,
            is_expired: false,
            file_present: true,
        }
    }
}

pub(crate) fn cursor_auth_details() -> CursorAuthDetails {
    let vault_token = hub::secret::resolve("CURSOR_TOKEN");
    let file_content = std::fs::read_to_string(cursor_auth_file()).ok();
    cursor_auth_details_from(
        vault_token.as_ref().map(|s| s.expose()),
        file_content.as_deref(),
        Utc::now().timestamp(),
    )
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
        balance_info: None,
        local_usage: None,
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
