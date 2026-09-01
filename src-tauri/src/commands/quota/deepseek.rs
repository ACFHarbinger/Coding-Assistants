//! DeepSeek API balance adapter.
//!
//! DeepSeek exposes a real account-balance endpoint
//! (`GET https://api.deepseek.com/user/balance`), which is what this adapter
//! calls — nothing about DeepSeek goes through OpenCode. Captured live from a
//! real invocation, 2026-08-30:
//!
//! ```text
//! { "is_available": true,
//!   "balance_infos": [ { "currency": "USD", "total_balance": "12.34",
//!                        "granted_balance": "5.00", "topped_up_balance": "7.34" } ] }
//! ```
//!
//! The balance fields are **JSON strings, not numbers** (`"12.34"`), so the
//! serde fields below are `String` and parsed explicitly — a naive `f64`
//! field would fail to deserialize. Balance is a dollar amount, not a percent
//! window, so it is surfaced via the dedicated `ProviderQuota.balance` field
//! (rendered distinctly by the frontend) rather than `windows`.

use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota};

const HARNESS_TITLE: &str = "DeepSeek";
const AGENT_ID: &str = "deepseek";
const PROVIDER: &str = "deepseek";
const BALANCE_URL: &str = "https://api.deepseek.com/user/balance";

fn unavailable(detail: impl Into<String>) -> ProviderQuota {
    unavailable_quota(AGENT_ID, PROVIDER, HARNESS_TITLE, detail)
}

#[derive(serde::Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
    granted_balance: Option<String>,
    topped_up_balance: Option<String>,
}

#[derive(serde::Deserialize)]
struct BalanceResponse {
    is_available: Option<bool>,
    balance_infos: Vec<BalanceInfo>,
}

/// Format a balance line like `Balance $12.34 USD — granted $5.00 + topped-up $7.34`.
fn format_balance(info: &BalanceInfo) -> String {
    let mut parts = vec![format!("Balance ${} {}", info.total_balance, info.currency)];
    let mut breakdown = Vec::new();
    if let Some(granted) = info.granted_balance.as_deref().filter(|v| !v.is_empty()) {
        breakdown.push(format!("granted ${granted}"));
    }
    if let Some(topped) = info.topped_up_balance.as_deref().filter(|v| !v.is_empty()) {
        breakdown.push(format!("topped-up ${topped}"));
    }
    if !breakdown.is_empty() {
        parts.push(breakdown.join(" + "));
    }
    parts.join(" — ")
}

fn valid_amount(value: &str) -> bool {
    value
        .parse::<f64>()
        .is_ok_and(|amount| amount.is_finite() && amount >= 0.0)
}

fn validate_balance(info: &BalanceInfo) -> Result<(), &'static str> {
    if info.currency.trim().is_empty() {
        return Err("DeepSeek returned an empty balance currency");
    }
    if !valid_amount(info.total_balance.trim()) {
        return Err("DeepSeek returned an invalid total balance");
    }
    if info
        .granted_balance
        .as_deref()
        .is_some_and(|value| !valid_amount(value.trim()))
        || info
            .topped_up_balance
            .as_deref()
            .is_some_and(|value| !valid_amount(value.trim()))
    {
        return Err("DeepSeek returned an invalid balance breakdown");
    }
    Ok(())
}

pub(crate) fn deepseek_quota() -> ProviderQuota {
    // Secret hygiene: only ever read from the environment, never logged, never
    // echoed back into an error message or sent anywhere but api.deepseek.com.
    let api_key = match std::env::var("DEEPSEEK_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key,
        _ => {
            return unavailable(
                "DEEPSEEK_API_KEY is not set in the environment; set it to fetch the DeepSeek account balance",
            )
        }
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        // Never forward the bearer credential through an HTTP redirect. The
        // balance endpoint is fixed and redirects are not part of its API.
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(error) => return unavailable(format!("HTTP client error: {error}")),
    };
    let response = client
        .get(BALANCE_URL)
        .header("Accept", "application/json")
        .bearer_auth(&api_key)
        .send();
    let response = match response {
        Ok(response) => response,
        Err(error) => return unavailable(format!("DeepSeek balance request failed: {error}")),
    };
    if !response.status().is_success() {
        let status = response.status();
        return unavailable(format!("DeepSeek balance endpoint returned {status}"));
    }
    let balance: BalanceResponse = match response.json() {
        Ok(balance) => balance,
        Err(error) => {
            return unavailable(format!(
                "Unexpected response shape from DeepSeek balance: {error}"
            ))
        }
    };

    if balance.is_available == Some(false) {
        return unavailable("DeepSeek reports the account is unavailable");
    }
    let Some(info) = balance.balance_infos.first() else {
        return unavailable("DeepSeek returned no balance info");
    };
    if let Err(detail) = validate_balance(info) {
        return unavailable(detail);
    }

    let balance_text = format_balance(info);
    ProviderQuota {
        agent_id: AGENT_ID.into(),
        provider: PROVIDER.into(),
        harness_title: HARNESS_TITLE.into(),
        status: "ok".into(),
        detail: None,
        windows: Vec::new(),
        fetched_at: now_unix(),
        balance: Some(balance_text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_a_real_balance_snapshot() {
        let info = BalanceInfo {
            currency: "USD".into(),
            total_balance: "12.34".into(),
            granted_balance: Some("5.00".into()),
            topped_up_balance: Some("7.34".into()),
        };
        assert_eq!(
            format_balance(&info),
            "Balance $12.34 USD — granted $5.00 + topped-up $7.34"
        );
    }

    #[test]
    fn formats_a_balance_without_a_breakdown() {
        let info = BalanceInfo {
            currency: "CNY".into(),
            total_balance: "88.00".into(),
            granted_balance: None,
            topped_up_balance: None,
        };
        assert_eq!(format_balance(&info), "Balance $88.00 CNY");
    }

    #[test]
    fn empty_key_yields_unavailable_without_panicking() {
        let quota = unavailable("DEEPSEEK_API_KEY is not set in the environment; set it to fetch the DeepSeek account balance");
        assert_eq!(quota.agent_id, "deepseek");
        assert_eq!(quota.status, "unavailable");
        assert!(quota.detail.is_some());
        assert!(quota.balance.is_none());
    }

    #[test]
    fn balance_response_parses_string_fields() {
        // Real shape: the monetary fields are JSON strings, which would panic a
        // naive `f64` deserializer.
        let json = r#"{
            "is_available": true,
            "balance_infos": [
                { "currency": "USD", "total_balance": "12.34",
                  "granted_balance": "5.00", "topped_up_balance": "7.34" }
            ]
        }"#;
        let response: BalanceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.is_available, Some(true));
        assert_eq!(response.balance_infos.len(), 1);
        assert_eq!(response.balance_infos[0].total_balance, "12.34");
        assert_eq!(response.balance_infos[0].currency, "USD");
    }

    #[test]
    fn rejects_non_numeric_or_non_finite_amounts() {
        for amount in ["", "not-a-number", "NaN", "inf", "-1.00"] {
            let info = BalanceInfo {
                currency: "USD".into(),
                total_balance: amount.into(),
                granted_balance: None,
                topped_up_balance: None,
            };
            assert!(validate_balance(&info).is_err(), "accepted {amount:?}");
        }
    }
}
