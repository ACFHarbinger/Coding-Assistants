//! Codex spend extraction for C6 / #84.
//!
//! Assistant chat text stays in [`super::codex`]. Token counts are a
//! sibling line: `type=event_msg`, `payload.type=token_count`.
//! `last_token_usage` is that turn's own figure and is recorded as-is.
//! `total_token_usage` is session-cumulative, so a later line records
//! only the increase over the previous total in the same file. A missing
//! key stays `None`. A line with no token object, or a zero increase,
//! records no `provider.usage` row.

use super::spend::HarnessTurnUsage;
use std::fs;
use std::path::Path;

pub(crate) fn codex_measured_turns(path: &Path, tail_lines: usize) -> Vec<HarnessTurnUsage> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = raw.lines().collect();
    let session = transcript_session(&lines, path);
    let start = lines.len().saturating_sub(tail_lines);
    let mut previous_total: Option<TokenCounts> = None;
    let mut turns = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        let Some(parsed) = token_line(&value) else {
            continue;
        };
        let in_tail = index >= start;
        if let Some(total) = parsed.total {
            let recorded = match previous_total {
                Some(previous) => total.increase_over(previous),
                None => total.measured(),
            };
            previous_total = Some(total);
            if in_tail && parsed.last.is_none() {
                if let Some(turn) = usage_turn(&value, line, &session, recorded) {
                    turns.push(turn);
                }
            }
        }
        if in_tail {
            if let Some(last) = parsed.last {
                if let Some(turn) = usage_turn(&value, line, &session, last.measured()) {
                    turns.push(turn);
                }
            }
        }
    }
    turns
}

fn transcript_session(lines: &[&str], path: &Path) -> String {
    lines
        .iter()
        .take(16)
        .find_map(|line| {
            let value = serde_json::from_str::<serde_json::Value>(line.trim()).ok()?;
            if value.get("type").and_then(|kind| kind.as_str()) != Some("session_meta") {
                return None;
            }
            value
                .get("payload")
                .and_then(|payload| payload.get("session_id"))
                .and_then(|id| id.as_str())
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .filter(|stem| !stem.is_empty())
                .unwrap_or("unknown")
                .to_string()
        })
}

struct TokenLine {
    total: Option<TokenCounts>,
    last: Option<TokenCounts>,
}

#[derive(Clone, Copy)]
struct TokenCounts {
    prompt: Option<u32>,
    completion: Option<u32>,
    cache: Option<u32>,
}

impl TokenCounts {
    fn measured(self) -> Self {
        Self {
            prompt: nonzero(self.prompt),
            completion: nonzero(self.completion),
            cache: nonzero(self.cache),
        }
    }

    fn increase_over(self, previous: Self) -> Self {
        Self {
            prompt: increase(previous.prompt, self.prompt),
            completion: increase(previous.completion, self.completion),
            cache: increase(previous.cache, self.cache),
        }
    }

    fn any(self) -> bool {
        self.prompt.is_some() || self.completion.is_some() || self.cache.is_some()
    }
}

fn token_line(line: &serde_json::Value) -> Option<TokenLine> {
    if line.get("type").and_then(|kind| kind.as_str()) != Some("event_msg") {
        return None;
    }
    let payload = line.get("payload")?;
    if payload.get("type").and_then(|kind| kind.as_str()) != Some("token_count") {
        return None;
    }
    let info = payload.get("info")?;
    Some(TokenLine {
        total: counts(info.get("total_token_usage")),
        last: counts(info.get("last_token_usage")),
    })
}

fn counts(usage: Option<&serde_json::Value>) -> Option<TokenCounts> {
    let usage = usage.filter(|value| value.is_object())?;
    let counts = TokenCounts {
        prompt: usage_field(usage, "input_tokens"),
        completion: usage_field(usage, "output_tokens"),
        cache: usage_field(usage, "cached_input_tokens"),
    };
    counts.any().then_some(counts)
}

fn nonzero(value: Option<u32>) -> Option<u32> {
    value.filter(|value| *value > 0)
}

fn increase(previous: Option<u32>, next: Option<u32>) -> Option<u32> {
    let next = nonzero(next)?;
    match previous {
        Some(previous) if next > previous => Some(next - previous),
        Some(previous) if next < previous => Some(next),
        None => Some(next),
        Some(_) => None,
    }
}

fn usage_turn(
    line: &serde_json::Value,
    raw: &str,
    session: &str,
    counts: TokenCounts,
) -> Option<HarnessTurnUsage> {
    if !counts.any() {
        return None;
    }
    Some(HarnessTurnUsage {
        provider: "codex",
        actor_agent: "codex",
        text: String::new(),
        model: None,
        prompt_tokens: counts.prompt,
        completion_tokens: counts.completion,
        cache_tokens: counts.cache,
        usage_key: Some(format!("{session}:{}", event_identity(line, raw))),
    })
}

fn usage_field(usage: &serde_json::Value, key: &str) -> Option<u32> {
    usage
        .get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
}

fn event_identity(line: &serde_json::Value, raw: &str) -> String {
    let payload = line.get("payload");
    json_id(payload.and_then(|payload| payload.get("id")))
        .or_else(|| json_id(payload.and_then(|payload| payload.get("event_id"))))
        .or_else(|| json_id(line.get("id")))
        .or_else(|| json_id(line.get("event_id")))
        .unwrap_or_else(|| stable_line_id(raw))
}

fn json_id(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str().map(str::trim).filter(|id| !id.is_empty()) {
        return Some(text.to_string());
    }
    value.as_u64().map(|id| id.to_string())
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
}
