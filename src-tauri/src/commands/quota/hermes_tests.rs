use super::*;

/// Verbatim live report shape (`hermes 0.21.2`, 2026-09-12).
fn live_report() -> Value {
    serde_json::json!({
        "estimated_cost_usd": 0.0,
        "cost_status": "estimated",
        "cost_source": "provider_models_api",
        "input_tokens": 12356,
        "output_tokens": 17,
        "cache_read_tokens": 0,
        "cache_write_tokens": 0,
        "reasoning_tokens": 13,
        "total_tokens": 12373,
        "api_calls": 1,
        "model": "upstage/solar-pro4:free",
        "provider": "nous",
        "session_id": "20260912_232548_916eae",
        "completed": true,
        "failed": false,
        "service_tier": null
    })
}

#[test]
fn accumulates_live_report_shape() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(&mut total, &live_report());
    assert_eq!(total.sessions, 1);
    assert_eq!(total.prompt_tokens, 12356);
    assert_eq!(total.completion_tokens, 30);
    assert_eq!(total.cached_tokens, 0);
}

#[test]
fn failed_runs_still_count_spent_tokens() {
    let mut failed = live_report();
    failed["completed"] = serde_json::json!(false);
    failed["failed"] = serde_json::json!(true);
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(&mut total, &failed);
    assert_eq!(total.sessions, 1);
    assert_eq!(total.prompt_tokens, 12356);
}

#[test]
fn negative_and_missing_counters_read_as_zero() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(
        &mut total,
        &serde_json::json!({"input_tokens": -5, "output_tokens": "many"}),
    );
    assert_eq!(total.sessions, 1);
    assert_eq!(total.prompt_tokens, 0);
    assert_eq!(total.completion_tokens, 0);
}

#[test]
fn reads_a_usage_dir_newest_first_with_since_floor() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.json"), live_report().to_string()).unwrap();
    // A corrupt file is skipped, not fatal.
    std::fs::write(dir.path().join("b.json"), "{not json").unwrap();
    std::fs::write(
        dir.path().join("c.json"),
        serde_json::json!({"input_tokens": 100, "output_tokens": 5}).to_string(),
    )
    .unwrap();

    let usage = local_usage_from(dir.path()).expect("two parseable reports");
    assert_eq!(usage.sessions, 2);
    assert_eq!(usage.prompt_tokens, 12456);
    assert_eq!(usage.completion_tokens, 35);
    assert!(usage.since.is_some_and(|s| s > 0));
}

#[test]
fn missing_or_empty_dir_reads_as_no_usage() {
    let dir = tempfile::tempdir().unwrap();
    assert!(local_usage_from(&dir.path().join("absent")).is_none());
    assert!(local_usage_from(dir.path()).is_none());
}

#[test]
fn quota_wrapper_reports_local_only() {
    let quota = hermes_quota_from_local(None);
    assert_eq!(quota.agent_id, "hermes");
    assert_eq!(quota.status, "unavailable");

    let quota = hermes_quota_from_local(local_usage_from(&std::path::PathBuf::from(
        "/definitely/not/here",
    )));
    assert_eq!(quota.status, "unavailable");
}
