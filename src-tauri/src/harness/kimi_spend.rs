//! Kimi Code spend extraction for C6 / #84.
//!
//! Chat text stays in [`super::kimi`]. A `usage.record` line is spend and
//! does not depend on text dedup. `usageScope` `turn` counts, and so does a
//! missing scope. Any other scope is skipped so it cannot double-count.
//! `inputOther` is prompt, `output` is completion, and `inputCacheRead` +
//! `inputCacheCreation` is one cache bucket. A missing key stays absent.
//! Cost is not estimated.

use super::spend::HarnessTurnUsage;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) fn kimi_measured_turns(
    path: &Path,
    tail_lines: usize,
    capture_session_id: Option<&str>,
) -> Vec<HarnessTurnUsage> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = raw.lines().collect();
    let session = usage_session(path, capture_session_id);
    let start = lines.len().saturating_sub(tail_lines);
    let mut turns = Vec::new();
    for line in &lines[start..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if let Some(turn) = usage_turn(&value, line, &session) {
            turns.push(turn);
        }
    }
    turns
}

fn usage_session(path: &Path, capture_session_id: Option<&str>) -> String {
    capture_session_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .or_else(|| session_directory_name(path))
        .unwrap_or_else(|| "unknown".to_string())
}

/// `agents/main/wire.jsonl` sits under the session directory. A wire file
/// placed directly in that directory uses its parent name instead.
fn session_directory_name(path: &Path) -> Option<String> {
    let parent = path.parent()?;
    let session_dir = if parent.file_name().and_then(|name| name.to_str()) == Some("main") {
        let agents = parent.parent()?;
        if agents.file_name().and_then(|name| name.to_str()) == Some("agents") {
            agents.parent()?
        } else {
            agents
        }
    } else {
        parent
    };
    session_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn counts_as_turn(line: &Value) -> bool {
    match line.get("usageScope") {
        None => true,
        Some(scope) => scope.as_str() == Some("turn"),
    }
}

fn usage_turn(line: &Value, raw: &str, session: &str) -> Option<HarnessTurnUsage> {
    if line.get("type").and_then(Value::as_str) != Some("usage.record") {
        return None;
    }
    if !counts_as_turn(line) {
        return None;
    }
    let usage = line.get("usage").filter(|value| value.is_object())?;
    let prompt_tokens = usage.get("inputOther").and_then(token_u32);
    let completion_tokens = usage.get("output").and_then(token_u32);
    let cache_tokens = cache_tokens(usage);
    if prompt_tokens.is_none() && completion_tokens.is_none() && cache_tokens.is_none() {
        return None;
    }
    Some(HarnessTurnUsage {
        provider: "kimi",
        actor_agent: "kimi",
        text: String::new(),
        model: model_name(line),
        prompt_tokens,
        completion_tokens,
        cache_tokens,
        usage_key: Some(usage_key(line, raw, session)),
    })
}

fn model_name(line: &Value) -> Option<String> {
    line.get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
}

/// A present non-negative integer that fits in `u32`, including 0.
/// Negative, non-integer, and overflowing values are absent, not zero.
fn token_u32(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|amount| u32::try_from(amount).ok())
}

fn cache_tokens(usage: &Value) -> Option<u32> {
    let mut sum: Option<u32> = None;
    for key in ["inputCacheRead", "inputCacheCreation"] {
        let Some(amount) = usage.get(key).and_then(token_u32) else {
            continue;
        };
        sum = Some(match sum {
            Some(current) => current.checked_add(amount)?,
            None => amount,
        });
    }
    sum
}

fn usage_key(line: &Value, raw: &str, session: &str) -> String {
    let line_hash = stable_line_id(raw);
    match line.get("time").and_then(json_number) {
        Some(time) => format!("{session}:{time}:{line_hash}"),
        None => format!("{session}:{line_hash}"),
    }
}

fn json_number(value: &Value) -> Option<String> {
    if let Some(number) = value.as_u64() {
        return Some(number.to_string());
    }
    if let Some(number) = value.as_i64() {
        return Some(number.to_string());
    }
    let number = value.as_f64()?;
    number.is_finite().then(|| number.to_string())
}

fn stable_line_id(raw: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("line:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const LIVE: &str = r#"{"type":"usage.record","agentId":"main","model":"kimi-code/kimi-for-coding","usage":{"inputOther":1849,"output":21,"inputCacheRead":18944,"inputCacheCreation":0},"usageScope":"turn","time":1789127115599}"#;
    const TEXT: &str = r#"{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"Captured text from Kimi"}},"time":200}"#;

    struct Fixture {
        _root: tempfile::TempDir,
        store: hub::HubStore,
        sessions: PathBuf,
        workspace: PathBuf,
    }

    fn fixture() -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let store = hub::HubStore::open(root.path().join("hub")).unwrap();
        let sessions = root.path().join("sessions");
        let workspace = root.path().join("workspace");
        Fixture {
            _root: root,
            store,
            sessions,
            workspace,
        }
    }

    fn write_session(sessions: &Path, session: &str, body: &str) {
        let session_dir = sessions.join("wd_test_123").join(session);
        let wire_dir = session_dir.join("agents").join("main");
        fs::create_dir_all(&wire_dir).unwrap();
        fs::write(
            session_dir.join("state.json"),
            format!(r#"{{"id":"{session}","cwd":"/tmp/ws"}}"#),
        )
        .unwrap();
        fs::write(wire_dir.join("wire.jsonl"), body).unwrap();
    }

    fn capture(
        fix: &Fixture,
        session: &str,
        hub_session: Option<&str>,
    ) -> super::super::kimi::KimiCaptureOutcome {
        super::super::kimi::capture_kimi_session_from(
            &fix.sessions,
            &fix.store,
            &fix.workspace,
            Some(session),
            hub_session,
        )
        .unwrap()
    }

    fn usage_rows(store: &hub::HubStore) -> Vec<serde_json::Value> {
        store
            .list_audit_events(false)
            .unwrap()
            .into_iter()
            .filter(|event| event.operation == "provider.usage")
            .map(|event| serde_json::from_str(&event.process_json).unwrap())
            .collect()
    }

    #[test]
    fn turn_record_writes_tokens_and_model_and_repoll_does_not() {
        let fix = fixture();
        let session = "session_live";
        write_session(&fix.sessions, session, &format!("{TEXT}\n{LIVE}\n"));
        let first = capture(&fix, session, Some("hub-kimi"));
        assert_eq!(first.captured.len(), 1);
        assert_eq!(first.captured[0].body, "Captured text from Kimi");
        let rows = usage_rows(&fix.store);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["provider"], "kimi");
        assert_eq!(rows[0]["actor_agent"], "kimi");
        assert_eq!(rows[0]["model"], "kimi-code/kimi-for-coding");
        assert_eq!(rows[0]["prompt_tokens"], 1849);
        assert_eq!(rows[0]["completion_tokens"], 21);
        assert_eq!(rows[0]["cache_tokens"], 18944);
        assert_eq!(rows[0]["currency"], "unknown");
        assert_eq!(rows[0]["session_id"], "hub-kimi");
        assert!(rows[0].get("cost").is_none());

        let second = capture(&fix, session, Some("hub-kimi"));
        assert!(second.captured.is_empty());
        assert_eq!(usage_rows(&fix.store).len(), 1);
    }

    #[test]
    fn a_later_time_adds_a_second_row() {
        let fix = fixture();
        let session = "session_twice";
        write_session(&fix.sessions, session, &format!("{LIVE}\n"));
        capture(&fix, session, None);
        let later = r#"{"type":"usage.record","usage":{"inputOther":3,"output":4,"inputCacheRead":1,"inputCacheCreation":1},"usageScope":"turn","time":1789127115600}"#;
        write_session(&fix.sessions, session, &format!("{LIVE}\n{later}\n"));
        capture(&fix, session, None);
        let prompts: Vec<u64> = usage_rows(&fix.store)
            .into_iter()
            .map(|row| row["prompt_tokens"].as_u64().unwrap())
            .collect();
        assert_eq!(prompts, vec![1849, 3]);
    }

    #[test]
    fn same_time_with_different_bytes_records_twice() {
        let fix = fixture();
        let session = "session_same_time";
        let first = r#"{"type":"usage.record","usage":{"output":1},"usageScope":"turn","time":10}"#;
        let second =
            r#"{"type":"usage.record","usage":{"output":2},"usageScope":"turn","time":10}"#;
        write_session(&fix.sessions, session, &format!("{first}\n{second}\n"));
        capture(&fix, session, None);
        let completions: Vec<u64> = usage_rows(&fix.store)
            .into_iter()
            .map(|row| row["completion_tokens"].as_u64().unwrap())
            .collect();
        assert_eq!(completions, vec![1, 2]);
    }

    #[test]
    fn foreign_scope_non_usage_and_empty_usage_write_nothing() {
        let fix = fixture();
        let session = "session_skip";
        let session_scope = r#"{"type":"usage.record","usage":{"inputOther":9,"output":8},"usageScope":"session","time":1}"#;
        let no_usage = r#"{"type":"usage.record","time":2}"#;
        let empty_usage = r#"{"type":"usage.record","usage":{},"usageScope":"turn","time":3}"#;
        write_session(
            &fix.sessions,
            session,
            &format!("{session_scope}\n{TEXT}\n{no_usage}\n{empty_usage}\n"),
        );
        let outcome = capture(&fix, session, None);
        assert_eq!(outcome.captured.len(), 1);
        assert!(usage_rows(&fix.store).is_empty());
    }

    #[test]
    fn missing_scope_still_records() {
        let fix = fixture();
        let session = "session_noscope";
        let line = r#"{"type":"usage.record","model":"kimi-code/kimi-for-coding","usage":{"inputOther":4,"output":5,"inputCacheCreation":6},"time":99}"#;
        write_session(&fix.sessions, session, &format!("{line}\n"));
        capture(&fix, session, None);
        let rows = usage_rows(&fix.store);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["prompt_tokens"], 4);
        assert_eq!(rows[0]["completion_tokens"], 5);
        assert_eq!(rows[0]["cache_tokens"], 6);
        assert_eq!(rows[0]["model"], "kimi-code/kimi-for-coding");
    }

    #[test]
    fn missing_token_key_stays_absent_and_tool_only_still_records() {
        let fix = fixture();
        let session = "session_partial";
        let tool = r#"{"type":"tool.call","name":"Read"}"#;
        let partial = r#"{"type":"usage.record","usage":{"output":7,"inputCacheRead":3},"usageScope":"turn","time":5}"#;
        let zero =
            r#"{"type":"usage.record","usage":{"inputOther":0},"usageScope":"turn","time":6}"#;
        write_session(
            &fix.sessions,
            session,
            &format!("{tool}\n{partial}\n{zero}\n"),
        );
        let outcome = capture(&fix, session, None);
        assert!(outcome.captured.is_empty());
        assert_eq!(outcome.scanned, 0);
        let rows = usage_rows(&fix.store);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].get("prompt_tokens").is_none());
        assert_eq!(rows[0]["completion_tokens"], 7);
        assert_eq!(rows[0]["cache_tokens"], 3);
        assert!(rows[0].get("cost").is_none());
        assert_eq!(rows[1]["prompt_tokens"], 0);
        assert!(rows[1].get("completion_tokens").is_none());
        assert!(rows[1].get("cache_tokens").is_none());
    }

    #[test]
    fn invalid_counters_leave_that_field_absent() {
        let fix = fixture();
        let session = "session_invalid";
        let bad = r#"{"type":"usage.record","usage":{"inputOther":-1,"output":1.5,"inputCacheRead":4294967296,"inputCacheCreation":2},"usageScope":"turn","time":1}"#;
        let overflow = r#"{"type":"usage.record","usage":{"inputCacheRead":4294967295,"inputCacheCreation":1,"output":3},"usageScope":"turn","time":2}"#;
        write_session(&fix.sessions, session, &format!("{bad}\n{overflow}\n"));
        capture(&fix, session, None);
        let rows = usage_rows(&fix.store);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].get("prompt_tokens").is_none());
        assert!(rows[0].get("completion_tokens").is_none());
        assert_eq!(rows[0]["cache_tokens"], 2);
        assert!(rows[1].get("cache_tokens").is_none());
        assert_eq!(rows[1]["completion_tokens"], 3);
    }

    #[test]
    fn usage_key_joins_session_time_and_raw_line_hash() {
        let dir = tempfile::tempdir().unwrap();
        let raw = r#"{"type":"usage.record","usage":{"output":1},"usageScope":"turn","time":10}"#;
        let path = dir
            .path()
            .join("session_from_dir")
            .join("agents")
            .join("main")
            .join("wire.jsonl");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("{raw}\n")).unwrap();

        let turns = kimi_measured_turns(&path, 2000, Some("capture-sess"));
        let expected = format!("capture-sess:10:{}", stable_line_id(raw));
        assert_eq!(turns[0].usage_key.as_deref(), Some(expected.as_str()));

        let no_time = r#"{"type":"usage.record","usage":{"output":2},"usageScope":"turn"}"#;
        fs::write(&path, format!("{no_time}\n")).unwrap();
        let turns = kimi_measured_turns(&path, 2000, None);
        let expected = format!("session_from_dir:{}", stable_line_id(no_time));
        assert_eq!(turns[0].usage_key.as_deref(), Some(expected.as_str()));

        let turns = kimi_measured_turns(&path, 2000, Some("  "));
        assert!(turns[0]
            .usage_key
            .as_deref()
            .unwrap()
            .starts_with("session_from_dir:"));

        let older = r#"{"type":"usage.record","usage":{"output":9},"usageScope":"turn","time":1}"#;
        fs::write(&path, format!("{older}\n{no_time}\n")).unwrap();
        let turns = kimi_measured_turns(&path, 1, Some("s"));
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].completion_tokens, Some(2));
    }
}
