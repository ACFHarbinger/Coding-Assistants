use super::*;
use serde_json::json;

/// Verbatim shape of a real `meta.json` written by `vibe 2.25.1`
/// (2026-09-09), trimmed to the fields this reader consumes.
fn live_meta(prompt: i64, completion: i64, cached: i64, start: &str) -> Value {
    json!({
        "session_id": "6d2776e5-fd91-a416-68df-ea585bacb872",
        "start_time": start,
        "end_time": "2026-09-09T17:04:48.577003+00:00",
        "environment": { "working_directory": "/tmp/scratch" },
        "total_messages": 2,
        "stats": {
            "steps": 2,
            "session_prompt_tokens": prompt,
            "session_completion_tokens": completion,
            "session_cached_tokens": cached,
            "context_tokens": 5007,
            "tool_calls_agreed": 0,
            "tool_calls_rejected": 1,
            "tool_calls_hook_denied": 0,
            "tool_calls_failed": 2,
            "tool_calls_succeeded": 3,
            "last_turn_prompt_tokens": prompt,
            "last_turn_completion_tokens": completion
        }
    })
}

#[test]
fn sums_session_counters_across_sessions() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(
        &mut total,
        &live_meta(4967, 40, 2048, "2026-09-09T17:04:46.154077+00:00"),
    );
    accumulate(
        &mut total,
        &live_meta(1000, 10, 512, "2026-09-08T09:00:00.000000+00:00"),
    );

    assert_eq!(total.sessions, 2);
    assert_eq!(total.prompt_tokens, 5967);
    assert_eq!(total.completion_tokens, 50);
    assert_eq!(total.cached_tokens, 2560);
    assert_eq!(total.tool_calls_succeeded, 6);
    assert_eq!(total.tool_calls_failed, 4);
    assert_eq!(total.tool_calls_rejected, 2);
}

#[test]
fn since_tracks_the_oldest_session_regardless_of_scan_order() {
    let mut total = ProviderQuotaLocalUsage::default();
    // Newest first, the order the directory sweep actually yields.
    accumulate(
        &mut total,
        &live_meta(1, 1, 0, "2026-09-09T17:04:46.154077+00:00"),
    );
    accumulate(
        &mut total,
        &live_meta(1, 1, 0, "2026-09-08T09:00:00.000000+00:00"),
    );
    let oldest = chrono::DateTime::parse_from_rfc3339("2026-09-08T09:00:00.000000+00:00")
        .unwrap()
        .timestamp();
    assert_eq!(total.since, Some(oldest));
}

#[test]
fn point_in_time_gauges_are_never_summed() {
    // `context_tokens` and `last_turn_*` describe the final turn, not a
    // counter; summing them across sessions would be meaningless.
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(
        &mut total,
        &live_meta(100, 5, 0, "2026-09-09T17:04:46.154077+00:00"),
    );
    accumulate(
        &mut total,
        &live_meta(100, 5, 0, "2026-09-09T18:04:46.154077+00:00"),
    );
    assert_eq!(total.prompt_tokens, 200, "session counters do accumulate");
    // 5007 appears twice in the fixtures but must not reach any total.
    let serialized = serde_json::to_string(&total).unwrap();
    assert!(!serialized.contains("5007"), "{serialized}");
    assert!(!serialized.contains("10014"), "{serialized}");
}

#[test]
fn a_crashed_session_counts_but_contributes_no_tokens() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(&mut total, &json!({ "session_id": "x" }));
    assert_eq!(total.sessions, 1);
    assert_eq!(total.prompt_tokens, 0);
    assert_eq!(total.since, None);
}

#[test]
fn negative_and_non_numeric_counters_are_ignored() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate(
        &mut total,
        &json!({ "stats": { "session_prompt_tokens": -5, "session_completion_tokens": "many" } }),
    );
    assert_eq!(total.sessions, 1);
    assert_eq!(total.prompt_tokens, 0);
    assert_eq!(total.completion_tokens, 0);
}

#[test]
fn a_missing_root_reads_as_no_usage_rather_than_zeroes() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("never-ran");
    assert!(local_usage_from(&missing).is_none());
}

#[test]
fn reads_real_session_directories_and_skips_malformed_ones() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let good = root.join("session_20260909_170446_6d2776e5");
    std::fs::create_dir_all(&good).unwrap();
    std::fs::write(
        good.join("meta.json"),
        serde_json::to_string(&live_meta(4967, 40, 2048, "2026-09-09T17:04:46.154077+00:00"))
            .unwrap(),
    )
    .unwrap();

    // Half-written meta.json: a session observed mid-flight is skipped, not fatal.
    let torn = root.join("session_20260909_180000_deadbeef");
    std::fs::create_dir_all(&torn).unwrap();
    std::fs::write(torn.join("meta.json"), "{\"stats\": {\"session_pro").unwrap();

    // A directory with no meta.json at all is not a session.
    std::fs::create_dir_all(root.join("active")).unwrap();

    let usage = local_usage_from(root).expect("one readable session");
    assert_eq!(usage.sessions, 1);
    assert_eq!(usage.prompt_tokens, 4967);
    assert_eq!(usage.completion_tokens, 40);
}

#[test]
fn vibe_home_env_overrides_the_default_root() {
    // Serialized against other env-mutating tests by the process-wide lock in
    // `commands::tests`; this one only reads its own variable back out.
    let previous = std::env::var("VIBE_HOME").ok();
    std::env::set_var("VIBE_HOME", "/tmp/ca-vibe-home-probe");
    let root = vibe_sessions_root();
    match previous {
        Some(value) => std::env::set_var("VIBE_HOME", value),
        None => std::env::remove_var("VIBE_HOME"),
    }
    assert_eq!(
        root,
        std::path::Path::new("/tmp/ca-vibe-home-probe/logs/session")
    );
}
