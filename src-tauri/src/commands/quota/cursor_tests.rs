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
    assert_eq!(windows.len(), 3);
    // The three bars the interactive `/usage` panel shows.
    assert_eq!(windows[0].label, "Included allowance");
    assert_eq!(windows[0].used_percent, 14); // totalPercentUsed 14.39 -> 14
    assert_eq!(windows[0].remaining_percent, 86);
    assert_eq!(windows[0].resets_at, Some(1_791_200_599));
    assert_eq!(windows[1].label, "Auto / Composer");
    assert_eq!(windows[1].used_percent, 13);
    assert_eq!(windows[1].remaining_percent, 87);
    assert_eq!(windows[2].label, "API");
    assert_eq!(windows[2].used_percent, 33);
    // The payload has no trustworthy "dollars used" figure: `totalSpend` 7122 =
    // `includedSpend` 2000 + `bonusSpend` 5122 (notional value, not money owed)
    // and `includedSpend` saturates at `limit`. Any "$X used of $Y" built from
    // them is wrong (it read "$71.22 used of $20.00", ≈356%). Consumption is the
    // windows above; the balance line states only the allowance in dollars.
    assert_eq!(
        balance_from_period_usage(&value).as_deref(),
        Some("$20.00 included this cycle")
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

    // 5. A recognized metric changing to a non-numeric type is schema drift,
    // not a misleading zero-percent quota.
    let invalid_metric: Value = serde_json::json!({
        "planUsage": { "autoPercentUsed": { "unexpected": true } }
    });
    assert_eq!(
        check_usage_schema(&invalid_metric),
        Err(SchemaDriftReason::InvalidMetricType)
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

fn make_test_jwt(exp: Option<i64>) -> String {
    let header = URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\",\"typ\":\"JWT\"}");
    let payload_val = match exp {
        Some(e) => format!("{{\"sub\":\"user123\",\"exp\":{e}}}"),
        None => "{\"sub\":\"user123\"}".to_string(),
    };
    let payload = URL_SAFE_NO_PAD.encode(payload_val.as_bytes());
    format!("{header}.{payload}.sig")
}

#[test]
fn parse_jwt_expiry_reads_exp_claim_and_handles_malformed() {
    let now = 1_757_361_234;
    let jwt = make_test_jwt(Some(now));
    assert_eq!(parse_jwt_expiry(&jwt), Some(now));

    let no_exp = make_test_jwt(None);
    assert_eq!(parse_jwt_expiry(&no_exp), None);

    assert_eq!(parse_jwt_expiry("not-a-jwt"), None);
    assert_eq!(parse_jwt_expiry("part1.part2"), None);
    assert_eq!(parse_jwt_expiry("part1.invalid#base64.part3"), None);
    assert_eq!(parse_jwt_expiry("part1.bm90LWpzb24.part3"), None);
}

#[test]
fn parse_expiry_value_handles_seconds_millis_and_rfc3339() {
    let secs = serde_json::json!({ "exp": 1_757_361_234 });
    assert_eq!(parse_expiry_value(&secs), Some(1_757_361_234));

    let millis = serde_json::json!({ "expiresAt": 1_757_361_234_000i64 });
    assert_eq!(parse_expiry_value(&millis), Some(1_757_361_234));

    let str_num = serde_json::json!({ "expires_at": "1757361234" });
    assert_eq!(parse_expiry_value(&str_num), Some(1_757_361_234));

    let rfc = serde_json::json!({ "expiry": "2026-01-01T00:00:00Z" });
    assert_eq!(parse_expiry_value(&rfc), Some(1_767_225_600));

    let none = serde_json::json!({ "other": 123 });
    assert_eq!(parse_expiry_value(&none), None);
}

#[test]
fn cursor_auth_details_from_evaluates_precedence_and_expiry() {
    let now = 1_757_361_234;
    let unexpired_jwt = make_test_jwt(Some(now + 3600));
    let expired_jwt = make_test_jwt(Some(now - 3600));

    // 1. Vault token takes precedence over file
    let vault_auth = cursor_auth_details_from(
        Some(&unexpired_jwt),
        Some(r#"{"accessToken":"tok_other"}"#),
        now,
    );
    assert!(vault_auth.token_present);
    assert!(!vault_auth.is_expired);
    assert!(vault_auth.auth_expires_at.is_some());
    assert!(!vault_auth.file_present);
    assert_eq!(vault_auth.detail(true), "Cursor Agent is logged in");

    // 2. Expired vault token
    let expired_vault = cursor_auth_details_from(Some(&expired_jwt), None, now);
    assert!(expired_vault.token_present);
    assert!(expired_vault.is_expired);
    assert_eq!(
        expired_vault.detail(true),
        "Cursor Agent access token expired; run `agent login` to refresh login"
    );

    // 3. File token unexpired
    let file_raw = format!(r#"{{"accessToken":"{unexpired_jwt}"}}"#);
    let file_auth = cursor_auth_details_from(None, Some(&file_raw), now);
    assert!(file_auth.token_present);
    assert!(!file_auth.is_expired);
    assert!(file_auth.file_present);

    // 4. File token expired
    let expired_file_raw = format!(r#"{{"accessToken":"{expired_jwt}"}}"#);
    let expired_file_auth = cursor_auth_details_from(None, Some(&expired_file_raw), now);
    assert!(expired_file_auth.token_present);
    assert!(expired_file_auth.is_expired);
    assert!(expired_file_auth.file_present);

    // 5. File has top-level expiresAt
    let file_with_top_exp = format!(
        r#"{{"accessToken":"opaque_token","expiresAt":{}}}"#,
        (now + 600) * 1000
    );
    let top_exp_auth = cursor_auth_details_from(None, Some(&file_with_top_exp), now);
    assert!(top_exp_auth.token_present);
    assert!(!top_exp_auth.is_expired);
    assert!(top_exp_auth.auth_expires_at.is_some());

    // 6. File without token
    let empty_file_auth = cursor_auth_details_from(None, Some(r#"{"other":true}"#), now);
    assert!(!empty_file_auth.token_present);
    assert!(empty_file_auth.file_present);
    assert_eq!(
        empty_file_auth.detail(true),
        "Cursor Agent is installed but CLI auth file has no access token; run `agent login`"
    );

    // 7. Neither vault nor file
    let no_auth = cursor_auth_details_from(None, None, now);
    assert!(!no_auth.token_present);
    assert!(!no_auth.file_present);
    assert_eq!(
        no_auth.detail(true),
        "Cursor Agent is installed but not logged in; run `agent login`"
    );
    assert_eq!(
        no_auth.detail(false),
        "The Cursor `agent` CLI was not found on PATH"
    );
}
