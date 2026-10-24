use super::*;
use serde_json::json;

/// Verbatim live response from `GET https://api.kimi.ai/coding/v1/usages`
/// (Kimi Code 2.3.0 `/usage` endpoint, 2026-10-23).
fn live_usages_sample() -> Value {
    json!({
        "limits": [
            {
                "window": {
                    "duration": 300,
                    "timeUnit": "TIME_UNIT_MINUTE"
                },
                "detail": {
                    "limit": "100",
                    "used": "100",
                    "resetTime": "2026-10-23T17:53:26.748457Z"
                }
            }
        ],
        "usages": {
            "limit_5h": {
                "used_ratio": 1,
                "reset_time": "2026-10-23T17:53:26Z"
            },
            "limit_month_total": {
                "used_ratio": 0.9109,
                "reset_time": "2026-11-21T00:00:00Z"
            },
            "limit_month_code": {
                "used_ratio": 0.196,
                "reset_time": "2026-11-21T00:00:00Z"
            }
        }
    })
}

#[test]
fn parses_live_kimi_usages_payload_with_5h_and_monthly_breakdown() {
    let budget = parse_managed_usage_payload(&live_usages_sample()).expect("valid live payload");
    assert_eq!(budget.windows.len(), 3, "{:?}", budget.windows);

    let five_h = &budget.windows[0];
    assert_eq!(five_h.label, "5h limit");
    assert_eq!(five_h.family.as_deref(), Some("Kimi Code"));
    assert_eq!(five_h.used_percent, 100);
    assert_eq!(five_h.remaining_percent, 0);
    assert_eq!(five_h.window_minutes, Some(300));
    assert!(five_h.resets_at.is_some());

    // 0.9109 total - 0.196 code = 0.7149 -> kimi 71% · code 20%
    let monthly = &budget.windows[1];
    assert_eq!(monthly.label, "Monthly limit (kimi 71% · code 20%)");
    assert_eq!(monthly.used_percent, 91);
    assert_eq!(monthly.remaining_percent, 9);
    assert_eq!(monthly.window_minutes, Some(30 * 24 * 60));
    assert!(monthly.resets_at.is_some());

    let monthly_code = &budget.windows[2];
    assert_eq!(monthly_code.label, "Monthly code");
    assert_eq!(monthly_code.used_percent, 20);
    assert_eq!(monthly_code.remaining_percent, 80);
    assert_eq!(monthly_code.window_minutes, Some(30 * 24 * 60));
}

#[test]
fn parses_weekly_window_and_booster_wallet_with_monthly_limit() {
    let payload = json!({
        "usages": {
            "limit_5h": { "used_ratio": 0.25, "reset_time": "2026-10-23T20:00:00Z" },
            "limit_7d": { "used_ratio": 0.60, "reset_time": "2026-10-28T00:00:00Z" },
            "limit_month_total": { "used_ratio": 0.40, "reset_time": "2026-11-01T00:00:00Z" }
        },
        "boosterWallet": {
            "balance": {
                "type": "BOOSTER",
                "amount": 5_000_000_000_i64,
                "amountLeft": 3_500_000_000_i64
            },
            "monthlyChargeLimitEnabled": true,
            "monthlyChargeLimit": { "priceInCents": 2000, "currency": "USD" },
            "monthlyUsed": { "priceInCents": 1500, "currency": "USD" }
        }
    });

    let budget = parse_managed_usage_payload(&payload).expect("valid weekly + booster payload");
    assert_eq!(budget.windows.len(), 3);
    assert_eq!(budget.windows[0].label, "5h limit");
    assert_eq!(budget.windows[0].used_percent, 25);
    assert_eq!(budget.windows[0].remaining_percent, 75);
    assert_eq!(budget.windows[1].label, "Weekly limit");
    assert_eq!(budget.windows[1].used_percent, 60);
    assert_eq!(budget.windows[1].remaining_percent, 40);
    assert_eq!(budget.windows[1].window_minutes, Some(7 * 24 * 60));
    assert_eq!(budget.windows[2].label, "Monthly limit");

    assert_eq!(
        budget.balance.as_deref(),
        Some("Extra usage: $15.00 / $20.00 this month · Balance $35.00")
    );
    let info = budget.balance_info.expect("booster wallet balance info");
    assert_eq!(info.currency, "USD");
    assert_eq!(info.kind.as_deref(), Some("spend"));
    assert!((info.total - 20.0).abs() < 1e-9);
    assert!((info.spent.unwrap_or_default() - 15.0).abs() < 1e-9);
    assert!((info.gift.unwrap_or_default() - 35.0).abs() < 1e-9);
}

#[test]
fn falls_back_to_limits_array_when_usages_map_is_absent() {
    let payload = json!({
        "limits": [
            {
                "window": { "duration": 300, "timeUnit": "TIME_UNIT_MINUTE" },
                "detail": {
                    "limit": "100",
                    "used": "42",
                    "resetTime": "2026-10-23T17:53:26Z"
                }
            }
        ]
    });
    let budget = parse_managed_usage_payload(&payload).expect("limits fallback");
    assert_eq!(budget.windows.len(), 1);
    assert_eq!(budget.windows[0].label, "5h limit");
    assert_eq!(budget.windows[0].used_percent, 42);
    assert_eq!(budget.windows[0].remaining_percent, 58);
}

#[test]
fn parses_kimi_config_toml_and_token_storage_name() {
    let toml_sample = r#"
default_model = "kimi-code/kimi-for-coding"

[thinking]
enabled = true

[providers."managed:kimi-code"]
type = "kimi"
api_key = ""
base_url = "https://api.kimi.ai/coding/v1"

[providers."managed:kimi-code".oauth]
storage = "file"
key = "oauth/kimi-code-env-0e4f99c69cc27850"
oauth_host = "https://auth.kimi.ai"
"#;

    let cfg = parse_kimi_config_toml(toml_sample);
    assert_eq!(
        cfg.base_url.as_deref(),
        Some("https://api.kimi.ai/coding/v1")
    );
    assert_eq!(cfg.api_key, None);
    assert_eq!(
        cfg.oauth_key.as_deref(),
        Some("oauth/kimi-code-env-0e4f99c69cc27850")
    );
    assert_eq!(cfg.oauth_host.as_deref(), Some("https://auth.kimi.ai"));
    assert_eq!(
        resolve_token_storage_name(cfg.oauth_key.as_deref()).as_deref(),
        Some("kimi-code-env-0e4f99c69cc27850")
    );
    assert_eq!(
        resolve_token_storage_name(Some("oauth/kimi-code")).as_deref(),
        Some("kimi-code")
    );
    assert_eq!(
        resolve_token_storage_name(None).as_deref(),
        Some("kimi-code")
    );
    assert!(resolve_token_storage_name(Some("oauth/../escape")).is_none());
}

#[test]
fn rejects_empty_or_non_object_payload() {
    assert!(parse_managed_usage_payload(&json!({})).is_err());
    assert!(parse_managed_usage_payload(&json!("not-an-object")).is_err());
}
