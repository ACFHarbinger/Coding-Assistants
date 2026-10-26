//! Automatic spend recording for CLI-subprocess harnesses (C6 / #84).
//!
//! The in-process HTTP path (OpenRouter, Grok Bot, generic `llm.rs`)
//! already auto-records each completion through
//! `HubStore::record_provider_usage`. A CLI harness runs outside the
//! process, so its observable completion is the capture adapter polling
//! the provider's transcript. Measured tokens are recorded once per
//! transcript session and provider message identity, including responses
//! that contain only tool or thinking blocks. Chat capture still dedups
//! identical reply text; that dedup does not drop a distinct billable
//! response. Missing figures stay absent (`record_provider_usage` is a
//! no-op without a measurement), so the scorecard reads `unknown`, never
//! an invented zero.
//!
//! First slice (#84): Claude Code. Later harnesses reuse
//! [`HarnessTurnUsage`] and [`record_harness_turn_usage`].

use hub::{HubStore, ProviderUsage, SliceStamp};
use std::fs;
use std::path::Path;

/// One captured CLI-harness assistant turn: the reply text plus whatever
/// usage figures the provider's transcript reported. `None` means the
/// transcript did not report that figure. `usage_key` is the durable
/// spend identity (`<transcript session>:<provider message id>`).
#[derive(Debug, Default, Clone)]
pub struct HarnessTurnUsage {
    pub provider: &'static str,
    pub actor_agent: &'static str,
    pub text: String,
    pub model: Option<String>,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub cache_tokens: Option<u32>,
    pub usage_key: Option<String>,
}

/// Records one harness turn's measured usage as a `provider.usage` audit
/// row. When `usage_key` is set, a later poll of the same provider message
/// is a no-op even if chat capture rejected the reply text as a duplicate.
pub(crate) fn record_harness_turn_usage(
    store: &HubStore,
    hub_session_id: Option<&str>,
    turn: &HarnessTurnUsage,
) {
    let usage = ProviderUsage {
        provider: turn.provider.to_string(),
        actor_agent: turn.actor_agent.to_string(),
        prompt_tokens: turn.prompt_tokens,
        completion_tokens: turn.completion_tokens,
        cache_tokens: turn.cache_tokens,
        // Claude Code transcripts carry token counts, not currency;
        // the field stays absent rather than guessed.
        cost: None,
        via_gateway: None,
        model: turn.model.clone(),
        stamp: SliceStamp::registered(
            None,
            hub_session_id.map(str::to_string),
            Some(turn.actor_agent.to_string()),
        ),
    };
    let recorded = match turn.usage_key.as_deref() {
        Some(key) if !key.trim().is_empty() => store.record_provider_usage_once(&usage, key),
        _ => store.record_provider_usage(&usage),
    };
    let _ = recorded;
}

fn stable_line_id(raw: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("line:{hash:016x}")
}

fn provider_message_id(line: &serde_json::Value, raw: &str) -> String {
    let message_id = line
        .get("message")
        .and_then(|message| message.get("id"))
        .and_then(|id| id.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let entry_id = line
        .get("uuid")
        .and_then(|id| id.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty());
    match message_id.or(entry_id) {
        Some(id) => id.to_string(),
        None => stable_line_id(raw),
    }
}

fn transcript_session(path: &Path, line: &serde_json::Value) -> String {
    line.get("sessionId")
        .and_then(|id| id.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn usage_field(usage: &serde_json::Value, key: &str) -> Option<u32> {
    usage
        .get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
}

/// Reads a Claude Code session transcript and extracts recent assistant
/// responses with the usage each reported. Text blocks are the chat
/// reply. A response that is only `tool_use` or `thinking` is still
/// returned when it reports token usage, with empty reply text.
pub(crate) fn claude_recent_assistant_turns(
    path: &Path,
    tail_lines: usize,
) -> Vec<HarnessTurnUsage> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = raw.lines().collect();
    let start = lines.len().saturating_sub(tail_lines);
    let mut turns = Vec::new();
    for line in &lines[start..] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let Some(content) = message.get("content").and_then(|c| c.as_array()) else {
            continue;
        };
        let text: String = content
            .iter()
            .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n");
        // Anthropic usage: prompt = input tokens, completion = output
        // tokens, cache = read + creation cache tokens (one honest bucket
        // per `ProviderUsage`; absent when the transcript reported none).
        let usage = message.get("usage");
        let prompt_tokens = usage.and_then(|usage| usage_field(usage, "input_tokens"));
        let completion_tokens = usage.and_then(|usage| usage_field(usage, "output_tokens"));
        let cache_tokens = usage.and_then(|usage| {
            let read = usage_field(usage, "cache_read_input_tokens").unwrap_or(0);
            let creation = usage_field(usage, "cache_creation_input_tokens").unwrap_or(0);
            (read > 0 || creation > 0).then_some(read + creation)
        });
        let measured =
            prompt_tokens.is_some() || completion_tokens.is_some() || cache_tokens.is_some();
        if text.trim().is_empty() && !measured {
            continue;
        }
        let session = transcript_session(path, &value);
        let identity = provider_message_id(&value, line);
        turns.push(HarnessTurnUsage {
            provider: "claude",
            actor_agent: "claude",
            text,
            model: message
                .get("model")
                .and_then(|model| model.as_str())
                .map(str::to_string),
            prompt_tokens,
            completion_tokens,
            cache_tokens,
            usage_key: Some(format!("{session}:{identity}")),
        });
    }
    turns
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_transcript(path: &Path, lines: &[&str]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = fs::File::create(path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
    }

    #[test]
    fn extracts_text_and_measured_usage_from_an_assistant_line() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session-1.jsonl");
        write_transcript(
            &path,
            &[
                r#"{"type":"user","message":{"role":"user","content":"hi"}}"#,
                r#"{"type":"assistant","message":{"role":"assistant","model":"claude-sonnet-4","content":[{"type":"thinking","thinking":"pondering"},{"type":"text","text":"Here is the review."}],"usage":{"input_tokens":12,"output_tokens":34,"cache_read_input_tokens":500,"cache_creation_input_tokens":60}}}"#,
            ],
        );
        let turns = claude_recent_assistant_turns(&path, 500);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].text, "Here is the review.");
        assert_eq!(turns[0].provider, "claude");
        assert_eq!(turns[0].actor_agent, "claude");
        assert_eq!(turns[0].model.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(turns[0].prompt_tokens, Some(12));
        assert_eq!(turns[0].completion_tokens, Some(34));
        assert_eq!(turns[0].cache_tokens, Some(560));
    }

    #[test]
    fn records_a_usage_row_only_when_something_was_measured() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let measured = HarnessTurnUsage {
            provider: "claude",
            actor_agent: "claude",
            text: "reply".into(),
            model: Some("claude-sonnet-4".into()),
            prompt_tokens: Some(12),
            completion_tokens: Some(34),
            cache_tokens: Some(560),
            usage_key: None,
        };
        record_harness_turn_usage(&store, Some("sess-1"), &measured);
        let events = store.list_audit_events(false).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].operation, "provider.usage");
        let json: serde_json::Value = serde_json::from_str(&events[0].process_json).unwrap();
        assert_eq!(json["provider"], "claude");
        assert_eq!(json["actor_agent"], "claude");
        assert_eq!(json["prompt_tokens"], 12);
        assert_eq!(json["completion_tokens"], 34);
        assert_eq!(json["cache_tokens"], 560);
        assert_eq!(json["currency"], "unknown");
        assert_eq!(json["model"], "claude-sonnet-4");
        assert_eq!(json["session_id"], "sess-1");

        // A transcript line without usage figures invents no zero row.
        let unmeasured = HarnessTurnUsage {
            provider: "claude",
            actor_agent: "claude",
            text: "reply".into(),
            ..Default::default()
        };
        record_harness_turn_usage(&store, None, &unmeasured);
        assert_eq!(store.list_audit_events(false).unwrap().len(), 1);
    }

    #[test]
    fn tool_only_response_keeps_measured_usage_and_empty_reply_text() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tool-session.jsonl");
        write_transcript(
            &path,
            &[
                r#"{"type":"assistant","uuid":"tool-1","sessionId":"tool-session","message":{"id":"msg_tool","role":"assistant","model":"claude-sonnet-4","content":[{"type":"thinking","thinking":"look"},{"type":"tool_use","name":"Read","input":{}}],"usage":{"input_tokens":8,"output_tokens":9}}}"#,
            ],
        );
        let turns = claude_recent_assistant_turns(&path, 500);
        assert_eq!(turns.len(), 1);
        assert!(turns[0].text.trim().is_empty());
        assert_eq!(turns[0].prompt_tokens, Some(8));
        assert_eq!(turns[0].completion_tokens, Some(9));
        assert_eq!(turns[0].usage_key.as_deref(), Some("tool-session:msg_tool"));
    }

    #[test]
    fn identical_reply_text_records_each_provider_message() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("same-text.jsonl");
        let line = |id: &str, prompt: u32| {
            format!(
                r#"{{"type":"assistant","uuid":"{id}","sessionId":"disk-sess","message":{{"id":"{id}","role":"assistant","model":"claude-sonnet-4","content":[{{"type":"text","text":"same reply"}}],"usage":{{"input_tokens":{prompt},"output_tokens":1}}}}}}"#
            )
        };
        let first = line("msg_a", 11);
        let second = line("msg_b", 22);
        write_transcript(&path, &[&first, &second]);
        let turns = claude_recent_assistant_turns(&path, 500);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].text, turns[1].text);
        assert_ne!(turns[0].usage_key, turns[1].usage_key);

        let store = HubStore::open(dir.path().join("hub")).unwrap();
        record_harness_turn_usage(&store, Some("hub-sess"), &turns[0]);
        record_harness_turn_usage(&store, Some("hub-sess"), &turns[1]);
        record_harness_turn_usage(&store, Some("hub-sess"), &turns[0]);
        let prompts: Vec<u64> = store
            .list_audit_events(false)
            .unwrap()
            .into_iter()
            .filter(|event| event.operation == "provider.usage")
            .map(|event| {
                let json: serde_json::Value = serde_json::from_str(&event.process_json).unwrap();
                json["prompt_tokens"].as_u64().unwrap()
            })
            .collect();
        assert_eq!(prompts, vec![11, 22]);
    }

    #[test]
    fn capture_records_spend_for_duplicate_text_and_tool_only_responses() {
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let projects = tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        let session_dir = projects
            .path()
            .join(workspace.to_string_lossy().replace('/', "-"));
        fs::create_dir_all(&session_dir).unwrap();
        write_transcript(
            &session_dir.join("disk-1.jsonl"),
            &[
                r#"{"type":"assistant","message":{"id":"msg_a","role":"assistant","content":[{"type":"text","text":"same"}],"usage":{"input_tokens":1,"output_tokens":1}}}"#,
                r#"{"type":"assistant","message":{"id":"msg_b","role":"assistant","content":[{"type":"text","text":"same"}],"usage":{"input_tokens":2,"output_tokens":1}}}"#,
                r#"{"type":"assistant","message":{"id":"msg_c","role":"assistant","content":[{"type":"tool_use","name":"Read"}],"usage":{"input_tokens":3,"output_tokens":4}}}"#,
            ],
        );
        let outcome = super::super::claude::capture_claude_session_from(
            projects.path(),
            &store,
            workspace,
            Some("disk-1"),
            Some("hub-1"),
        )
        .unwrap();
        assert_eq!(outcome.captured.len(), 1);
        assert_eq!(outcome.captured[0].body, "same");
        let prompts: Vec<u64> = store
            .list_audit_events(false)
            .unwrap()
            .into_iter()
            .filter(|event| event.operation == "provider.usage")
            .map(|event| {
                let json: serde_json::Value = serde_json::from_str(&event.process_json).unwrap();
                json["prompt_tokens"].as_u64().unwrap()
            })
            .collect();
        assert_eq!(prompts, vec![1, 2, 3]);

        let again = super::super::claude::capture_claude_session_from(
            projects.path(),
            &store,
            workspace,
            Some("disk-1"),
            Some("hub-1"),
        )
        .unwrap();
        assert!(again.captured.is_empty());
        assert_eq!(
            store
                .list_audit_events(false)
                .unwrap()
                .iter()
                .filter(|event| event.operation == "provider.usage")
                .count(),
            3
        );
    }
}
