//! Qwen Code end-of-session spend for C6 / #84.
//!
//! Chat transcripts carry no token counters. The free source is
//! `$QWEN_HOME/usage_record.jsonl` when that variable is set and
//! non-blank, otherwise `~/.qwen/usage_record.jsonl`. qwen 0.23.2 does
//! not read `QWEN_CODE_HOME`. One JSON object per line. A later line
//! with the same `sessionId` is a refreshed total, so only the increase
//! over that previous total is recorded. Other session ids are ignored.

use super::spend::HarnessTurnUsage;
use std::fs;
use std::path::{Path, PathBuf};

/// `$QWEN_HOME` is the global dir itself, matching qwen 0.23.2.
pub(crate) fn usage_record_path() -> PathBuf {
    match std::env::var("QWEN_HOME") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir.trim()).join("usage_record.jsonl"),
        _ => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".qwen").join("usage_record.jsonl")
        }
    }
}

pub(crate) fn record_qwen_session_usage(
    store: &hub::HubStore,
    path: &Path,
    session_id: &str,
    hub_session_id: Option<&str>,
) {
    for turn in session_turns(path, session_id) {
        super::spend::record_harness_turn_usage(store, hub_session_id, &turn);
    }
}

fn session_turns(path: &Path, session_id: &str) -> Vec<HarnessTurnUsage> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Vec::new();
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut previous: Option<TokenCounts> = None;
    let mut turns = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        if value
            .get("sessionId")
            .and_then(|id| id.as_str())
            .map(str::trim)
            != Some(session_id)
        {
            continue;
        }
        let Some(measured) = measure(&value) else {
            continue;
        };
        let recorded = match previous {
            Some(previous) => measured.counts.increase_over(previous),
            None => measured.counts,
        };
        // A partial snapshot does not reset counters it omitted. Preserve
        // the last known totals so reappearing buckets are not billed twice.
        previous = Some(match previous {
            Some(prior) => TokenCounts {
                prompt: measured.counts.prompt.or(prior.prompt),
                completion: measured.counts.completion.or(prior.completion),
                cache: measured.counts.cache.or(prior.cache),
            },
            None => measured.counts,
        });
        if !recorded.any() {
            continue;
        }
        turns.push(HarnessTurnUsage {
            provider: "qwen",
            actor_agent: "qwen",
            text: String::new(),
            model: measured.model,
            prompt_tokens: recorded.prompt,
            completion_tokens: recorded.completion,
            cache_tokens: recorded.cache,
            usage_key: Some(format!("{session_id}:{}", line_hash(line))),
        });
    }
    turns
}

struct Measured {
    counts: TokenCounts,
    model: Option<String>,
}

#[derive(Clone, Copy)]
struct TokenCounts {
    prompt: Option<u32>,
    completion: Option<u32>,
    cache: Option<u32>,
}

impl TokenCounts {
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

/// `next > previous` records the delta. `next < previous` records `next`
/// (counter reset). Equal records nothing. A bucket that was absent and
/// is now present records that value, including 0. A missing key stays
/// `None` and is not compared as zero.
fn increase(previous: Option<u32>, next: Option<u32>) -> Option<u32> {
    let next = next?;
    match previous {
        Some(previous) if next == previous => None,
        Some(previous) if next > previous => Some(next - previous),
        _ => Some(next),
    }
}

fn measure(record: &serde_json::Value) -> Option<Measured> {
    let models = record.get("models")?.as_object()?;
    let counts = TokenCounts {
        prompt: sum_fields(models, &["inputTokens"]),
        completion: sum_fields(models, &["outputTokens", "thoughtsTokens"]),
        cache: sum_fields(models, &["cachedTokens"]),
    };
    if !counts.any() {
        return None;
    }
    Some(Measured {
        counts,
        model: model_label(models),
    })
}

fn sum_fields(models: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<u32> {
    let mut total = None;
    for model in models.values() {
        for key in keys {
            include(&mut total, model.get(*key).and_then(fitting_u32));
        }
    }
    total
}

fn include(total: &mut Option<u32>, next: Option<u32>) {
    let Some(next) = next else {
        return;
    };
    *total = Some(total.unwrap_or(0).saturating_add(next));
}

/// Present non-negative integers that fit in `u32`, including 0.
/// Negative, non-integer, or too-large values are left out of the sum.
fn fitting_u32(value: &serde_json::Value) -> Option<u32> {
    u32::try_from(value.as_u64()?).ok()
}

fn model_label(models: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    let mut names: Vec<&str> = models
        .keys()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .collect();
    names.sort_unstable();
    names.dedup();
    if names.is_empty() {
        None
    } else {
        Some(names.join("+"))
    }
}

fn line_hash(raw: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn usage_rows(store: &hub::HubStore) -> Vec<serde_json::Value> {
        store
            .list_audit_events(false)
            .unwrap()
            .into_iter()
            .filter(|event| event.operation == "provider.usage")
            .map(|event| serde_json::from_str(&event.process_json).unwrap())
            .collect()
    }

    fn record(dir: &Path, path: &Path, session_id: &str) -> hub::HubStore {
        let store_dir = dir.join("hub");
        fs::create_dir_all(&store_dir).unwrap();
        let store = hub::HubStore::open(&store_dir).unwrap();
        record_qwen_session_usage(&store, path, session_id, Some("hub-s"));
        store
    }

    fn write(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
    }

    const SAMPLE: &str = concat!(
        r#"{"version":1,"sessionId":"S","timestamp":1788978939883,"#,
        r#""startTime":1788978936079,"project":"/tmp/qwen","models":{"qwen3.5-plus":{"#,
        r#""requests":1,"inputTokens":12,"outputTokens":4,"cachedTokens":2,"thoughtsTokens":3,"#,
        r#""totalTokens":19,"totalLatencyMs":3257}},"tools":{"totalCalls":0,"totalSuccess":0,"#,
        r#""totalFail":0,"byName":{}}}"#,
        "\n",
    );

    #[test]
    fn sample_line_records_tokens_and_repoll_is_a_noop() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("usage_record.jsonl");
        write(&path, SAMPLE);
        let store = hub::HubStore::open(dir.path()).unwrap();
        record_qwen_session_usage(&store, &path, "S", Some("hub-s"));
        let rows = usage_rows(&store);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["provider"], "qwen");
        assert_eq!(rows[0]["actor_agent"], "qwen");
        assert_eq!(rows[0]["prompt_tokens"], 12);
        assert_eq!(rows[0]["completion_tokens"], 7);
        assert_eq!(rows[0]["cache_tokens"], 2);
        assert_eq!(rows[0]["model"], "qwen3.5-plus");
        assert_eq!(rows[0]["currency"], "unknown");
        assert_eq!(rows[0]["session_id"], "hub-s");
        assert!(rows[0].get("cost").is_none());
        record_qwen_session_usage(&store, &path, "S", Some("hub-s"));
        assert_eq!(usage_rows(&store).len(), 1);
    }

    #[test]
    fn later_line_records_only_the_increase_and_equal_totals_add_nothing() {
        let dir = tempdir().unwrap();
        let higher = dir.path().join("higher.jsonl");
        write(
            &higher,
            &format!(
                "{SAMPLE}{}",
                concat!(
                    r#"{"version":1,"sessionId":"S","timestamp":1788978940000,"#,
                    r#""models":{"qwen3.5-plus":{"inputTokens":20,"outputTokens":6,"#,
                    r#""cachedTokens":2,"thoughtsTokens":5,"totalTokens":99}}}"#,
                    "\n",
                )
            ),
        );
        let rows = usage_rows(&record(dir.path(), &higher, "S"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1]["prompt_tokens"], 8);
        assert_eq!(rows[1]["completion_tokens"], 4);
        assert!(rows[1].get("cache_tokens").is_none());

        let equal_dir = tempdir().unwrap();
        let equal = equal_dir.path().join("equal.jsonl");
        write(
            &equal,
            &format!(
                "{SAMPLE}{}",
                concat!(
                    r#"{"version":1,"sessionId":"S","timestamp":1788978940001,"#,
                    r#""models":{"qwen3.5-plus":{"inputTokens":12,"outputTokens":4,"#,
                    r#""cachedTokens":2,"thoughtsTokens":3}}}"#,
                    "\n",
                )
            ),
        );
        assert_eq!(usage_rows(&record(equal_dir.path(), &equal, "S")).len(), 1);
    }

    #[test]
    fn counter_reset_records_the_new_value() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("usage_record.jsonl");
        write(
            &path,
            concat!(
                r#"{"sessionId":"S","models":{"m":{"inputTokens":12,"outputTokens":9,"cachedTokens":4}}}"#,
                "\n",
                r#"{"sessionId":"S","timestamp":2,"models":{"m":{"inputTokens":3,"outputTokens":1,"cachedTokens":1}}}"#,
                "\n",
            ),
        );
        let rows = usage_rows(&record(dir.path(), &path, "S"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1]["prompt_tokens"], 3);
        assert_eq!(rows[1]["completion_tokens"], 1);
        assert_eq!(rows[1]["cache_tokens"], 1);
    }

    #[test]
    fn other_session_ids_are_not_attributed_to_this_capture() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("usage_record.jsonl");
        write(
            &path,
            &format!(
                "{}\n{SAMPLE}",
                r#"{"sessionId":"OTHER","models":{"m":{"inputTokens":1000,"outputTokens":1000}}}"#,
            ),
        );
        let rows = usage_rows(&record(dir.path(), &path, "S"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["prompt_tokens"], 12);
        assert_eq!(rows[0]["completion_tokens"], 7);
    }

    #[test]
    fn no_models_writes_nothing_and_missing_input_leaves_prompt_absent() {
        let dir = tempdir().unwrap();
        let bare = dir.path().join("bare.jsonl");
        write(&bare, "{\"sessionId\":\"S\",\"timestamp\":1}\n");
        assert!(usage_rows(&record(dir.path(), &bare, "S")).is_empty());

        let partial = dir.path().join("partial.jsonl");
        write(
            &partial,
            concat!(
                r#"{"sessionId":"S","models":{"qwen3.5-plus":{"outputTokens":4,"thoughtsTokens":3}}}"#,
                "\n",
            ),
        );
        let rows = usage_rows(&record(dir.path(), &partial, "S"));
        assert_eq!(rows.len(), 1);
        assert!(rows[0].get("prompt_tokens").is_none());
        assert_eq!(rows[0]["completion_tokens"], 7);
        assert!(rows[0].get("cache_tokens").is_none());
        assert_eq!(rows[0]["model"], "qwen3.5-plus");
    }

    #[test]
    fn two_model_keys_sum_and_the_label_is_sorted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("usage_record.jsonl");
        write(
            &path,
            concat!(
                r#"{"sessionId":"S","models":{"#,
                r#""qwen3.5-plus":{"inputTokens":10,"outputTokens":1},"#,
                r#""qwen-max":{"inputTokens":2,"outputTokens":3,"cachedTokens":4}}}"#,
                "\n",
            ),
        );
        let rows = usage_rows(&record(dir.path(), &path, "S"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["prompt_tokens"], 12);
        assert_eq!(rows[0]["completion_tokens"], 4);
        assert_eq!(rows[0]["cache_tokens"], 4);
        assert_eq!(rows[0]["model"], "qwen-max+qwen3.5-plus");
    }

    #[test]
    fn missing_usage_file_records_nothing() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope.jsonl");
        assert!(usage_rows(&record(dir.path(), &missing, "S")).is_empty());
    }
    #[test]
    fn missing_bucket_does_not_erase_its_previous_total() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("usage_record.jsonl");
        write(
            &path,
            concat!(
                r#"{"sessionId":"S","timestamp":1,"models":{"m":{"inputTokens":12,"outputTokens":4}}}"#,
                "\n",
                r#"{"sessionId":"S","timestamp":2,"models":{"m":{"outputTokens":5}}}"#,
                "\n",
                r#"{"sessionId":"S","timestamp":3,"models":{"m":{"inputTokens":12,"outputTokens":6}}}"#,
                "\n",
            ),
        );
        let rows = usage_rows(&record(dir.path(), &path, "S"));
        assert_eq!(rows.len(), 3);
        assert!(rows[1].get("prompt_tokens").is_none());
        assert!(rows[2].get("prompt_tokens").is_none());
        assert_eq!(rows[2]["completion_tokens"], 1);
    }
}
