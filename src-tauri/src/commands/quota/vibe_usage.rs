//! Local Mistral Vibe session usage, read from the CLI's own on-disk
//! accounting.
//!
//! Vibe has no free usage endpoint on a personal plan — the Mistral Admin API
//! (`/v1/admin/usage`) needs a Backoffice admin key that an individual
//! subscriber does not have. But `vibe` writes a per-session record of its own
//! token accounting, so a truthful local read-out is available on **every**
//! plan tier for the cost of a few `stat`/`read` calls: no network, no model
//! turn, nothing to gate behind `allow_metered_quota_probes`.
//!
//! Layout, captured live from `vibe 2.25.1` (2026-09-09) — one directory per
//! session under `$VIBE_HOME/logs/session` (`VIBE_HOME` defaults to `~/.vibe`):
//!
//! ```text
//! session_<YYYYMMDD>_<HHMMSS>_<short-id>/
//!   ├── messages.jsonl
//!   └── meta.json
//! ```
//!
//! `meta.json` carries `session_id`, `start_time` (RFC 3339), and a `stats`
//! object:
//!
//! ```json
//! {"steps": 2, "session_prompt_tokens": 4967, "session_completion_tokens": 40,
//!  "session_cached_tokens": 2048, "context_tokens": 5007,
//!  "tool_calls_agreed": 0, "tool_calls_rejected": 0, "tool_calls_hook_denied": 0,
//!  "tool_calls_failed": 0, "tool_calls_succeeded": 0}
//! ```
//!
//! Only the `session_*` totals are summed. `context_tokens` and the
//! `last_turn_*` fields are point-in-time gauges for the final turn, not
//! counters, so adding them across sessions would be meaningless.

use super::quota_codex::ProviderQuotaLocalUsage;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Cap on directories inspected in one sweep. A long-lived install
/// accumulates a directory per session; this keeps a quota refresh bounded no
/// matter how many have piled up. Newest-first, so the cap drops the oldest.
const MAX_SESSIONS_SCANNED: usize = 500;

/// `$VIBE_HOME/logs/session`, falling back to `~/.vibe/logs/session`.
/// `VIBE_HOME` is the CLI's own documented override, which also makes this
/// testable against a scratch directory.
pub(crate) fn vibe_sessions_root() -> PathBuf {
    let home = match std::env::var("VIBE_HOME") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir.trim()),
        _ => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".vibe")
        }
    };
    home.join("logs").join("session")
}

fn counter(stats: &Value, key: &str) -> u64 {
    stats
        .get(key)
        .and_then(Value::as_i64)
        .filter(|n| *n >= 0)
        .unwrap_or(0) as u64
}

/// Seconds since the epoch for a `meta.json` `start_time`, or `None` if it is
/// absent or not RFC 3339.
fn start_epoch(meta: &Value) -> Option<i64> {
    let raw = meta.get("start_time").and_then(Value::as_str)?;
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp())
}

/// Fold one parsed `meta.json` into the running totals. Pure: every test
/// drives this directly rather than laying out a filesystem.
pub(crate) fn accumulate(total: &mut ProviderQuotaLocalUsage, meta: &Value) {
    let Some(stats) = meta.get("stats") else {
        // A session that crashed before writing `stats` still counts as a
        // session, but contributes no tokens.
        total.sessions += 1;
        return;
    };
    total.sessions += 1;
    total.prompt_tokens += counter(stats, "session_prompt_tokens");
    total.completion_tokens += counter(stats, "session_completion_tokens");
    total.cached_tokens += counter(stats, "session_cached_tokens");
    total.tool_calls_succeeded += counter(stats, "tool_calls_succeeded");
    total.tool_calls_failed += counter(stats, "tool_calls_failed");
    total.tool_calls_rejected += counter(stats, "tool_calls_rejected");
    if let Some(started) = start_epoch(meta) {
        total.since = Some(match total.since {
            Some(earliest) => earliest.min(started),
            None => started,
        });
    }
}

/// Newest-first list of `meta.json` paths under `root`, capped at
/// [`MAX_SESSIONS_SCANNED`]. A `root` that does not exist yields an empty
/// list rather than an error — Vibe simply has not run yet.
fn session_metas(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let meta_path = entry.path().join("meta.json");
            if !meta_path.is_file() {
                return None;
            }
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            Some((modified, meta_path))
        })
        .collect();
    dirs.sort_by(|a, b| b.0.cmp(&a.0));
    dirs.truncate(MAX_SESSIONS_SCANNED);
    dirs.into_iter().map(|(_, path)| path).collect()
}

/// Sum every readable session's `stats`. Returns `None` when Vibe has written
/// no sessions at all, so callers can omit `local_usage` entirely rather than
/// reporting a row of zeroes that looks like real measured silence.
pub(crate) fn local_usage_from(root: &Path) -> Option<ProviderQuotaLocalUsage> {
    let mut total = ProviderQuotaLocalUsage::default();
    for path in session_metas(root) {
        // An unreadable or half-written meta.json is skipped, not fatal: a
        // session in flight can be observed mid-write.
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(meta) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        accumulate(&mut total, &meta);
    }
    (total.sessions > 0).then_some(total)
}

/// Local usage for the current user's Vibe install.
pub(crate) fn local_usage() -> Option<ProviderQuotaLocalUsage> {
    local_usage_from(&vibe_sessions_root())
}

#[cfg(test)]
#[path = "vibe_usage_tests.rs"]
mod tests;
