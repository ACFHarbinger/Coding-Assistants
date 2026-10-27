use super::*;
use serde_json::json;
use tempfile::tempdir;

const SAMPLE_STATS: &str = r#"{
    "steps": 2,
    "session_prompt_tokens": 4967,
    "session_completion_tokens": 40,
    "session_cached_tokens": 2048,
    "context_tokens": 5007,
    "tool_calls_agreed": 0,
    "tool_calls_rejected": 0,
    "tool_calls_hook_denied": 0,
    "tool_calls_failed": 0,
    "tool_calls_succeeded": 0,
    "last_turn_prompt_tokens": 4967,
    "last_turn_completion_tokens": 40
}"#;

fn write_meta(dir: &Path, body: &serde_json::Value) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("meta.json"), body.to_string()).unwrap();
}

fn record(store: &HubStore, dir: &Path, capture: Option<&str>, hub: Option<&str>) {
    record_vibe_session_spend(store, &dir.join("messages.jsonl"), capture, hub);
}

fn usage_rows(store: &HubStore) -> Vec<serde_json::Value> {
    store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .filter(|event| event.operation == "provider.usage")
        .map(|event| serde_json::from_str(&event.process_json).unwrap())
        .collect()
}

fn open_store() -> (tempfile::TempDir, HubStore) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    (dir, store)
}

#[test]
fn documented_sample_records_session_totals_and_repoll_writes_nothing() {
    let (_store_dir, store) = open_store();
    let dir = tempdir().unwrap();
    let stats: serde_json::Value = serde_json::from_str(SAMPLE_STATS).unwrap();
    write_meta(
        dir.path(),
        &json!({"session_id": "sess-sample", "stats": stats}),
    );
    record(&store, dir.path(), Some("capture-ignored"), Some("hub-1"));
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["provider"], "mistral");
    assert_eq!(rows[0]["actor_agent"], "mistral");
    assert_eq!(rows[0]["prompt_tokens"], 4967);
    assert_eq!(rows[0]["completion_tokens"], 40);
    assert_eq!(rows[0]["cache_tokens"], 2048);
    assert_eq!(rows[0]["currency"], "unknown");
    assert_eq!(rows[0]["session_id"], "hub-1");
    assert!(rows[0].get("cost").is_none());
    assert!(rows[0].get("model").is_none());
    assert!(rows[0].get("context_tokens").is_none());
    assert!(rows[0].get("steps").is_none());
    assert_eq!(
        store
            .latest_absolute_spend_key("sess-sample")
            .unwrap()
            .as_deref(),
        Some("sess-sample:abs:1:4967:40:2048")
    );
    record(&store, dir.path(), Some("capture-ignored"), Some("hub-1"));
    assert_eq!(usage_rows(&store).len(), 1);
}

#[test]
fn later_snapshot_records_only_the_increase() {
    let (_store_dir, store) = open_store();
    let dir = tempdir().unwrap();
    let stats: serde_json::Value = serde_json::from_str(SAMPLE_STATS).unwrap();
    write_meta(
        dir.path(),
        &json!({"session_id": "sess-delta", "stats": stats}),
    );
    record(&store, dir.path(), None, Some("hub-delta"));
    write_meta(
        dir.path(),
        &json!({
            "session_id": "sess-delta",
            "stats": {
                "steps": 4,
                "session_prompt_tokens": 5100,
                "session_completion_tokens": 90,
                "session_cached_tokens": 2048,
                "context_tokens": 9000,
                "last_turn_prompt_tokens": 200,
                "last_turn_completion_tokens": 15
            }
        }),
    );
    record(&store, dir.path(), None, Some("hub-delta"));
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["prompt_tokens"], 4967);
    assert_eq!(rows[0]["cache_tokens"], 2048);
    assert_eq!(rows[1]["prompt_tokens"], 133);
    assert_eq!(rows[1]["completion_tokens"], 50);
    assert!(rows[1].get("cache_tokens").is_none());
    assert!(rows[1].get("context_tokens").is_none());
    assert_eq!(
        store
            .latest_absolute_spend_key("sess-delta")
            .unwrap()
            .as_deref(),
        Some("sess-delta:abs:2:5100:90:2048")
    );
}

#[test]
fn dropped_prompt_counter_records_the_new_value() {
    let (_store_dir, store) = open_store();
    let dir = tempdir().unwrap();
    write_meta(
        dir.path(),
        &json!({
            "session_id": "sess-reset",
            "stats": {
                "session_prompt_tokens": 4967,
                "session_completion_tokens": 40,
                "session_cached_tokens": 2048
            }
        }),
    );
    record(&store, dir.path(), None, None);
    write_meta(
        dir.path(),
        &json!({
            "session_id": "sess-reset",
            "stats": {
                "session_prompt_tokens": 1000,
                "session_completion_tokens": 40,
                "session_cached_tokens": 2048
            }
        }),
    );
    record(&store, dir.path(), None, None);
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["prompt_tokens"], 1000);
    assert!(rows[1].get("completion_tokens").is_none());
    assert!(rows[1].get("cache_tokens").is_none());
    assert_ne!(rows[1]["prompt_tokens"], 5967);
}

#[test]
fn missing_prompt_stays_absent_and_no_stats_writes_nothing() {
    let (_store_dir, store) = open_store();
    let dir = tempdir().unwrap();
    write_meta(dir.path(), &json!({"session_id": "sess-partial"}));
    record(&store, dir.path(), Some("sess-partial"), None);
    assert!(usage_rows(&store).is_empty());
    write_meta(
        dir.path(),
        &json!({
            "session_id": "sess-partial",
            "stats": {
                "steps": 1,
                "context_tokens": 99,
                "session_completion_tokens": 12,
                "last_turn_prompt_tokens": 4
            }
        }),
    );
    record(&store, dir.path(), Some("sess-partial"), None);
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].get("prompt_tokens").is_none());
    assert_eq!(rows[0]["completion_tokens"], 12);
    assert!(rows[0].get("cache_tokens").is_none());
    assert!(rows[0].get("context_tokens").is_none());
}

#[test]
fn second_session_is_not_updated_by_the_first() {
    let (_store_dir, store) = open_store();
    let first = tempdir().unwrap();
    let second = tempdir().unwrap();
    write_meta(
        first.path(),
        &json!({
            "session_id": "sess-a",
            "stats": {"session_prompt_tokens": 10, "session_completion_tokens": 1}
        }),
    );
    write_meta(
        second.path(),
        &json!({
            "stats": {"session_prompt_tokens": 100, "session_completion_tokens": 4}
        }),
    );
    record(&store, first.path(), Some("capture-a"), Some("hub-a"));
    record(&store, second.path(), Some("capture-b"), Some("hub-b"));
    record(&store, first.path(), Some("capture-a"), Some("hub-a"));
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["prompt_tokens"], 10);
    assert_eq!(rows[0]["session_id"], "hub-a");
    assert_eq!(rows[1]["prompt_tokens"], 100);
    assert_eq!(rows[1]["completion_tokens"], 4);
    assert_eq!(rows[1]["session_id"], "hub-b");
    assert_eq!(
        store
            .latest_absolute_spend_key("sess-a")
            .unwrap()
            .as_deref(),
        Some("sess-a:abs:1:10:1:na")
    );
    assert_eq!(
        store
            .latest_absolute_spend_key("capture-b")
            .unwrap()
            .as_deref(),
        Some("capture-b:abs:1:100:4:na")
    );
}

#[test]
fn explicit_zero_records_and_invalid_numbers_stay_absent() {
    let (_store_dir, store) = open_store();
    let dir = tempdir().unwrap();
    write_meta(
        dir.path(),
        &json!({
            "session_id": "sess-zero",
            "stats": {
                "session_prompt_tokens": 0,
                "session_completion_tokens": -3,
                "session_cached_tokens": 4294967296u64
            }
        }),
    );
    record(&store, dir.path(), None, None);
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["prompt_tokens"], 0);
    assert!(rows[0].get("completion_tokens").is_none());
    assert!(rows[0].get("cache_tokens").is_none());
    write_meta(
        dir.path(),
        &json!({
            "session_id": "sess-zero",
            "stats": {
                "session_prompt_tokens": "12",
                "session_completion_tokens": 1.5,
                "session_cached_tokens": 2
            }
        }),
    );
    record(&store, dir.path(), None, None);
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 2);
    assert!(rows[1].get("prompt_tokens").is_none());
    assert!(rows[1].get("completion_tokens").is_none());
    assert_eq!(rows[1]["cache_tokens"], 2);
    assert_eq!(
        store
            .latest_absolute_spend_key("sess-zero")
            .unwrap()
            .as_deref(),
        Some("sess-zero:abs:2:0:na:2")
    );
}

#[test]
fn capture_records_meta_spend_and_missing_meta_writes_none() {
    let root = tempdir().unwrap();
    let (_store_dir, store) = open_store();
    let workspace = Path::new("/tmp/c6-vibe-spend-hook");
    let session_dir = root.path().join("session_20260909_120000_abc");
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        session_dir.join("messages.jsonl"),
        "{\"role\":\"assistant\",\"content\":\"hello from vibe\"}\n",
    )
    .unwrap();
    let stats: serde_json::Value = serde_json::from_str(SAMPLE_STATS).unwrap();
    write_meta(
        &session_dir,
        &json!({"session_id": "sess-hook", "stats": stats}),
    );
    let first = super::super::vibe::capture_vibe_session_from(
        root.path(),
        &store,
        workspace,
        Some("sess-hook"),
        Some("hub-hook"),
    )
    .unwrap();
    assert_eq!(first.captured.len(), 1);
    assert_eq!(first.captured[0].body, "hello from vibe");
    assert_eq!(usage_rows(&store).len(), 1);
    assert_eq!(usage_rows(&store)[0]["prompt_tokens"], 4967);
    let second = super::super::vibe::capture_vibe_session_from(
        root.path(),
        &store,
        workspace,
        Some("sess-hook"),
        Some("hub-hook"),
    )
    .unwrap();
    assert!(second.captured.is_empty());
    assert_eq!(usage_rows(&store).len(), 1);

    let bare_name = "session_20260909_130000_bare";
    let bare = root.path().join(bare_name);
    fs::create_dir_all(&bare).unwrap();
    fs::write(
        bare.join("messages.jsonl"),
        "{\"role\":\"assistant\",\"content\":\"no meta yet\"}\n",
    )
    .unwrap();
    let bare_capture = super::super::vibe::capture_vibe_session_from(
        root.path(),
        &store,
        workspace,
        Some(bare_name),
        Some("hub-bare"),
    )
    .unwrap();
    assert_eq!(bare_capture.captured.len(), 1);
    assert_eq!(bare_capture.captured[0].body, "no meta yet");
    assert_eq!(usage_rows(&store).len(), 1);
}

#[test]
fn review_counter_reset_can_revisit_a_previously_recorded_absolute() {
    let (_home, store) = open_store();
    let dir = tempdir().unwrap();
    for value in [10, 20, 10, 20] {
        write_meta(
            dir.path(),
            &json!({"session_id": "review-reset", "stats": {"session_prompt_tokens": value}}),
        );
        record(&store, dir.path(), None, None);
    }
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows.iter()
            .map(|row| row["prompt_tokens"].as_u64().unwrap())
            .sum::<u64>(),
        40
    );
    assert_eq!(
        store
            .latest_absolute_spend_key("review-reset")
            .unwrap()
            .as_deref(),
        Some("review-reset:abs:4:20:na:na")
    );
}

#[test]
fn review_partial_snapshots_retain_last_known_buckets() {
    let (_home, store) = open_store();
    let dir = tempdir().unwrap();
    for stats in [
        json!({"session_prompt_tokens": 10, "session_completion_tokens": 1}),
        json!({"session_completion_tokens": 2}),
        json!({"session_prompt_tokens": 10, "session_completion_tokens": 3}),
    ] {
        write_meta(
            dir.path(),
            &json!({"session_id": "review-partial", "stats": stats}),
        );
        record(&store, dir.path(), None, None);
    }
    let rows = usage_rows(&store);
    assert_eq!(rows.len(), 3);
    assert!(rows[2].get("prompt_tokens").is_none());
    assert_eq!(rows[2]["completion_tokens"], 1);
    assert_eq!(
        store
            .latest_absolute_spend_key("review-partial")
            .unwrap()
            .as_deref(),
        Some("review-partial:abs:3:10:3:na")
    );
}
