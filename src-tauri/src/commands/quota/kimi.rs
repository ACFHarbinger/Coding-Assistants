//! Moonshot Kimi Code `/usage` plan-budget and booster-wallet quota adapter.
//!
//! Mirrors the interactive `/usage` slash command in the `kimi` CLI:
//! - Endpoint: `GET {base_url}/usages` (`https://api.kimi.ai/coding/v1/usages`
//!   or `https://api.kimi.com/coding/v1/usages` from `~/.kimi-code/config.toml`).
//! - Auth: `Authorization: Bearer <access_token>` from `~/.kimi-code/credentials/<key>.json`,
//!   refreshing via `POST {oauth_host}/api/oauth/token` when expired.
//! - Free metadata read (no model turn, no token spend); 8s timeout, redirects disabled.

#[path = "kimi_auth.rs"]
mod auth;

use super::quota_codex::{now_unix, ProviderQuotaBalance, ProviderQuotaWindow};
use super::quota_kimi_usage::kimi_home;
use auth::{
    find_credential_file, refresh_oauth_token, resolve_base_urls, resolve_oauth_host,
    save_refreshed_token, KimiStoredToken, DEFAULT_GLOBAL_BASE_URL,
};
#[cfg(test)]
pub(crate) use auth::{parse_kimi_config_toml, resolve_token_storage_name};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

const FIXED_POINT_CENTS: f64 = 1_000_000.0;
const FAMILY: &str = "Kimi Code";

/// Managed `/usage` plan-budget half of a Kimi quota. Merged with on-disk
/// `local_usage` (`kimi_usage.rs`) in `quotas::compose_kimi_quota`.
#[derive(Debug, Clone)]
pub(crate) struct KimiManagedBudget {
    pub windows: Vec<ProviderQuotaWindow>,
    pub balance: Option<String>,
    pub balance_info: Option<ProviderQuotaBalance>,
    pub detail: Option<String>,
}

fn json_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|n| n as f64))
        .or_else(|| value.as_u64().map(|n| n as f64))
        .or_else(|| value.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
        .filter(|n| n.is_finite())
}

fn json_i64(value: &Value) -> Option<i64> {
    json_f64(value).map(|n| n.trunc() as i64)
}

fn parse_rfc3339_secs(raw: Option<&Value>) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw?.as_str()?.trim())
        .ok()
        .map(|dt| dt.timestamp())
}

fn ratio_to_used_percent(ratio: f64) -> i32 {
    (ratio.clamp(0.0, 1.0) * 100.0).round() as i32
}

fn parse_quota_entry(raw: Option<&Value>) -> Option<(f64, Option<i64>)> {
    let obj = raw?.as_object()?;
    let ratio = obj
        .get("used_ratio")
        .or_else(|| obj.get("usedRatio"))
        .and_then(json_f64)?;
    let resets_at = parse_rfc3339_secs(obj.get("reset_time").or_else(|| obj.get("resetAt")));
    Some((ratio.clamp(0.0, 1.0), resets_at))
}

fn push_ratio_window(
    windows: &mut Vec<ProviderQuotaWindow>,
    label: impl Into<String>,
    used_ratio: f64,
    resets_at: Option<i64>,
    window_minutes: Option<i64>,
) {
    let used_percent = ratio_to_used_percent(used_ratio);
    windows.push(ProviderQuotaWindow {
        label: label.into(),
        family: Some(FAMILY.into()),
        used_percent,
        remaining_percent: (100 - used_percent).clamp(0, 100),
        resets_at,
        window_minutes,
    });
}

fn fixed_point_to_cents(value: i64) -> i64 {
    let cents = (value as f64) / FIXED_POINT_CENTS;
    if cents > 0.0 && cents < 1.0 {
        1
    } else {
        cents.round() as i64
    }
}

fn parse_money_cents(raw: Option<&Value>) -> Option<(i64, String)> {
    let obj = raw?.as_object()?;
    let cents = obj.get("priceInCents").and_then(json_i64)?;
    let currency = obj
        .get("currency")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    Some((cents, currency))
}

fn format_currency_cents(cents: i64, currency: &str) -> String {
    let amount = (cents.max(0) as f64) / 100.0;
    match currency.trim().to_ascii_uppercase().as_str() {
        "USD" | "$" | "" => format!("${amount:.2}"),
        "CNY" | "¥" => format!("¥{amount:.2}"),
        "EUR" | "€" => format!("€{amount:.2}"),
        "GBP" | "£" => format!("£{amount:.2}"),
        other => format!("{amount:.2} {other}"),
    }
}

/// Parse Kimi's `boosterWallet` (`extraUsage` in `/usage`) into `(balance, balance_info)`.
pub(crate) fn parse_booster_wallet(raw: Option<&Value>) -> Option<(String, ProviderQuotaBalance)> {
    let obj = raw?.as_object()?;
    let balance = obj.get("balance")?.as_object()?;
    if balance.get("type").and_then(Value::as_str) != Some("BOOSTER") {
        return None;
    }
    let amount_raw = balance
        .get("amount")
        .and_then(json_i64)
        .filter(|a| *a > 0)?;
    let _total_cents = fixed_point_to_cents(amount_raw);
    let balance_cents = balance
        .get("amountLeft")
        .and_then(json_i64)
        .map(fixed_point_to_cents)
        .unwrap_or(0);
    let monthly_limit = parse_money_cents(obj.get("monthlyChargeLimit"));
    let monthly_used = parse_money_cents(obj.get("monthlyUsed"));
    let monthly_limit_enabled = obj
        .get("monthlyChargeLimitEnabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let currency = monthly_limit
        .as_ref()
        .or(monthly_used.as_ref())
        .map(|(_, c)| c.as_str())
        .filter(|c| !c.is_empty())
        .unwrap_or("USD")
        .to_string();
    let used_cents = monthly_used.map(|(c, _)| c.max(0)).unwrap_or(0);
    let limit_cents = monthly_limit.map(|(c, _)| c.max(0)).unwrap_or(0);

    if monthly_limit_enabled && limit_cents > 0 {
        let summary = format!(
            "Extra usage: {} / {} this month · Balance {}",
            format_currency_cents(used_cents, &currency),
            format_currency_cents(limit_cents, &currency),
            format_currency_cents(balance_cents, &currency),
        );
        Some((
            summary,
            ProviderQuotaBalance {
                currency,
                total: (limit_cents as f64) / 100.0,
                kind: Some("spend".into()),
                spent: Some((used_cents as f64) / 100.0),
                granted: None,
                topped_up: None,
                paid: None,
                gift: (balance_cents > 0).then_some((balance_cents as f64) / 100.0),
                spend_cap: None,
            },
        ))
    } else {
        let summary = format!(
            "Extra usage balance {} · Used {} this month",
            format_currency_cents(balance_cents, &currency),
            format_currency_cents(used_cents, &currency),
        );
        Some((
            summary,
            ProviderQuotaBalance {
                currency,
                total: (balance_cents as f64) / 100.0,
                kind: Some("balance".into()),
                spent: (used_cents > 0).then_some((used_cents as f64) / 100.0),
                granted: None,
                topped_up: None,
                paid: Some((balance_cents as f64) / 100.0),
                gift: None,
                spend_cap: None,
            },
        ))
    }
}

fn parse_limits_fallback(raw: Option<&Value>) -> Vec<ProviderQuotaWindow> {
    let Some(items) = raw.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut windows = Vec::new();
    for item in items {
        let Some(detail) = item.get("detail").and_then(Value::as_object) else {
            continue;
        };
        let (Some(limit), Some(used)) = (
            detail.get("limit").and_then(json_f64).filter(|l| *l > 0.0),
            detail.get("used").and_then(json_f64),
        ) else {
            continue;
        };
        let window_minutes = item.get("window").and_then(Value::as_object).and_then(|w| {
            let d = w.get("duration").and_then(json_i64)?;
            match w.get("timeUnit").and_then(Value::as_str).unwrap_or("") {
                "TIME_UNIT_HOUR" => Some(d * 60),
                "TIME_UNIT_DAY" => Some(d * 24 * 60),
                _ => Some(d),
            }
        });
        let label = match window_minutes {
            Some(300) => "5h limit".into(),
            Some(10_080) => "Weekly limit".into(),
            Some(m) if m >= 28 * 24 * 60 => "Monthly limit".into(),
            Some(m) if m % 60 == 0 => format!("{}h limit", m / 60),
            Some(m) => format!("{m}m limit"),
            None => "Rate limit".into(),
        };
        let resets_at =
            parse_rfc3339_secs(detail.get("resetTime").or_else(|| detail.get("reset_time")));
        push_ratio_window(&mut windows, label, used / limit, resets_at, window_minutes);
    }
    windows
}

/// Pure parser for a `GET {base_url}/usages` payload, matching Kimi's `/usage` rows.
pub(crate) fn parse_managed_usage_payload(payload: &Value) -> Result<KimiManagedBudget, String> {
    let obj = payload
        .as_object()
        .ok_or_else(|| "Kimi /usages response is not a JSON object".to_string())?;
    let mut windows = Vec::new();
    if let Some(usages) = obj.get("usages").and_then(Value::as_object) {
        if let Some((ratio, resets_at)) =
            parse_quota_entry(usages.get("limit_5h").or_else(|| usages.get("limit5h")))
        {
            push_ratio_window(&mut windows, "5h limit", ratio, resets_at, Some(300));
        }
        if let Some((ratio, resets_at)) =
            parse_quota_entry(usages.get("limit_7d").or_else(|| usages.get("limit7d")))
        {
            push_ratio_window(&mut windows, "Weekly limit", ratio, resets_at, Some(10_080));
        }
        let month_total = parse_quota_entry(
            usages
                .get("limit_month_total")
                .or_else(|| usages.get("monthTotal")),
        );
        let month_code = parse_quota_entry(
            usages
                .get("limit_month_code")
                .or_else(|| usages.get("monthCode")),
        );
        if let Some((total_ratio, resets_at)) = month_total {
            let label = if let Some((code_ratio, _)) = month_code {
                let kimi_ratio = (((total_ratio - code_ratio) * 1_000_000.0).round() / 1_000_000.0)
                    .clamp(0.0, 1.0);
                format!(
                    "Monthly limit (kimi {}% · code {}%)",
                    ratio_to_used_percent(kimi_ratio),
                    ratio_to_used_percent(code_ratio)
                )
            } else {
                "Monthly limit".into()
            };
            push_ratio_window(&mut windows, label, total_ratio, resets_at, Some(43_200));
        }
        if let Some((code_ratio, resets_at)) = month_code {
            push_ratio_window(
                &mut windows,
                "Monthly code",
                code_ratio,
                resets_at,
                Some(43_200),
            );
        }
    }
    if windows.is_empty() {
        windows = parse_limits_fallback(obj.get("limits"));
    }
    let (balance, balance_info) =
        match parse_booster_wallet(obj.get("boosterWallet").or_else(|| obj.get("extraUsage"))) {
            Some((s, i)) => (Some(s), Some(i)),
            None => (None, None),
        };
    if windows.is_empty() && balance.is_none() {
        return Err("Kimi /usages endpoint returned no recognizable quota windows".into());
    }
    Ok(KimiManagedBudget {
        windows,
        balance,
        balance_info,
        detail: None,
    })
}

fn fetch_usages_json(
    client: &reqwest::blocking::Client,
    base_url: &str,
    token: &str,
) -> Result<Value, (Option<u16>, String)> {
    let url = format!("{}/usages", base_url.trim_end_matches('/'));
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "kimi-code-cli/2.3.0")
        .bearer_auth(token)
        .send()
        .map_err(|e| (None, format!("Kimi /usages request failed: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        let code = status.as_u16();
        let msg = match code {
            401 | 403 => {
                "Kimi /usages rejected the credentials (HTTP 401); run `kimi login` to authenticate"
                    .into()
            }
            404 => "Kimi /usages endpoint not available for this account (HTTP 404)".into(),
            _ => format!("Kimi /usages endpoint returned HTTP {status}"),
        };
        return Err((Some(code), msg));
    }
    response
        .json()
        .map_err(|e| (None, format!("Unexpected Kimi /usages response shape: {e}")))
}

pub(crate) fn kimi_managed_budget_from(home: &Path) -> Result<KimiManagedBudget, String> {
    let cfg = std::fs::read_to_string(home.join("config.toml"))
        .ok()
        .map(|raw| auth::parse_kimi_config_toml(&raw))
        .unwrap_or_default();
    let base_urls = resolve_base_urls(home, &cfg);
    let primary_base = base_urls
        .first()
        .map(String::as_str)
        .unwrap_or(DEFAULT_GLOBAL_BASE_URL);
    let oauth_host = resolve_oauth_host(&cfg, primary_base);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    if let Some(cred_path) = find_credential_file(home, cfg.oauth_key.as_deref()) {
        let raw = std::fs::read_to_string(&cred_path)
            .map_err(|e| format!("Could not read Kimi credentials: {e}"))?;
        let mut stored: KimiStoredToken = serde_json::from_str(&raw)
            .map_err(|e| format!("Could not parse Kimi credentials: {e}"))?;
        let expired = stored.expires_at > 0 && stored.expires_at <= now_unix();
        if (stored.access_token.trim().is_empty() || expired)
            && !stored.refresh_token.trim().is_empty()
        {
            if let Ok(refreshed) =
                refresh_oauth_token(&client, &oauth_host, stored.refresh_token.trim())
            {
                save_refreshed_token(&cred_path, &refreshed);
                stored = refreshed;
            }
        }
        if stored.access_token.trim().is_empty() {
            return Err(
                "Kimi Code OAuth credentials have no access token; run `kimi login`".into(),
            );
        }
        let mut last_err = "Kimi /usages returned no recognizable quota windows".to_string();
        for base_url in &base_urls {
            match fetch_usages_json(&client, base_url, stored.access_token.trim()) {
                Ok(payload) => return parse_managed_usage_payload(&payload),
                Err((Some(401), _)) if !stored.refresh_token.trim().is_empty() => {
                    let refreshed =
                        refresh_oauth_token(&client, &oauth_host, stored.refresh_token.trim())?;
                    save_refreshed_token(&cred_path, &refreshed);
                    stored = refreshed;
                    match fetch_usages_json(&client, base_url, stored.access_token.trim()) {
                        Ok(payload) => return parse_managed_usage_payload(&payload),
                        Err((_, msg)) => last_err = msg,
                    }
                }
                Err((_, msg)) => last_err = msg,
            }
        }
        return Err(last_err);
    }

    let token = cfg
        .api_key
        .filter(|k| !k.trim().is_empty())
        .or_else(|| hub::secret::resolve("KIMI_API_KEY").map(|s| s.expose().to_string()))
        .ok_or_else(|| {
            "Not logged in to Kimi Code (no ~/.kimi-code/credentials/*.json); run `kimi login` to enable `/usage` plan-budget windows.".to_string()
        })?;
    let mut last_err = "Kimi /usages returned no recognizable quota windows".to_string();
    for base_url in &base_urls {
        match fetch_usages_json(&client, base_url, token.trim()) {
            Ok(payload) => return parse_managed_usage_payload(&payload),
            Err((_, msg)) => last_err = msg,
        }
    }
    Err(last_err)
}

pub(crate) fn kimi_managed_budget() -> Result<KimiManagedBudget, String> {
    kimi_managed_budget_from(&kimi_home())
}

#[cfg(test)]
#[path = "kimi_tests.rs"]
mod tests;
