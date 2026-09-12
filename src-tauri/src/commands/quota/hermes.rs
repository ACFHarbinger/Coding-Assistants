//! Hermes Agent local-usage quota adapter (#322, C14.16).
//!
//! Every managed spawn passes `--usage-file <hub>/hermes-usage/<name>.json`
//! (see `harness::hermes_spawn`), and the CLI writes the report even when
//! the run fails — verified live against `hermes 0.21.2`:
//!
//! ```json
//! {"estimated_cost_usd": 0.0, "cost_status": "estimated",
//!  "input_tokens": 12356, "output_tokens": 17, "cache_read_tokens": 0,
//!  "cache_write_tokens": 0, "reasoning_tokens": 13, "total_tokens": 12373,
//!  "api_calls": 1, "model": "upstage/solar-pro4:free", "provider": "nous",
//!  "session_id": "20260912_232548_916eae", "completed": true, "failed": false}
//! ```
//!
//! This reader sums those files newest-first (cap 500): `input_tokens` →
//! prompt, `output_tokens` + `reasoning_tokens` → completion (there is no
//! reasoning field to put them in), both cache fields → cached. `api_calls`
//! are LLM calls, not tool calls, so they are not summed. Files carry no
//! timestamp, so `since` is the oldest file mtime. Each file counts as one
//! session (one recorded run). No network, no model turn, not gated on
//! `allow_metered_quota_probes`. Estimated dollar costs stay in the files;
//! only token counts surface here.

use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaLocalUsage};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const AGENT_ID: &str = "hermes";
const PROVIDER: &str = "nous";
const HARNESS_TITLE: &str = "Hermes Agent";
const MAX_FILES_SCANNED: usize = 500;
const LOCAL_ONLY_DETAIL: &str = "Locally recorded Hermes Agent call usage. \
     Plan/budget windows live in the Nous Portal, not in a CLI-readable API.";
const NO_SESSIONS_DETAIL: &str =
    "No Hermes Agent calls recorded yet; run a managed `hermes` task once";

fn unavailable(detail: impl Into<String>) -> ProviderQuota {
    unavailable_quota(AGENT_ID, PROVIDER, HARNESS_TITLE, detail)
}

fn counter(report: &Value, key: &str) -> u64 {
    report
        .get(key)
        .and_then(Value::as_i64)
        .filter(|n| *n >= 0)
        .unwrap_or(0) as u64
}

/// Fold one `--usage-file` report into the running totals. Pure: tests drive
/// this directly. Reports from failed runs still carry token counts and are
/// summed — tokens were spent either way.
pub(crate) fn accumulate(total: &mut ProviderQuotaLocalUsage, report: &Value) {
    total.sessions += 1;
    total.prompt_tokens = total
        .prompt_tokens
        .saturating_add(counter(report, "input_tokens"));
    total.completion_tokens = total
        .completion_tokens
        .saturating_add(counter(report, "output_tokens"))
        .saturating_add(counter(report, "reasoning_tokens"));
    total.cached_tokens = total
        .cached_tokens
        .saturating_add(counter(report, "cache_read_tokens"))
        .saturating_add(counter(report, "cache_write_tokens"));
}

fn file_mtime_secs(path: &Path) -> Option<i64> {
    path.metadata()
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|age| age.as_secs() as i64)
}

/// Sum the newest [`MAX_FILES_SCANNED`] usage reports in `dir`. `None` when
/// the directory is missing or holds no parseable report — callers omit
/// `local_usage` rather than reporting zeroes.
pub(crate) fn local_usage_from(dir: &Path) -> Option<ProviderQuotaLocalUsage> {
    let mut files: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json")
        })
        .map(|path| {
            let modified = path
                .metadata()
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (modified, path)
        })
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.truncate(MAX_FILES_SCANNED);

    let mut total = ProviderQuotaLocalUsage::default();
    for (_, path) in &files {
        let Ok(raw) = std::fs::read(path) else {
            continue;
        };
        let Ok(report) = serde_json::from_slice::<Value>(&raw) else {
            continue;
        };
        accumulate(&mut total, &report);
        if let Some(mtime) = file_mtime_secs(path) {
            total.since = Some(match total.since {
                Some(earliest) => earliest.min(mtime),
                None => mtime,
            });
        }
    }
    (total.sessions > 0).then_some(total)
}

pub(crate) fn local_usage() -> Option<ProviderQuotaLocalUsage> {
    local_usage_from(&hub::hermes_usage_dir())
}

/// Wrap local totals in a `ProviderQuota`. Tests drive this without disk.
pub(crate) fn hermes_quota_from_local(local: Option<ProviderQuotaLocalUsage>) -> ProviderQuota {
    match local {
        Some(local_usage) => ProviderQuota {
            agent_id: AGENT_ID.into(),
            provider: PROVIDER.into(),
            harness_title: HARNESS_TITLE.into(),
            status: "ok".into(),
            detail: Some(LOCAL_ONLY_DETAIL.into()),
            windows: Vec::new(),
            fetched_at: now_unix(),
            balance: None,
            balance_info: None,
            local_usage: Some(local_usage),
        },
        None => unavailable(NO_SESSIONS_DETAIL),
    }
}

pub(crate) fn hermes_quota() -> ProviderQuota {
    hermes_quota_from_local(local_usage())
}

#[cfg(test)]
#[path = "hermes_tests.rs"]
mod tests;
