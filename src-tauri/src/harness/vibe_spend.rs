//! Vibe spend extraction for C6 / #84.
//!
//! `meta.json` `stats.session_*_tokens` are session-cumulative counters.
//! Each poll records the increase over the previous absolute snapshot for
//! that session. `context_tokens`, `last_turn_*`, steps, and tool-call
//! counts are gauges and are not recorded. A missing key stays `None`.
//! The spawn gate on harness id `vibe` is unchanged by this slice.

use hub::{HubStore, ProviderUsage, SliceStamp};
use std::fs;
use std::path::Path;

/// Records one Vibe session's cumulative token snapshot, if `meta.json`
/// beside `messages_path` reports any `session_*` counter.
///
/// Missing meta, a missing `stats` object, or no measured counter writes
/// nothing and does not fail chat capture. The usage key stores the new
/// absolute (`{session}:abs:{prompt}:{completion}:{cache}`); the audit
/// row stores the delta, or the first absolute when nothing was stored.
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
    let key = absolute_key(&session_id, absolute);
    let Ok(previous_key) = store.latest_absolute_spend_key(&session_id) else {
        return;
    };
    if previous_key.as_deref() == Some(key.as_str()) {
        return;
    }
    let delta = match previous_key
        .as_deref()
        .and_then(|prior| parse_absolute(&session_id, prior))
    {
        Some(previous) => absolute.increase_over(previous),
        None => absolute,
    };
    if !delta.any() {
        return;
    }
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
    let _ = store.record_provider_usage_once(&usage, &key);
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

fn absolute_key(session_id: &str, counts: TokenCounts) -> String {
    format!(
        "{session_id}:abs:{}:{}:{}",
        bucket(counts.prompt),
        bucket(counts.completion),
        bucket(counts.cache)
    )
}

fn parse_absolute(session_id: &str, key: &str) -> Option<TokenCounts> {
    let rest = key.strip_prefix(&format!("{session_id}:abs:"))?;
    let mut parts = rest.split(':');
    let prompt = parse_bucket(parts.next()?)?;
    let completion = parse_bucket(parts.next()?)?;
    let cache = parse_bucket(parts.next()?)?;
    parts.next().is_none().then_some(TokenCounts {
        prompt,
        completion,
        cache,
    })
}

fn parse_bucket(part: &str) -> Option<Option<u32>> {
    if part == "na" {
        return Some(None);
    }
    part.parse::<u32>().ok().map(Some)
}

#[cfg(test)]
mod tests {
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
            Some("sess-sample:abs:4967:40:2048")
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
            Some("sess-delta:abs:5100:90:2048")
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
            Some("sess-a:abs:10:1:na")
        );
        assert_eq!(
            store
                .latest_absolute_spend_key("capture-b")
                .unwrap()
                .as_deref(),
            Some("capture-b:abs:100:4:na")
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
}
