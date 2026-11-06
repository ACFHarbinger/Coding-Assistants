//! Qwen Code local-usage quota adapter (#310).
//!
//! Spike, `qwen` 0.23.2, 2026-09-11 — report lives on the GitHub issue:
//!
//! - **Free persisted source:** `~/.qwen/usage_record.jsonl`, written by the
//!   CLI's `usageHistoryService.persistSessionUsage` at end of session.
//!   `$QWEN_HOME` relocates that global dir (the live env; `QWEN_CODE_HOME`
//!   is not read by 0.23.2). No network, no model turn, not gated on
//!   `allow_metered_quota_probes`.
//! - **`qwen sessions list|ps --json`:** no usage field. Empty even in a
//!   project that has `projects/*/chats/*.jsonl`. Chat transcripts carry no
//!   token counters either.
//! - **Account / Token Plan budget:** the `sk-sp-…`
//!   `BAILIAN_CODING_PLAN_API_KEY` is inference-only.
//!   `GET coding-intl.dashscope.aliyuncs.com/v1/models` → 200;
//!   `/v1/usage`, `/api/v1/{usage,quota}`,
//!   `/v1/dashboard/billing/credit_grants` → 404. Remaining credits live on
//!   cookie-authenticated console RPCs, the same class as Mistral's
//!   Admin-key gate and the Cursor app `/usage` panel (#281) — not scraped.
//! - **stream-json `result.usage`:** stdout of a live turn only. Unused
//!   here; the jsonl file is the free equivalent of that tail.
//!
//! Shape captured live from three end-of-session lines (token fields were
//! zero on those 401-era runs; the keys are what this reader sums):
//!
//! ```json
//! {"version":1,"sessionId":"…","timestamp":1788978939883,
//!  "startTime":1788978936079,"project":"/tmp/…",
//!  "models":{"qwen3.5-plus":{"requests":1,"inputTokens":12,
//!    "outputTokens":4,"cachedTokens":2,"thoughtsTokens":3,
//!    "totalTokens":19,"totalLatencyMs":3257}},
//!  "tools":{"totalCalls":0,"totalSuccess":0,"totalFail":0,"byName":{}}}
//! ```
//!
//! `thoughtsTokens` folds into `completion_tokens` (`ProviderQuotaLocalUsage`
//! has no reasoning field). `totalTokens` / latency / files / skills are
//! gauges or duplicates and are not summed.

use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaLocalUsage};
use serde_json::Value;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const AGENT_ID: &str = "qwen";
const PROVIDER: &str = "qwen";
const HARNESS_TITLE: &str = "Qwen Code";
const MAX_SESSIONS_SCANNED: usize = 500;
const PLAN_CONSOLE_DETAIL: &str = "Locally recorded Qwen Code session usage. \
     Remaining Token Plan credits are not exposed to the Coding Plan API key; \
     check the Qwen Cloud / Model Studio Coding Plan console.";
const NO_SESSIONS_DETAIL: &str =
    "No Qwen Code sessions recorded yet (~/.qwen/usage_record.jsonl); run `qwen` once";

/// `$QWEN_HOME`, falling back to `~/.qwen`. Matches `Storage.getGlobalQwenDir`
/// in qwen-code 0.23.2 — `QWEN_HOME` *is* the global dir, not its parent.
pub(crate) fn qwen_home() -> PathBuf {
    match std::env::var("QWEN_HOME") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir.trim()),
        _ => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".qwen")
        }
    }
}

pub(crate) fn usage_record_path() -> PathBuf {
    qwen_home().join("usage_record.jsonl")
}

fn unavailable(detail: impl Into<String>) -> ProviderQuota {
    unavailable_quota(AGENT_ID, PROVIDER, HARNESS_TITLE, detail)
}

fn counter(value: &Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(Value::as_i64)
        .filter(|n| *n >= 0)
        .unwrap_or(0) as u64
}

fn sum_model_counter(models: &Value, key: &str) -> u64 {
    let Some(map) = models.as_object() else {
        return 0;
    };
    // Saturating fold, not `.sum()`: a crafted or corrupt record with
    // several models each near `u64::MAX` must clamp the total, not panic a
    // debug build or wrap a release one into a tiny, wrong number.
    map.values()
        .fold(0u64, |acc, model| acc.saturating_add(counter(model, key)))
}

/// UNIX seconds from a Qwen millisecond (or already-second) timestamp.
fn epoch_secs(value: &Value) -> Option<i64> {
    let n = value
        .as_i64()
        .or_else(|| value.as_f64().map(|n| n as i64))
        .filter(|n| *n > 0)?;
    // 10^12 ms ≈ 2001-09-09; anything that large is milliseconds.
    Some(if n >= 1_000_000_000_000 { n / 1000 } else { n })
}

fn session_start(record: &Value) -> Option<i64> {
    record
        .get("startTime")
        .and_then(epoch_secs)
        .or_else(|| record.get("timestamp").and_then(epoch_secs))
}

/// Fold one `usage_record.jsonl` line into the running totals. Pure: tests
/// drive this directly rather than laying out a file.
pub(crate) fn accumulate(total: &mut ProviderQuotaLocalUsage, record: &Value) {
    total.sessions += 1;
    if let Some(models) = record.get("models") {
        total.prompt_tokens = total
            .prompt_tokens
            .saturating_add(sum_model_counter(models, "inputTokens"));
        total.completion_tokens = total
            .completion_tokens
            .saturating_add(sum_model_counter(models, "outputTokens"))
            .saturating_add(sum_model_counter(models, "thoughtsTokens"));
        total.cached_tokens = total
            .cached_tokens
            .saturating_add(sum_model_counter(models, "cachedTokens"));
    }
    if let Some(tools) = record.get("tools") {
        total.tool_calls_succeeded = total
            .tool_calls_succeeded
            .saturating_add(counter(tools, "totalSuccess"));
        total.tool_calls_failed = total
            .tool_calls_failed
            .saturating_add(counter(tools, "totalFail"));
    }
    if let Some(started) = session_start(record) {
        total.since = Some(match total.since {
            Some(earliest) => earliest.min(started),
            None => started,
        });
    }
}

/// Sum the last [`MAX_SESSIONS_SCANNED`] valid jsonl records. `None` when
/// the file is missing, empty, or every line is unparseable — so callers can
/// omit `local_usage` rather than report a row of zeroes.
///
/// Streams the file line by line into a bounded ring buffer rather than
/// reading it whole and parsing every line before trimming to the cap: a
/// long-lived install's `usage_record.jsonl` grows one line per session
/// without bound, so peak memory here is `MAX_SESSIONS_SCANNED` parsed
/// records, not the file's full line count.
pub(crate) fn local_usage_from(path: &Path) -> Option<ProviderQuotaLocalUsage> {
    let Ok(file) = std::fs::File::open(path) else {
        return None;
    };
    let mut window: VecDeque<Value> = VecDeque::with_capacity(MAX_SESSIONS_SCANNED);
    for line in BufReader::new(file).lines() {
        // An I/O error (e.g. invalid UTF-8) stops the scan but keeps
        // whatever window was already built, rather than discarding
        // everything the way a failed whole-file read would have.
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if window.len() == MAX_SESSIONS_SCANNED {
            window.pop_front();
        }
        window.push_back(record);
    }
    let mut total = ProviderQuotaLocalUsage::default();
    for record in &window {
        accumulate(&mut total, record);
    }
    (total.sessions > 0).then_some(total)
}

pub(crate) fn local_usage() -> Option<ProviderQuotaLocalUsage> {
    local_usage_from(&usage_record_path())
}

/// Wrap local totals in a `ProviderQuota`. Tests drive this without disk.
pub(crate) fn qwen_quota_from_local(local: Option<ProviderQuotaLocalUsage>) -> ProviderQuota {
    match local {
        Some(local_usage) => ProviderQuota {
            agent_id: AGENT_ID.into(),
            provider: PROVIDER.into(),
            harness_title: HARNESS_TITLE.into(),
            status: "ok".into(),
            detail: Some(PLAN_CONSOLE_DETAIL.into()),
            windows: Vec::new(),
            fetched_at: now_unix(),
            balance: None,
            balance_info: None,
            local_usage: Some(local_usage),
        },
        None => unavailable(NO_SESSIONS_DETAIL),
    }
}

pub(crate) fn qwen_quota() -> ProviderQuota {
    qwen_quota_from_local(local_usage())
}

#[cfg(test)]
#[path = "qwen_tests.rs"]
mod tests;
