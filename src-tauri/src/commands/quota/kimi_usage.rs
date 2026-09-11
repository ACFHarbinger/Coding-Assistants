//! Local Moonshot Kimi Code session usage, read from the CLI's own on-disk
//! accounting (#311, C14.14).
//!
//! Kimi exposes no stable direct plan-budget endpoint for a quota adapter to
//! call: the account surface (`/oauth/usage` — weekly + rolling windows plus
//! an optional pay-as-you-go balance) is served through the CLI's own local
//! daemon (`kimi web` → `GET /api/v1/oauth/usage`), which composes the
//! upstream call internally (region-aware host selection, 15-minute OAuth
//! token refresh). A `Bearer` call with the on-disk OAuth token 404s on every
//! probed host (`auth.kimi.ai`, `auth.kimi.com`, `api.kimi.ai`,
//! `platform.kimi.ai`, `platform.moonshot.cn`, `api.moonshot.cn`), so there is
//! no direct route to reimplement — and quota refresh must not assume a
//! daemon is running. The account half therefore stays unbuilt (spike
//! recorded on #311); this module is the free local half, merged later the
//! way `mistral_quota()` merges its two halves if a stable route is found.
//!
//! What `kimi` does write, as a normal side effect of every turn, is a
//! per-turn token record in each session's wire event log (captured live from
//! `kimi 0.42.0`, 2026-09-11):
//!
//! ```text
//! $KIMI_CODE_HOME/sessions (≈ ~/.kimi-code/sessions)
//!   └── wd_<slug>_<hash>/
//!       └── session_<uuid>/
//!           ├── state.json          (createdAt: ms epoch — the `since` floor)
//!           └── agents/main/wire.jsonl
//!               {"type":"usage.record","usage":{"inputOther":1849,"output":21,
//!                "inputCacheRead":18944,"inputCacheCreation":0},
//!                "usageScope":"turn", ...}
//! ```
//!
//! Only `usageScope == "turn"` records are summed (a missing scope is counted
//! for backward compatibility; any other scope is skipped so a future scope
//! cannot double-count). `inputOther` feeds `prompt_tokens`, `output` feeds
//! `completion_tokens`, and both cache fields feed `cached_tokens` — the same
//! input/output/cached split `vibe_usage.rs` uses. A session counts when its
//! `wire.jsonl` reads, even with zero records (mirrors a crashed Vibe
//! session); an unreadable file is skipped, not fatal.

use super::quota_codex::ProviderQuotaLocalUsage;
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cap on `wire.jsonl` files inspected in one sweep. A long-lived install
/// accumulates sessions without bound; this keeps a quota refresh bounded no
/// matter how many have piled up. Newest-first, so the cap drops the oldest.
const MAX_SESSIONS_SCANNED: usize = 500;

/// `$KIMI_CODE_HOME`, falling back to `~/.kimi-code`. `KIMI_CODE_HOME` is the
/// CLI's own documented override, which also makes readers testable against
/// a scratch directory with no global state. (Retarget to
/// `hub::kimi_sessions_root()` when #309 lands on `main`.)
pub(crate) fn kimi_home() -> PathBuf {
    match std::env::var("KIMI_CODE_HOME") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir.trim()),
        _ => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".kimi-code")
        }
    }
}

/// `$KIMI_CODE_HOME/sessions`, falling back to `~/.kimi-code/sessions`.
pub(crate) fn kimi_sessions_root() -> PathBuf {
    kimi_home().join("sessions")
}

/// A non-negative token counter, tolerating integer and float JSON shapes
/// the way `cursor.rs` does.
fn counter(usage: &Value, key: &str) -> u64 {
    let amount = usage
        .get(key)
        .and_then(|value| {
            value
                .as_u64()
                .map(|n| n as f64)
                .or_else(|| value.as_i64().map(|n| n as f64))
                .or_else(|| value.as_f64())
        })
        .unwrap_or(0.0);
    if amount.is_finite() && amount >= 0.0 {
        amount.round() as u64
    } else {
        0
    }
}

/// Fold one `wire.jsonl` line into the running totals. Pure: every test
/// drives this directly rather than laying out a filesystem. Lines that are
/// not turn-scoped `usage.record` entries contribute nothing.
pub(crate) fn accumulate_line(total: &mut ProviderQuotaLocalUsage, line: &Value) {
    if line.get("type").and_then(Value::as_str) != Some("usage.record") {
        return;
    }
    // The only scope observed live is "turn". A missing scope is counted for
    // backward compatibility; any other scope is skipped so a future scope
    // cannot double-count the same tokens.
    if let Some(scope) = line.get("usageScope").and_then(Value::as_str) {
        if scope != "turn" {
            return;
        }
    }
    let Some(usage) = line.get("usage") else {
        return;
    };
    // Saturating, not `+=`: these counters fold an unbounded number of
    // sessions from files that are just JSON on disk — a crafted or corrupt
    // value near `u64::MAX` must clamp the total, not panic a debug build or
    // silently wrap a release one into a tiny, wrong number.
    total.prompt_tokens = total
        .prompt_tokens
        .saturating_add(counter(usage, "inputOther"));
    total.completion_tokens = total
        .completion_tokens
        .saturating_add(counter(usage, "output"));
    total.cached_tokens = total
        .cached_tokens
        .saturating_add(counter(usage, "inputCacheRead"))
        .saturating_add(counter(usage, "inputCacheCreation"));
}

/// `state.json` `createdAt` (ms epoch) beside a `wire.jsonl` path, or `None`
/// when it is absent or unparseable. The wire log lives at
/// `session_<id>/agents/main/wire.jsonl`, so the state file is three levels
/// up from the log's parent.
fn session_created_secs(wire_path: &Path) -> Option<i64> {
    let state_path = wire_path.parent()?.parent()?.parent()?.join("state.json");
    let raw = std::fs::read_to_string(state_path).ok()?;
    let state: Value = serde_json::from_str(&raw).ok()?;
    let millis = state
        .get("createdAt")
        .and_then(Value::as_i64)
        .filter(|n| *n > 0)?;
    Some(millis / 1000)
}

/// Fold one session's `wire.jsonl` into the running totals: the session
/// counts when the log opens (even with zero records), and its `createdAt`
/// floors `since`. Streams the file line by line rather than reading it
/// whole — a `wire.jsonl` is a full agent event log (tool schemas, system
/// prompts, every loop event), not the terse per-turn record this reader
/// actually wants, so a long-lived session's log can be large; peak memory
/// here is one line, not the file.
fn accumulate_file(total: &mut ProviderQuotaLocalUsage, wire_path: &Path) {
    let Ok(file) = std::fs::File::open(wire_path) else {
        return;
    };
    total.sessions += 1;
    for line in BufReader::new(file).lines() {
        // An I/O error (e.g. invalid UTF-8) partway through the file stops
        // this session's folding but keeps what was already summed, rather
        // than discarding the whole session the way a failed whole-file read
        // would have.
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // A half-written trailing line from a session in flight is skipped,
        // not fatal.
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        accumulate_line(total, &value);
    }
    if let Some(created) = session_created_secs(wire_path) {
        total.since = Some(match total.since {
            Some(earliest) => earliest.min(created),
            None => created,
        });
    }
}

/// Newest-first list of `wire.jsonl` paths under `root`, capped at
/// [`MAX_SESSIONS_SCANNED`]. Walks the CLI's own fixed two-level layout
/// directly (`root/<workspace>/<session>/agents/main/wire.jsonl`) instead of
/// a generic recursive directory walk, and keeps at most
/// [`MAX_SESSIONS_SCANNED`] candidates in memory throughout via a bounded
/// min-heap — a long-lived install with far more sessions than the cap never
/// materializes the full list before trimming it down. A `root`, workspace,
/// or session directory that can't be read is skipped, not fatal — Kimi
/// simply has not run yet, or a directory disappeared mid-scan.
fn session_wires(root: &Path) -> Vec<PathBuf> {
    // Min-heap on mtime: the *oldest* kept candidate sits at the top, so once
    // the heap is at capacity a newer one can evict it in O(log cap) without
    // the heap ever holding more than `MAX_SESSIONS_SCANNED` entries.
    let mut newest: BinaryHeap<Reverse<(SystemTime, PathBuf)>> = BinaryHeap::new();

    let Ok(workspace_dirs) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    for workspace_dir in workspace_dirs.flatten() {
        let Ok(session_dirs) = std::fs::read_dir(workspace_dir.path()) else {
            continue;
        };
        for session_dir in session_dirs.flatten() {
            let wire_path = session_dir
                .path()
                .join("agents")
                .join("main")
                .join("wire.jsonl");
            let Ok(metadata) = std::fs::metadata(&wire_path) else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if newest.len() < MAX_SESSIONS_SCANNED {
                newest.push(Reverse((modified, wire_path)));
            } else if let Some(Reverse((oldest_kept, _))) = newest.peek() {
                if modified > *oldest_kept {
                    newest.pop();
                    newest.push(Reverse((modified, wire_path)));
                }
            }
        }
    }

    let mut wires: Vec<(SystemTime, PathBuf)> =
        newest.into_iter().map(|Reverse(pair)| pair).collect();
    wires.sort_by(|a, b| b.0.cmp(&a.0));
    wires.into_iter().map(|(_, path)| path).collect()
}

/// Sum every readable session's turn records. Returns `None` when Kimi has
/// written no sessions at all, so callers can omit `local_usage` entirely
/// rather than reporting a row of zeroes that looks like real measured
/// silence.
pub(crate) fn local_usage_from(root: &Path) -> Option<ProviderQuotaLocalUsage> {
    let mut total = ProviderQuotaLocalUsage::default();
    for path in session_wires(root) {
        // An unreadable log is skipped inside `accumulate_file`, not fatal:
        // a session in flight can be observed mid-write. A *readable* log
        // always counts, even with zero records.
        accumulate_file(&mut total, &path);
    }
    (total.sessions > 0).then_some(total)
}

/// Local usage for the current user's Kimi install.
pub(crate) fn local_usage() -> Option<ProviderQuotaLocalUsage> {
    local_usage_from(&kimi_sessions_root())
}

#[cfg(test)]
#[path = "kimi_usage_tests.rs"]
mod tests;
