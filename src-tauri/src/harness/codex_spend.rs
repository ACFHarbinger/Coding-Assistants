//! Codex spend extraction for C6 / #84.
//!
//! Assistant chat text stays in [`super::codex`]. Token counts are a
//! sibling line: `type=event_msg`, `payload.type=token_count`.
//! `last_token_usage` is that turn's own figure and is recorded as-is,
//! unless its cumulative snapshot shows no new measured usage.
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
            // Token-count status snapshots may repeat the preceding turn's
            // last usage with a new timestamp/event identity. A cumulative
            // snapshot without an increase is not another billable turn.
            // Track this before the tail cutoff so its first line can repeat
            // a snapshot outside the capture window without charging again.
            if !recorded.any() {
                continue;
            }
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
#[path = "codex_spend_tests.rs"]
mod tests;
