//! Vibe spend extraction for C6 / #84.
//!
//! `meta.json` `stats.session_*_tokens` are session-cumulative counters.
//! Each poll records the increase over the previous absolute snapshot for
//! that session. A counter omitted from a later snapshot keeps its last
//! known absolute, so a reappearing bucket is not billed again. An explicit
//! lower number is a reset and records that new value. The usage key is
//! `{session}:abs:{generation}:{prompt}:{completion}:{cache}` so a later
//! epoch that revisits an earlier total is a new row. `context_tokens`,
//! `last_turn_*`, steps, and tool-call counts are gauges and are not
//! recorded. A missing key stays `None`. The spawn gate on harness id
//! `vibe` is unchanged by this slice.

use hub::{AbsoluteSpendCommit, HubStore, ProviderUsage, SliceStamp};
use std::fs;
use std::path::Path;

/// Records one Vibe session's cumulative token snapshot, if `meta.json`
/// beside `messages_path` reports any `session_*` counter.
///
/// Missing meta, a missing `stats` object, or no measured counter writes
/// nothing and does not fail chat capture. The usage key stores the filled
/// absolute (omitted buckets carried forward) and a generation. The audit
/// row stores the delta, or the first absolute when nothing was stored.
/// An unchanged filled snapshot writes no key and no row.
pub(crate) fn record_vibe_session_spend(
    store: &HubStore,
    messages_path: &Path,
    capture_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) {
    let Some(raw) = messages_path
        .parent()
        .and_then(|dir| fs::read_to_string(dir.join("meta.json")).ok())
    else {
        return;
    };
    let Ok(meta) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let Some(stats) = meta.get("stats").filter(|value| value.is_object()) else {
        return;
    };
    let absolute = TokenCounts {
        prompt: measured_u32(stats, "session_prompt_tokens"),
        completion: measured_u32(stats, "session_completion_tokens"),
        cache: measured_u32(stats, "session_cached_tokens"),
    };
    if !absolute.any() {
        return;
    }
    let Some(session_id) = spend_session_id(&meta, capture_session_id) else {
        return;
    };
    // Overlapping capture polls (the desktop refresh does not wait) must not
    // both bill from the same baseline. A stale commit means the other poll
    // won; recompute against the key it stored.
    for _ in 0..8 {
        let Ok(previous_key) = store.latest_absolute_spend_key(&session_id) else {
            return;
        };
        let previous = previous_key
            .as_deref()
            .and_then(|prior| parse_absolute(&session_id, prior));
        let filled = match previous {
            Some(prior) => absolute.fill_from(prior.counts),
            None => absolute,
        };
        let delta = match previous {
            Some(prior) => filled.increase_over(prior.counts),
            None => filled,
        };
        if !delta.any() {
            return;
        }
        let generation = previous
            .map(|prior| prior.generation.saturating_add(1))
            .unwrap_or(1);
        let key = absolute_key(&session_id, generation, filled);
        let usage = ProviderUsage {
            provider: "mistral".to_string(),
            actor_agent: "mistral".to_string(),
            prompt_tokens: delta.prompt,
            completion_tokens: delta.completion,
            cache_tokens: delta.cache,
            cost: None,
            via_gateway: None,
            model: None,
            stamp: SliceStamp::registered(
                None,
                hub_session_id.map(str::to_string),
                Some("mistral".to_string()),
            ),
        };
        match store.commit_absolute_spend(&session_id, previous_key.as_deref(), &usage, &key) {
            Ok(AbsoluteSpendCommit::Committed | AbsoluteSpendCommit::Duplicate) => return,
            Ok(AbsoluteSpendCommit::Stale) => continue,
            Err(_) => return,
        }
    }
}

fn spend_session_id(meta: &serde_json::Value, capture_session_id: Option<&str>) -> Option<String> {
    non_empty(meta.get("session_id").and_then(|value| value.as_str()))
        .or_else(|| non_empty(capture_session_id))
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn measured_u32(stats: &serde_json::Value, key: &str) -> Option<u32> {
    let whole = stats.get(key)?.as_u64()?;
    u32::try_from(whole).ok()
}

#[derive(Clone, Copy)]
struct TokenCounts {
    prompt: Option<u32>,
    completion: Option<u32>,
    cache: Option<u32>,
}

impl TokenCounts {
    fn any(self) -> bool {
        self.prompt.is_some() || self.completion.is_some() || self.cache.is_some()
    }

    /// A missing bucket keeps the last known absolute. An explicit number,
    /// including zero, replaces it.
    fn fill_from(self, previous: Self) -> Self {
        Self {
            prompt: self.prompt.or(previous.prompt),
            completion: self.completion.or(previous.completion),
            cache: self.cache.or(previous.cache),
        }
    }

    fn increase_over(self, previous: Self) -> Self {
        Self {
            prompt: increase(previous.prompt, self.prompt),
            completion: increase(previous.completion, self.completion),
            cache: increase(previous.cache, self.cache),
        }
    }
}

fn increase(previous: Option<u32>, next: Option<u32>) -> Option<u32> {
    let next = next?;
    match previous {
        Some(previous) if next > previous => Some(next - previous),
        Some(previous) if next < previous => Some(next),
        None => Some(next),
        Some(_) => None,
    }
}

fn bucket(value: Option<u32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "na".to_string(),
    }
}

fn absolute_key(session_id: &str, generation: u64, counts: TokenCounts) -> String {
    format!(
        "{session_id}:abs:{generation}:{}:{}:{}",
        bucket(counts.prompt),
        bucket(counts.completion),
        bucket(counts.cache)
    )
}

#[derive(Clone, Copy)]
struct ParsedAbsolute {
    generation: u64,
    counts: TokenCounts,
}

fn parse_absolute(session_id: &str, key: &str) -> Option<ParsedAbsolute> {
    let rest = key.strip_prefix(&format!("{session_id}:abs:"))?;
    let mut parts = rest.split(':');
    let generation = parts.next()?.parse::<u64>().ok()?;
    let prompt = parse_bucket(parts.next()?)?;
    let completion = parse_bucket(parts.next()?)?;
    let cache = parse_bucket(parts.next()?)?;
    parts.next().is_none().then_some(ParsedAbsolute {
        generation,
        counts: TokenCounts {
            prompt,
            completion,
            cache,
        },
    })
}

fn parse_bucket(part: &str) -> Option<Option<u32>> {
    if part == "na" {
        return Some(None);
    }
    part.parse::<u32>().ok().map(Some)
}

#[cfg(test)]
#[path = "vibe_spend_tests.rs"]
mod tests;
