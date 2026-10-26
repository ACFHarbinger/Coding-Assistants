//! Automatic spend recording for CLI-subprocess harnesses (C6 / #84).
//!
//! The in-process HTTP path (OpenRouter, Grok Bot, generic `llm.rs`)
//! already auto-records each completion through
//! `HubStore::record_provider_usage`. A CLI harness runs outside the
//! process, so its observable completion is the capture adapter polling
//! the provider's transcript: when `record_harness_capture` deduplicates
//! a newly captured assistant turn, that turn's measured tokens are
//! recorded through the same `ProviderUsage` path. Missing figures stay
//! absent (`record_provider_usage` is a no-op without a measurement), so
//! the scorecard reads `unknown`, never an invented zero.
//!
//! First slice (#84): Claude Code. Later harnesses reuse
//! [`HarnessTurnUsage`] and [`record_harness_turn_usage`].

use hub::{HubStore, ProviderUsage, SliceStamp};
use std::fs;
use std::path::Path;

/// One captured CLI-harness assistant turn: the reply text plus whatever
/// usage figures the provider's transcript reported. `None` means the
/// transcript did not report that figure.
#[derive(Debug, Default, Clone)]
pub struct HarnessTurnUsage {
    pub provider: &'static str,
    pub actor_agent: &'static str,
    pub text: String,
    pub model: Option<String>,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub cache_tokens: Option<u32>,
}

/// Records one harness turn's measured usage as a `provider.usage` audit
/// row. Call only for a turn `record_harness_capture` accepted as new —
/// the capture content-hash dedup is what keeps repeated transcript polls
/// from double-counting spend.
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
    let _ = store.record_provider_usage(&usage);
}

fn usage_field(usage: &serde_json::Value, key: &str) -> Option<u32> {
    usage
        .get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
}

/// Reads a Claude Code session transcript and extracts recent assistant
/// replies with the usage each reported. Mirrors the assistant-text
/// extraction rules of the capture adapter (text blocks only; thinking
/// and tool_use are not replies) so a captured turn and its usage row
/// always describe the same provider response.
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
        if text.trim().is_empty() {
            continue;
        }
        // Anthropic usage: prompt = input tokens, completion = output
        // tokens, cache = read + creation cache tokens (one honest bucket
        // per `ProviderUsage`; absent when the transcript reported none).
        let usage = message.get("usage");
        let cache_tokens = usage.and_then(|usage| {
            let read = usage_field(usage, "cache_read_input_tokens").unwrap_or(0);
            let creation = usage_field(usage, "cache_creation_input_tokens").unwrap_or(0);
            (read > 0 || creation > 0).then_some(read + creation)
        });
        turns.push(HarnessTurnUsage {
            provider: "claude",
            actor_agent: "claude",
            text,
            model: message
                .get("model")
                .and_then(|model| model.as_str())
                .map(str::to_string),
            prompt_tokens: usage.and_then(|usage| usage_field(usage, "input_tokens")),
            completion_tokens: usage.and_then(|usage| usage_field(usage, "output_tokens")),
            cache_tokens,
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
}
