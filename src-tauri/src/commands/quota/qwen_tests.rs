use super::*;
use serde_json::json;

/// Verbatim 0.23.2 `usage_record.jsonl` line (2026-09-11), with non-zero
/// counters so the sums are observable. `api_key` is a synthetic canary.
fn live_record(prompt: i64, completion: i64, cached: i64, thoughts: i64, start_ms: i64) -> Value {
    json!({
        "version": 1,
        "sessionId": "a2602eab-4e09-487c-954d-3de3b269a2c0",
        "timestamp": start_ms + 3804,
        "startTime": start_ms,
        "project": "/tmp/scratch",
        "durationMs": 3804,
        "totalLatencyMs": 3257,
        "api_key": "canary-qwen-key-do-not-leak",
        "models": {
            "qwen3.5-plus": {
                "requests": 1,
                "inputTokens": prompt,
                "outputTokens": completion,
                "cachedTokens": cached,
                "thoughtsTokens": thoughts,
                "totalTokens": prompt + completion + thoughts,
                "totalLatencyMs": 3257
            }
        },
        "tools": {
            "totalCalls": 3,
            "totalSuccess": 2,
            "totalFail": 1,
            "byName": {}
        },
        "files": { "linesAdded": 4, "linesRemoved": 1 },
        "skills": { "totalCalls": 0, "totalSuccess": 0, "totalFail": 0, "byName": {} }
    })
}

#[test]
fn sums_model_and_tool_counters_across_sessions() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(&mut total, &live_record(12, 4, 2, 3, 1_788_978_936_079));
    accumulate(&mut total, &live_record(8, 1, 1, 0, 1_788_970_000_000));

    assert_eq!(total.sessions, 2);
    assert_eq!(total.prompt_tokens, 20);
    assert_eq!(total.completion_tokens, 8); // 4+3 thoughts, plus 1+0
    assert_eq!(total.cached_tokens, 3);
    assert_eq!(total.tool_calls_succeeded, 4);
    assert_eq!(total.tool_calls_failed, 2);
    assert_eq!(total.tool_calls_rejected, 0);
}

#[test]
fn since_tracks_the_oldest_start_in_unix_seconds() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(&mut total, &live_record(1, 1, 0, 0, 1_788_978_936_079));
    accumulate(&mut total, &live_record(1, 1, 0, 0, 1_788_970_000_000));
    assert_eq!(total.since, Some(1_788_970_000_000 / 1000));
}

#[test]
fn gauges_and_secrets_never_enter_the_totals() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(&mut total, &live_record(12, 4, 2, 3, 1_788_978_936_079));
    let serialized = serde_json::to_string(&total).unwrap();
    assert!(
        !serialized.contains("canary-qwen-key-do-not-leak"),
        "{serialized}"
    );
    assert!(!serialized.contains("3257"), "{serialized}");
    assert!(!serialized.contains("19"), "{serialized}"); // totalTokens
}

#[test]
fn a_session_without_models_counts_but_adds_no_tokens() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(&mut total, &json!({ "sessionId": "x" }));
    assert_eq!(total.sessions, 1);
    assert_eq!(total.prompt_tokens, 0);
    assert_eq!(total.since, None);
}

#[test]
fn negative_and_non_numeric_counters_are_ignored() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(
        &mut total,
        &json!({
            "models": { "qwen3.5-plus": { "inputTokens": -5, "outputTokens": "many" } },
            "tools": { "totalSuccess": -1, "totalFail": 2 }
        }),
    );
    assert_eq!(total.sessions, 1);
    assert_eq!(total.prompt_tokens, 0);
    assert_eq!(total.completion_tokens, 0);
    assert_eq!(total.tool_calls_failed, 2);
}

#[test]
fn a_missing_file_reads_as_no_usage_rather_than_zeroes() {
    let dir = tempfile::tempdir().unwrap();
    assert!(local_usage_from(&dir.path().join("never-ran.jsonl")).is_none());
}

#[test]
fn reads_jsonl_and_skips_malformed_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("usage_record.jsonl");
    let good = serde_json::to_string(&live_record(12, 4, 2, 3, 1_788_978_936_079)).unwrap();
    std::fs::write(&path, format!("{good}\n{{\"models\": {{\"qwen\n\n")).unwrap();
    let usage = local_usage_from(&path).expect("one readable session");
    assert_eq!(usage.sessions, 1);
    assert_eq!(usage.prompt_tokens, 12);
    assert_eq!(usage.completion_tokens, 7);
}

#[test]
fn qwen_home_env_overrides_the_default_root() {
    let previous = std::env::var("QWEN_HOME").ok();
    std::env::set_var("QWEN_HOME", "/tmp/ca-qwen-home-probe");
    let path = usage_record_path();
    match previous {
        Some(value) => std::env::set_var("QWEN_HOME", value),
        None => std::env::remove_var("QWEN_HOME"),
    }
    assert_eq!(
        path,
        std::path::Path::new("/tmp/ca-qwen-home-probe/usage_record.jsonl")
    );
}

#[test]
fn quota_is_ok_with_local_usage_and_unavailable_without() {
    let usage = {
        let mut total = ProviderQuotaLocalUsage::default();
        accumulate(&mut total, &live_record(12, 4, 2, 3, 1_788_978_936_079));
        total
    };
    let ok = qwen_quota_from_local(Some(usage));
    assert_eq!(ok.agent_id, "qwen");
    assert_eq!(ok.provider, "qwen");
    assert_eq!(ok.harness_title, "Qwen Code");
    assert_eq!(ok.status, "ok");
    assert!(ok.windows.is_empty());
    assert!(ok.balance.is_none());
    assert_eq!(ok.local_usage.as_ref().map(|u| u.sessions), Some(1));
    let detail = ok.detail.unwrap_or_default();
    assert!(detail.contains("Locally recorded"), "{detail}");
    assert!(detail.contains("console"), "{detail}");
    assert!(!detail.contains("canary-qwen-key-do-not-leak"), "{detail}");

    let missing = qwen_quota_from_local(None);
    assert_eq!(missing.status, "unavailable");
    let detail = missing.detail.unwrap_or_default();
    assert!(detail.contains("usage_record.jsonl"), "{detail}");
}
