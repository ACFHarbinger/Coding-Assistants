use super::*;
use std::io::Write;
use tempfile::tempdir;

fn write_transcript(root: &Path, name: &str, cwd: &str, session_id: &str, lines: &[&str]) {
    let dir = root.join("2026").join("10").join("27");
    fs::create_dir_all(&dir).unwrap();
    let mut file = fs::File::create(dir.join(name)).unwrap();
    writeln!(
        file,
        r#"{{"type":"session_meta","payload":{{"cwd":"{cwd}","session_id":"{session_id}"}}}}"#
    )
    .unwrap();
    for line in lines {
        writeln!(file, "{line}").unwrap();
    }
}

fn usage_prompts(store: &hub::HubStore) -> Vec<serde_json::Value> {
    store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .filter(|event| event.operation == "provider.usage")
        .map(|event| serde_json::from_str(&event.process_json).unwrap())
        .collect()
}

#[test]
fn measured_token_count_writes_one_row_and_repoll_writes_nothing() {
    let root = tempdir().unwrap();
    let store = hub::HubStore::open(tempdir().unwrap().path()).unwrap();
    let workspace = Path::new("/tmp/c6-codex-spend");
    write_transcript(
        root.path(),
        "rollout-measured.jsonl",
        "/tmp/c6-codex-spend",
        "disk-measured",
        &[
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"measured reply"}]}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","id":"evt-measured","info":{"total_token_usage":{"input_tokens":12,"output_tokens":34,"cached_input_tokens":5},"last_token_usage":{"input_tokens":1,"output_tokens":1}}}}"#,
        ],
    );
    let first = super::super::codex::capture_codex_session_from(
        root.path(),
        &store,
        workspace,
        Some("disk-measured"),
        Some("hub-measured"),
    )
    .unwrap();
    assert_eq!(first.captured.len(), 1);
    assert_eq!(first.captured[0].body, "measured reply");
    let rows = usage_prompts(&store);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["provider"], "codex");
    assert_eq!(rows[0]["actor_agent"], "codex");
    assert_eq!(rows[0]["prompt_tokens"], 1);
    assert_eq!(rows[0]["completion_tokens"], 1);
    assert!(rows[0].get("cache_tokens").is_none());
    assert_eq!(rows[0]["currency"], "unknown");
    assert_eq!(rows[0]["session_id"], "hub-measured");
    assert!(rows[0].get("cost").is_none());

    let second = super::super::codex::capture_codex_session_from(
        root.path(),
        &store,
        workspace,
        Some("disk-measured"),
        Some("hub-measured"),
    )
    .unwrap();
    assert!(second.captured.is_empty());
    assert_eq!(usage_prompts(&store).len(), 1);
}

#[test]
fn missing_token_object_writes_no_usage_row() {
    let root = tempdir().unwrap();
    let store = hub::HubStore::open(tempdir().unwrap().path()).unwrap();
    let workspace = Path::new("/tmp/c6-codex-missing");
    write_transcript(
        root.path(),
        "rollout-missing.jsonl",
        "/tmp/c6-codex-missing",
        "disk-missing",
        &[
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"no figures"}]}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","id":"evt-empty"}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","id":"evt-blank","info":{"total_token_usage":{}}}}"#,
        ],
    );
    let outcome = super::super::codex::capture_codex_session_from(
        root.path(),
        &store,
        workspace,
        None,
        Some("hub-missing"),
    )
    .unwrap();
    assert_eq!(outcome.captured.len(), 1);
    assert!(usage_prompts(&store).is_empty());
}

#[test]
fn absent_token_key_stays_absent_and_partial_counts_still_record() {
    let root = tempdir().unwrap();
    let store = hub::HubStore::open(tempdir().unwrap().path()).unwrap();
    let workspace = Path::new("/tmp/c6-codex-partial");
    write_transcript(
        root.path(),
        "rollout-partial.jsonl",
        "/tmp/c6-codex-partial",
        "disk-partial",
        &[
            r#"{"type":"event_msg","payload":{"type":"token_count","id":"evt-partial","info":{"total_token_usage":{"input_tokens":12}}}}"#,
        ],
    );
    super::super::codex::capture_codex_session_from(root.path(), &store, workspace, None, None)
        .unwrap();
    let rows = usage_prompts(&store);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["prompt_tokens"], 12);
    assert!(rows[0].get("completion_tokens").is_none());
    assert!(rows[0].get("cache_tokens").is_none());
}

#[test]
fn identical_reply_text_with_two_event_ids_records_twice() {
    let root = tempdir().unwrap();
    let store = hub::HubStore::open(tempdir().unwrap().path()).unwrap();
    let workspace = Path::new("/tmp/c6-codex-twice");
    write_transcript(
        root.path(),
        "rollout-twice.jsonl",
        "/tmp/c6-codex-twice",
        "disk-twice",
        &[
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"same reply"}]}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","id":"evt-a","info":{"total_token_usage":{"input_tokens":11,"output_tokens":1}}}}"#,
            r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"same reply"}]}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","id":"evt-b","info":{"total_token_usage":{"input_tokens":22,"output_tokens":1}}}}"#,
        ],
    );
    let outcome = super::super::codex::capture_codex_session_from(
        root.path(),
        &store,
        workspace,
        None,
        Some("hub-twice"),
    )
    .unwrap();
    assert_eq!(outcome.captured.len(), 1);
    assert_eq!(outcome.scanned, 2);
    let prompts: Vec<u64> = usage_prompts(&store)
        .into_iter()
        .map(|row| row["prompt_tokens"].as_u64().unwrap())
        .collect();
    assert_eq!(prompts, vec![11, 11]);
}

#[test]
fn tool_only_line_still_records_measured_usage() {
    let root = tempdir().unwrap();
    let store = hub::HubStore::open(tempdir().unwrap().path()).unwrap();
    let workspace = Path::new("/tmp/c6-codex-tool");
    write_transcript(
        root.path(),
        "rollout-tool.jsonl",
        "/tmp/c6-codex-tool",
        "disk-tool",
        &[
            r#"{"type":"response_item","payload":{"type":"function_call","name":"Read"}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","event_id":"tool-evt","info":{"total_token_usage":{"input_tokens":3,"output_tokens":4}}}}"#,
        ],
    );
    let outcome = super::super::codex::capture_codex_session_from(
        root.path(),
        &store,
        workspace,
        None,
        Some("hub-tool"),
    )
    .unwrap();
    assert!(outcome.captured.is_empty());
    let rows = usage_prompts(&store);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["prompt_tokens"], 3);
    assert_eq!(rows[0]["completion_tokens"], 4);
    assert_eq!(rows[0]["actor_agent"], "codex");
}

#[test]
fn usage_key_uses_session_plus_event_id_or_line_hash() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session-file.jsonl");
    fs::write(
        &path,
        concat!(
            r#"{"type":"session_meta","payload":{"session_id":"disk-key"}}"#,
            "\n",
            r#"{"type":"event_msg","payload":{"type":"token_count","id":42,"info":{"total_token_usage":{"output_tokens":2}}}}"#,
            "\n",
            r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":8}}}}"#,
            "\n",
        ),
    )
    .unwrap();
    let turns = codex_measured_turns(&path, 1);
    assert_eq!(turns.len(), 1, "tail of 1 keeps the last token line only");
    assert!(turns[0]
        .usage_key
        .as_deref()
        .unwrap()
        .starts_with("disk-key:line:"));
    assert_eq!(turns[0].prompt_tokens, Some(8));
    assert_eq!(turns[0].completion_tokens, None);

    let all = codex_measured_turns(&path, 500);
    assert_eq!(all[0].usage_key.as_deref(), Some("disk-key:42"));
    assert_eq!(all[1].usage_key, turns[0].usage_key);
    let again = codex_measured_turns(&path, 500);
    assert_eq!(all[1].usage_key, again[1].usage_key);
}

fn snapshot(timestamp: &str, total: u32) -> String {
    serde_json::json!({
        "timestamp": timestamp,
        "type": "event_msg",
        "payload": {"type": "token_count", "info": {
            "total_token_usage": {"input_tokens": total, "output_tokens": total},
            "last_token_usage": {"input_tokens": 11, "output_tokens": 11},
        }},
    })
    .to_string()
}

#[test]
fn repeated_snapshots_with_different_timestamps_record_no_additional_spend() {
    let root = tempdir().unwrap();
    let home = tempdir().unwrap();
    let store = hub::HubStore::open(home.path()).unwrap();
    let workspace = Path::new("/tmp/c6-codex-snapshots");
    let lines = [
        snapshot("2026-10-26T10:00:00Z", 11),
        snapshot("2026-10-26T10:00:01Z", 11),
        // A genuinely new turn with identical last usage must still count.
        snapshot("2026-10-26T10:00:02Z", 22),
        snapshot("2026-10-26T10:00:03Z", 22),
        // Cumulative counters can reset; preserve that first new measurement.
        snapshot("2026-10-26T10:00:04Z", 11),
    ];
    let refs: Vec<_> = lines.iter().map(String::as_str).collect();
    write_transcript(
        root.path(),
        "rollout-snapshots.jsonl",
        workspace.to_str().unwrap(),
        "snapshots",
        &refs,
    );
    for _ in 0..2 {
        super::super::codex::capture_codex_session_from(
            root.path(),
            &store,
            workspace,
            Some("snapshots"),
            Some("hub-snapshots"),
        )
        .unwrap();
        let rows = usage_prompts(&store);
        assert_eq!(
            rows.len(),
            3,
            "duplicate token snapshots and repolls must not bill again"
        );
        assert!(rows
            .iter()
            .all(|row| row["prompt_tokens"] == 11 && row["completion_tokens"] == 11));
    }
}

#[test]
fn snapshot_before_the_tail_still_suppresses_a_duplicate_inside_it() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("snapshots.jsonl");
    fs::write(
        &path,
        format!("{}\n{}\n", snapshot("first", 11), snapshot("repeat", 11)),
    )
    .unwrap();
    assert!(codex_measured_turns(&path, 1).is_empty());
    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(format!("{}\n", snapshot("new-turn", 22)).as_bytes())
        .unwrap();
    let turns = codex_measured_turns(&path, 1);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].prompt_tokens, Some(11));
}
