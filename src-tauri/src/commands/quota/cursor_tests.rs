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
fn contract_check_validates_live_and_snake_case() {
    let live_val: Value = serde_json::from_str(LIVE_SAMPLE).unwrap();
    let summary = verify_usage_contract(&live_val).expect("live sample must satisfy contract");
    assert!(summary.has_billing_cycle);
    assert_eq!(summary.auto_percent, Some(13));
    assert_eq!(summary.api_percent, Some(33));
    assert_eq!(summary.total_percent, Some(14));
    assert_eq!(summary.total_spend_cents, Some(7122));
    assert_eq!(summary.limit_cents, Some(2000));

    let snake_val: Value = serde_json::from_str(
        r#"{ "billing_cycle_end": 1791200599,
             "plan_usage": { "included_spend": 500, "limit": 2000 } }"#,
    )
    .unwrap();
    let snake_summary =
        verify_usage_contract(&snake_val).expect("snake case sample must satisfy contract");
    assert!(snake_summary.has_billing_cycle);
    assert_eq!(snake_summary.limit_cents, Some(2000));
}

#[test]
fn contract_check_detects_schema_drift() {
    // 1. Not an object
    let array_val: Value = serde_json::json!([1, 2, 3]);
    assert_eq!(
        check_usage_schema(&array_val),
        Err(SchemaDriftReason::NotAnObject)
    );

    // 2. Missing planUsage
    let missing_plan: Value = serde_json::json!({
        "error": "unknown_rpc",
        "code": 404
    });
    assert_eq!(
        check_usage_schema(&missing_plan),
        Err(SchemaDriftReason::MissingPlanUsage)
    );

    // 3. planUsage is not an object
    let invalid_plan: Value = serde_json::json!({
        "planUsage": "unexpected_string_format"
    });
    assert_eq!(
        check_usage_schema(&invalid_plan),
        Err(SchemaDriftReason::PlanUsageNotAnObject)
    );

    // 4. Missing expected metrics in planUsage
    let empty_plan: Value = serde_json::json!({
        "planUsage": {
            "unrelatedMetadata": "xyz",
            "tier": "enterprise"
        }
    });
    assert_eq!(
        check_usage_schema(&empty_plan),
        Err(SchemaDriftReason::MissingExpectedFields)
    );
}

#[test]
fn drift_causes_safe_unavailable_degrade() {
    let drifted_val: Value = serde_json::json!({
        "error": "internal_error",
        "message": "service migrated"
    });
    let quota = cursor_quota_from_period(&drifted_val);
    assert_eq!(quota.status, "unavailable");
    assert!(quota.windows.is_empty());
    assert!(quota.balance.is_none());
    let detail = quota.detail.expect("must include drift detail");
    assert!(detail.contains("schema drift detected"));
}

#[test]
fn missing_auth_file_is_unavailable_without_panic() {
    let quota = unavailable(
        "Not logged in to Cursor Agent (no CLI auth file). Run `agent login`, or add CURSOR_TOKEN in Settings → Credentials.",
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
