use super::*;
use serde_json::json;

/// Verbatim shape of a real `usage.record` line written by `kimi 0.42.0`
/// (2026-09-11), trimmed to the fields this reader consumes.
fn live_record(input_other: i64, output: i64, cache_read: i64, cache_creation: i64) -> Value {
    json!({
        "type": "usage.record",
        "agentId": "main",
        "model": "kimi-code/kimi-for-coding",
        "usage": {
            "inputOther": input_other,
            "output": output,
            "inputCacheRead": cache_read,
            "inputCacheCreation": cache_creation
        },
        "usageScope": "turn",
        "time": 1789127115599_i64
    })
}

#[test]
fn sums_turn_records_into_prompt_completion_and_cached() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate_line(&mut total, &live_record(1849, 21, 18944, 0));
    accumulate_line(&mut total, &live_record(100, 5, 0, 512));

    assert_eq!(total.prompt_tokens, 1949);
    assert_eq!(total.completion_tokens, 26);
    assert_eq!(total.cached_tokens, 19456);
}

#[test]
fn non_record_lines_and_foreign_scopes_contribute_nothing() {
    let mut total = ProviderQuotaLocalUsage::default();
    // Assistant text, think parts, step markers: not usage.
    accumulate_line(
        &mut total,
        &json!({"type": "context.append_loop_event",
                "event": {"type": "content.part",
                          "part": {"type": "text", "text": "Hello"}}}),
    );
    // A hypothetical non-turn scope must not double-count the same tokens.
    let mut other_scope = live_record(1000, 1000, 1000, 1000);
    other_scope["usageScope"] = json!("session");
    accumulate_line(&mut total, &other_scope);
    // A record with no usage object is malformed, not zero usage.
    accumulate_line(&mut total, &json!({"type": "usage.record"}));

    assert_eq!(total.prompt_tokens, 0);
    assert_eq!(total.completion_tokens, 0);
    assert_eq!(total.cached_tokens, 0);
    assert_eq!(total.sessions, 0, "line folding never counts sessions");
}

#[test]
fn negative_and_non_numeric_counters_are_ignored() {
    let mut total = ProviderQuotaLocalUsage::default();
    accumulate_line(
        &mut total,
        &json!({"type": "usage.record",
                "usage": {"inputOther": -5, "output": "many",
                          "inputCacheRead": 10.7, "inputCacheCreation": null}}),
    );
    assert_eq!(total.prompt_tokens, 0);
    assert_eq!(total.completion_tokens, 0);
    // Float counters round; null stays zero.
    assert_eq!(total.cached_tokens, 11);
}

/// Lays out `sessions/wd_<slug>/session_<id>/{state.json,agents/main/wire.jsonl}`
/// exactly as the CLI does, with `createdAt` in ms epoch.
fn write_session(root: &Path, wd: &str, session: &str, created_ms: i64, wire: &str) {
    let session_dir = root.join(wd).join(session);
    let wire_dir = session_dir.join("agents").join("main");
    std::fs::create_dir_all(&wire_dir).unwrap();
    std::fs::write(
        session_dir.join("state.json"),
        json!({"id": session, "createdAt": created_ms}).to_string(),
    )
    .unwrap();
    std::fs::write(wire_dir.join("wire.jsonl"), wire).unwrap();
}

#[test]
fn reads_real_session_layout_and_floors_since_on_oldest_created() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let record = live_record(1849, 21, 18944, 0).to_string();
    let wire = format!(
        "{{\"type\":\"permission.set_mode\",\"mode\":\"auto\"}}\n{record}\n{{\"type\":\"turn.ended\"}}\n"
    );
    write_session(
        root,
        "wd_proj_abc123",
        "session_11111111-1111-1111-1111-111111111111",
        1_789_127_258_325,
        &wire,
    );
    write_session(
        root,
        "wd_proj_abc123",
        "session_22222222-2222-2222-2222-222222222222",
        1_789_040_000_000,
        &format!("{}\n", live_record(100, 5, 0, 512)),
    );

    // A torn trailing line from a session in flight is skipped, not fatal;
    // a session directory with no wire.jsonl at all is not a session.
    let torn_dir = root
        .join("wd_proj_abc123")
        .join("session_33333333-3333-3333-3333-333333333333")
        .join("agents")
        .join("main");
    std::fs::create_dir_all(&torn_dir).unwrap();
    std::fs::write(torn_dir.join("wire.jsonl"), "{\"type\": \"usage.rec").unwrap();
    std::fs::create_dir_all(root.join("wd_empty")).unwrap();

    let usage = local_usage_from(root).expect("two readable sessions");
    assert_eq!(usage.sessions, 3, "readable logs count, even torn ones");
    assert_eq!(usage.prompt_tokens, 1949);
    assert_eq!(usage.completion_tokens, 26);
    assert_eq!(usage.cached_tokens, 19456);
    assert_eq!(usage.since, Some(1_789_040_000), "oldest createdAt wins");
}

#[test]
fn near_max_counters_saturate_instead_of_panicking_or_wrapping() {
    let mut total = ProviderQuotaLocalUsage::default();
    // Two records each carrying a near-u64::MAX cache value: a naive `+=`
    // either panics (debug overflow checks) or silently wraps to a tiny
    // number (release). Saturating addition must clamp at u64::MAX instead.
    let huge = u64::MAX - 1;
    accumulate_line(
        &mut total,
        &json!({"type": "usage.record",
                "usage": {"inputOther": huge, "output": huge,
                          "inputCacheRead": huge, "inputCacheCreation": 0}}),
    );
    accumulate_line(
        &mut total,
        &json!({"type": "usage.record",
                "usage": {"inputOther": huge, "output": huge,
                          "inputCacheRead": huge, "inputCacheCreation": huge}}),
    );
    assert_eq!(total.prompt_tokens, u64::MAX);
    assert_eq!(total.completion_tokens, u64::MAX);
    assert_eq!(total.cached_tokens, u64::MAX);
}

#[test]
fn session_cap_is_enforced_during_traversal_not_after_collecting_everything() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // One more session than the cap, each with a distinct mtime via a
    // distinct createdAt (session_wires sorts by file mtime, not createdAt,
    // but writing them in order gives ascending mtimes on any filesystem).
    let extra = 5;
    for i in 0..(MAX_SESSIONS_SCANNED + extra) {
        write_session(
            root,
            "wd_bulk",
            &format!("session_{i:06}"),
            1_789_000_000_000 + i as i64,
            &live_record(1, 1, 1, 1).to_string(),
        );
        // Force a distinct, monotonically increasing mtime independent of
        // filesystem timestamp resolution (std-only, no extra dependency).
        let wire_path = root
            .join("wd_bulk")
            .join(format!("session_{i:06}"))
            .join("agents")
            .join("main")
            .join("wire.jsonl");
        let when = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(i as u64);
        std::fs::File::open(&wire_path)
            .unwrap()
            .set_modified(when)
            .unwrap();
    }

    let wires = session_wires(root);
    assert_eq!(
        wires.len(),
        MAX_SESSIONS_SCANNED,
        "the cap holds even though {} sessions exist on disk",
        MAX_SESSIONS_SCANNED + extra
    );
    // Newest-first: the last-written session (highest mtime) comes first,
    // and the oldest `extra` sessions were evicted during traversal.
    assert!(wires[0]
        .to_string_lossy()
        .contains(&format!("session_{:06}", MAX_SESSIONS_SCANNED + extra - 1)));
    for i in 0..extra {
        assert!(
            !wires
                .iter()
                .any(|p| p.to_string_lossy().contains(&format!("session_{i:06}/"))),
            "session_{i:06} is older than the cap and must have been evicted"
        );
    }
}

#[test]
fn a_missing_root_reads_as_no_usage_rather_than_zeroes() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("never-ran");
    assert!(local_usage_from(&missing).is_none());
}

#[test]
fn kimi_home_env_overrides_the_default_root() {
    let previous = std::env::var("KIMI_CODE_HOME").ok();
    std::env::set_var("KIMI_CODE_HOME", "/tmp/ca-kimi-home-probe");
    let root = kimi_sessions_root();
    match previous {
        Some(value) => std::env::set_var("KIMI_CODE_HOME", value),
        None => std::env::remove_var("KIMI_CODE_HOME"),
    }
    assert_eq!(
        root,
        std::path::Path::new("/tmp/ca-kimi-home-probe/sessions")
    );
}
