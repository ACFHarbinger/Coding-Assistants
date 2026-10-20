use super::*;
use serde_json::json;

// NOTE (2026-09-09): no Mistral Backoffice admin key exists in this
// environment, so — unlike the DeepSeek/Muse adapters — these fixtures are
// *contract-shaped*, built from the assigned S3 shape (`GET /usage`: spend by
// category incl. `vibe_usage` + `currency`/`start_date`/`end_date`;
// `GET /spend-limit`: `{amount, no_monthly_limit}`), not from a live capture.
// A verbatim live response stays open on #306 for the owner to record.

fn usage_map() -> Value {
    json!({
        "currency": "USD",
        "start_date": "2026-09-01",
        "end_date": "2026-09-30",
        "usage": {
            "vibe_usage": 12.5,
            "api_usage": 3.0,
            "agents_usage": 0.75
        }
    })
}

fn usage_list() -> Value {
    json!({
        "currency": "EUR",
        "usage": [
            { "category": "vibe_usage", "spend": 7.25 },
            { "category": "api_usage", "spend": "2.75" },
            { "category": "broken", "spend": "not-a-number" },
            { "category": "negative", "spend": -1.0 }
        ]
    })
}

#[test]
fn map_form_usage_parses_total_vibe_and_window_dates() {
    let summary = summarize_usage(&usage_map()).expect("spend present");
    assert_eq!(summary.currency, "USD");
    assert!((summary.total_spend - 16.25).abs() < 1e-9);
    assert_eq!(summary.vibe_spend, Some(12.5));
    assert_eq!(summary.start_date.as_deref(), Some("2026-09-01"));
    assert_eq!(summary.end_date.as_deref(), Some("2026-09-30"));
}

#[test]
fn list_form_usage_parses_and_skips_bad_entries() {
    let summary = summarize_usage(&usage_list()).expect("spend present");
    assert_eq!(summary.currency, "EUR");
    assert!((summary.total_spend - 10.0).abs() < 1e-9);
    assert_eq!(summary.vibe_spend, Some(7.25));
    assert_eq!(summary.start_date, None);
}

#[test]
fn usage_without_recognizable_spend_is_none_not_zero() {
    for body in [
        json!({}),
        json!({ "currency": "USD" }),
        json!({ "usage": {} }),
        json!({ "usage": [] }),
        json!({ "usage": { "vibe_usage": "pending" } }),
        json!({ "usage": [{ "category": "vibe_usage" }] }),
    ] {
        assert!(
            summarize_usage(&body).is_none(),
            "should be drift, not zero: {body}"
        );
    }
}

#[test]
fn spend_limit_parses_capped_uncapped_and_absent() {
    let capped = parse_spend_limit(&json!({ "amount": 100.0, "no_monthly_limit": false }));
    assert_eq!(capped.amount, Some(100.0));
    assert!(!capped.no_monthly_limit);

    let uncapped = parse_spend_limit(&json!({ "amount": 100.0, "no_monthly_limit": true }));
    assert!(uncapped.no_monthly_limit);

    let absent = parse_spend_limit(&json!({}));
    assert_eq!(absent.amount, None);
    assert!(!absent.no_monthly_limit);

    let string_amount = parse_spend_limit(&json!({ "amount": "50.5" }));
    assert_eq!(string_amount.amount, Some(50.5));

    let negative = parse_spend_limit(&json!({ "amount": -5.0 }));
    assert_eq!(negative.amount, None);
}

#[test]
fn capped_budget_yields_a_monthly_spend_window() {
    let summary = summarize_usage(&usage_map()).unwrap();
    let budget = budget_from_summary(&summary, SpendLimitResult::Capped(100.0));
    assert_eq!(budget.windows.len(), 1);
    let window = &budget.windows[0];
    assert_eq!(window.label, "Monthly spend");
    assert_eq!(window.family.as_deref(), Some("Mistral Admin"));
    assert_eq!(window.used_percent, 16);
    assert_eq!(window.remaining_percent, 84);
    assert!(budget.detail.is_none());
    let balance = budget.balance.expect("spend line");
    assert!(balance.contains("$16.25 USD"), "{balance}");
    assert!(balance.contains("Vibe $12.50"), "{balance}");
    assert!(balance.contains("2026-09-01 → 2026-09-30"), "{balance}");
    let info = budget.balance_info.expect("currency figure");
    assert_eq!(info.currency, "USD");
    assert!((info.total - 16.25).abs() < 1e-9);
    assert_eq!(info.kind.as_deref(), Some("spend"));
    assert_eq!(info.paid, None);
}

#[test]
fn uncapped_limit_reports_spend_only_with_uncapped_note() {
    let summary = summarize_usage(&usage_map()).unwrap();
    let budget = budget_from_summary(&summary, SpendLimitResult::Uncapped);
    assert!(budget.windows.is_empty());
    assert!(budget
        .balance
        .as_deref()
        .unwrap_or_default()
        .contains("$16.25"));
    assert!(
        budget
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("No monthly spend cap is set"),
        "spend-only note missing: {:?}",
        budget.detail
    );
    let info = budget.balance_info.expect("currency figure");
    assert_eq!(info.kind.as_deref(), Some("spend"));
}

#[test]
fn failed_limit_request_reports_failure_reason_not_uncapped() {
    let summary = summarize_usage(&usage_map()).unwrap();
    let budget = budget_from_summary(
        &summary,
        SpendLimitResult::Failed("HTTP 500 Internal Server Error".into()),
    );
    assert!(budget.windows.is_empty());
    assert!(budget
        .balance
        .as_deref()
        .unwrap_or_default()
        .contains("$16.25"));
    let detail = budget.detail.expect("detail note");
    assert!(
        detail.contains("could not be read"),
        "unexpected detail: {detail}"
    );
    assert!(
        !detail.contains("No monthly spend cap is set"),
        "must not claim uncapped on failed request: {detail}"
    );
}

#[test]
fn window_percent_clamps_at_full_spend() {
    let summary = summarize_usage(&usage_map()).unwrap();
    let budget = budget_from_summary(&summary, SpendLimitResult::Capped(1.0));
    assert_eq!(budget.windows[0].used_percent, 100);
    assert_eq!(budget.windows[0].remaining_percent, 0);
}

#[test]
fn spend_line_degrades_without_vibe_slice_or_dates() {
    let summary = summarize_usage(&json!({
        "currency": "USD",
        "usage": { "api_usage": 4.0 }
    }))
    .unwrap();
    assert_eq!(summary.vibe_spend, None);
    let budget = budget_from_summary(&summary, SpendLimitResult::Uncapped);
    let balance = budget.balance.unwrap();
    assert_eq!(balance, "Spent $4.00 USD this period");
}

#[test]
fn json_amount_rejects_non_finite_and_negative() {
    for raw in ["", "NaN", "inf", "-1.00", "pending"] {
        assert!(
            json_amount(&Value::String(raw.into())).is_none(),
            "accepted {raw:?}"
        );
    }
    assert_eq!(json_amount(&json!(3)), Some(3.0));
    assert_eq!(json_amount(&json!(" 4.5 ")), Some(4.5));
}
