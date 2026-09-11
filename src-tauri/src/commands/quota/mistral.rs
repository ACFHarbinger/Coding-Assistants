//! Mistral Admin API quota adapter (S3, #306).
//!
//! Spend for a Mistral workspace comes from the Backoffice Admin API, not
//! from a completion key:
//!
//! ```text
//! base   https://api.mistral.ai/v1/admin
//! auth   x-api-key: <MISTRAL_ADMIN_API_KEY>
//! GET    /usage        spend by category, incl. a dedicated `vibe_usage`,
//!                      plus `currency` / `start_date` / `end_date`
//! GET    /spend-limit  `{ "amount": <number>, "no_monthly_limit": <bool> }`
//! ```
//!
//! **This is the first non-Bearer [REDACTED] header in the repo.** Every
//! other quota adapter sends `Authorization: Bearer <key>`; Mistral's Admin
//! API instead expects the raw admin key in a dedicated `x-api-key` header
//! with no scheme prefix. A regular completion `MISTRAL_API_KEY` does not
//! work here — the admin key is created separately in the Backoffice — so a
//! 401/403 names the right credential instead of blaming the login.
//!
//! The two reads compose into one budget: total spend ÷ monthly limit becomes
//! a `"Monthly spend"` percent window, and the currency figure becomes
//! `balance_info`. `no_monthly_limit` (or a missing/invalid `amount`) means
//! there is no cap to divide by, so the adapter reports spend only and no
//! window — never a fabricated percentage.
//!
//! # Cost
//!
//! Both endpoints are free metadata reads: no model turn, no token spend, so
//! — unlike the `gemini` / `opencode` / `muse` adapters — this takes **no**
//! `allow_metered` parameter and is always attempted when a key exists.
//!
//! # Safety
//!
//! `reqwest::blocking`, 10s timeout, redirects disabled. Any transport, auth,
//! or shape failure degrades to `Err(detail)` — the caller keeps the landed
//! `local_usage` read-out and reports the admin state in its own detail —
//! never a hard error. Secret hygiene: the key crosses IPC only as the
//! `x-api-key` header value; it never enters a detail, a log, or an error
//! string.

use super::quota_codex::{ProviderQuotaBalance, ProviderQuotaWindow};
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;

const ADMIN_BASE: &str = "https://api.mistral.ai/v1/admin";
const FAMILY: &str = "Mistral Admin";

static SCHEMA_DRIFT_REPORTED: OnceLock<()> = OnceLock::new();

/// A non-negative finite amount, tolerating integer, float, and numeric-string
/// JSON shapes the way `cursor.rs` does.
fn json_amount(value: &Value) -> Option<f64> {
    let amount = value
        .as_f64()
        .or_else(|| value.as_i64().map(|n| n as f64))
        .or_else(|| value.as_u64().map(|n| n as f64))
        .or_else(|| {
            value
                .as_str()
                .and_then(|raw| raw.trim().parse::<f64>().ok())
        })?;
    (amount.is_finite() && amount >= 0.0).then_some(amount)
}

/// `(category, spend)` pairs out of a `/usage` body. Accepts both a map
/// (`"usage": { "vibe_usage": 12.5, ... }`) and a list
/// (`"usage": [{ "category": "vibe_usage", "spend": 12.5 }, ...]`);
/// non-numeric entries are skipped, not fatal.
fn spend_entries(body: &Value) -> Vec<(String, f64)> {
    let usage = match body.get("usage") {
        Some(usage) => usage,
        None => return Vec::new(),
    };
    if let Some(map) = usage.as_object() {
        return map
            .iter()
            .filter_map(|(category, spend)| {
                json_amount(spend).map(|spend| (category.clone(), spend))
            })
            .collect();
    }
    if let Some(list) = usage.as_array() {
        return list
            .iter()
            .filter_map(|entry| {
                let category = entry
                    .get("category")
                    .or_else(|| entry.get("name"))
                    .and_then(Value::as_str)?;
                let spend = entry
                    .get("spend")
                    .or_else(|| entry.get("amount"))
                    .or_else(|| entry.get("value"))
                    .and_then(json_amount)?;
                Some((category.to_string(), spend))
            })
            .collect();
    }
    Vec::new()
}

fn string_field(body: &Value, key: &str) -> Option<String> {
    body.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
}

struct UsageSummary {
    currency: String,
    total_spend: f64,
    vibe_spend: Option<f64>,
    start_date: Option<String>,
    end_date: Option<String>,
}

/// Pure parser for a `GET /usage` body. `None` when no numeric spend entry
/// survives — the caller treats that as schema drift, not as zero spend.
fn summarize_usage(body: &Value) -> Option<UsageSummary> {
    let entries = spend_entries(body);
    if entries.is_empty() {
        return None;
    }
    let total_spend = entries.iter().map(|(_, spend)| spend).sum();
    let vibe_spend = entries
        .iter()
        .find(|(category, _)| category == "vibe_usage")
        .map(|(_, spend)| *spend);
    Some(UsageSummary {
        currency: string_field(body, "currency").unwrap_or_else(|| "USD".into()),
        total_spend,
        vibe_spend,
        start_date: string_field(body, "start_date"),
        end_date: string_field(body, "end_date"),
    })
}

struct SpendLimit {
    amount: Option<f64>,
    no_monthly_limit: bool,
}

fn parse_spend_limit(body: &Value) -> SpendLimit {
    SpendLimit {
        amount: body.get("amount").and_then(json_amount),
        no_monthly_limit: body
            .get("no_monthly_limit")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

/// The admin-API half of a Mistral quota: percent window(s), a human-readable
/// balance line, and the structured currency figure. `local_usage` is filled
/// in separately by `vibe_usage.rs` — this struct never touches it.
pub(crate) struct MistralAdminBudget {
    pub windows: Vec<ProviderQuotaWindow>,
    pub balance: Option<String>,
    pub balance_info: Option<ProviderQuotaBalance>,
    pub detail: Option<String>,
}

fn format_spend(summary: &UsageSummary) -> String {
    let mut line = format!(
        "Spent ${:.2} {} this period",
        summary.total_spend, summary.currency
    );
    if let Some(vibe) = summary.vibe_spend {
        line.push_str(&format!(" (Vibe ${vibe:.2})"));
    }
    if let (Some(start), Some(end)) = (&summary.start_date, &summary.end_date) {
        line.push_str(&format!(" · {start} → {end}"));
    }
    line
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SpendLimitResult {
    Capped(f64),
    Uncapped,
    Failed(String),
}

/// Pure composition of the two endpoint bodies into a budget.
fn budget_from_summary(summary: &UsageSummary, limit: SpendLimitResult) -> MistralAdminBudget {
    let balance_info = ProviderQuotaBalance {
        currency: summary.currency.clone(),
        total: summary.total_spend,
        kind: Some("spend".into()),
        granted: None,
        topped_up: None,
        paid: None,
        gift: None,
    };
    match limit {
        SpendLimitResult::Capped(cap) => {
            let used = ((summary.total_spend / cap) * 100.0)
                .clamp(0.0, 100.0)
                .round() as i32;
            MistralAdminBudget {
                windows: vec![ProviderQuotaWindow {
                    label: "Monthly spend".into(),
                    family: Some(FAMILY.into()),
                    used_percent: used,
                    remaining_percent: (100 - used).clamp(0, 100),
                    resets_at: None,
                    window_minutes: None,
                }],
                balance: Some(format_spend(summary)),
                balance_info: Some(balance_info),
                detail: None,
            }
        }
        SpendLimitResult::Uncapped => MistralAdminBudget {
            windows: Vec::new(),
            balance: Some(format_spend(summary)),
            balance_info: Some(balance_info),
            detail: Some(
                "No monthly spend cap is set on this Mistral workspace — reporting spend only."
                    .into(),
            ),
        },
        SpendLimitResult::Failed(err) => MistralAdminBudget {
            windows: Vec::new(),
            balance: Some(format_spend(summary)),
            balance_info: Some(balance_info),
            detail: Some(format!(
                "Monthly spend cap could not be read from Mistral Admin API ({err}) — reporting spend only."
            )),
        },
    }
}

fn report_drift(reason: impl Into<String>) -> String {
    let detail = reason.into();
    if SCHEMA_DRIFT_REPORTED.set(()).is_ok() {
        eprintln!("[ca:quota:mistral] Admin API response schema drift: {detail}; degrading safely");
    }
    detail
}

fn fetch_admin_json(
    client: &reqwest::blocking::Client,
    path: &str,
    api_key: &str,
) -> Result<Value, String> {
    let response = client
        .get(format!("{ADMIN_BASE}{path}"))
        .header("Accept", "application/json")
        .header("x-api-key", api_key)
        .send()
        .map_err(|error| format!("Mistral Admin API {path} request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => format!(
                "Mistral Admin API rejected the admin key (HTTP {status}); \
                 MISTRAL_ADMIN_API_KEY needs a Backoffice admin key, not a completion API key"
            ),
            429 => "Mistral Admin API rate-limited the request (HTTP 429); try again shortly"
                .to_string(),
            _ => format!("Mistral Admin API {path} returned HTTP {status}"),
        });
    }
    response.json().map_err(|error| {
        report_drift(format!(
            "Unexpected response shape from Mistral Admin API {path}: {error}"
        ))
    })
}

/// The admin-API budget half. `Ok` carries windows/balance for the caller to
/// merge with `local_usage`; `Err` carries a user-facing reason and the
/// caller falls back to the local-only read-out.
pub(crate) fn mistral_admin_budget() -> Result<MistralAdminBudget, String> {
    // Secret hygiene: resolved from the vault (keychain / file) or the
    // environment as fallback via hub::secret::resolve — never logged, never
    // echoed back into an error message, never sent anywhere but
    // api.mistral.ai.
    let api_key = match hub::secret::resolve("MISTRAL_ADMIN_API_KEY") {
        Some(secret) => secret,
        None => {
            return Err(
                "MISTRAL_ADMIN_API_KEY is not set; add it in Settings → Credentials \
                        or export it in the environment. Local Vibe session usage still applies."
                    .into(),
            );
        }
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        // Never forward the admin key through an HTTP redirect. Both admin
        // endpoints are fixed and redirects are not part of their API.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("HTTP client error: {error}"))?;

    let usage = fetch_admin_json(&client, "/usage", api_key.expose())?;
    let Some(summary) = summarize_usage(&usage) else {
        return Err(report_drift(
            "Mistral Admin API /usage returned no recognizable spend entries",
        ));
    };

    // A failed or uncapped `/spend-limit` read degrades to spend-only, not to
    let limit = match fetch_admin_json(&client, "/spend-limit", api_key.expose()) {
        Ok(body) => {
            let limit = parse_spend_limit(&body);
            if limit.no_monthly_limit {
                SpendLimitResult::Uncapped
            } else if let Some(amount) = limit.amount.filter(|a| *a > 0.0) {
                SpendLimitResult::Capped(amount)
            } else {
                SpendLimitResult::Failed(
                    "Spend limit response contained neither a valid amount nor no_monthly_limit"
                        .into(),
                )
            }
        }
        Err(err) => SpendLimitResult::Failed(err),
    };
    Ok(budget_from_summary(&summary, limit))
}

#[cfg(test)]
#[path = "mistral_tests.rs"]
mod tests;
