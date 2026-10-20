//! Provider quota IPC commands — thin Tauri wrappers over the shared
//! `quota` crate (S5c / #358). All probe logic lives in `crates/quota`,
//! which `crates/cli` also depends on for headless `ca schedule tick`;
//! this module only keeps the IPC surface (async offload + audit
//! snapshot) that is desktop-specific.

use quota::ProviderQuota;

/// Best-effort `quota.snapshot` audit row (Team Scorecard T7 / #347) so the
/// scorecard's quota-vs-cost probe has a balance side. Runs on explicit
/// refreshes only — never the 60-second poll, which would grow the hash
/// chain with duplicate information (the Hub-side writer dedupes unchanged
/// values too). A snapshot failure never blocks the refresh itself.
fn record_snapshot_best_effort(quota: &ProviderQuota) {
    let balance = quota.balance_info.as_ref().map(|info| info.total);
    let (used_percent, window_label) = quota
        .windows
        .first()
        .map(|window| (Some(window.used_percent), Some(window.label.clone())))
        .unwrap_or((None, None));
    let snapshot = hub::QuotaSnapshot {
        provider: quota.provider.clone(),
        balance,
        used_percent,
        window_label,
    };
    if let Ok(store) = crate::commands::commands::store::open_store() {
        let _ = store.record_quota_snapshot(&snapshot);
    }
}

/// Async + `spawn_blocking`, not a plain sync command: `codex_quota` and
/// `gemini_quota` spawn a real subprocess and block reading its stdout
/// (gemini_quota's `agy` call alone allows up to 25s). A sync
/// `#[tauri::command]` runs inline on the same thread that dispatches IPC
/// — confirmed live, 2026-08-14: selecting the Usage tab froze the whole
/// window, not just that panel, until the subprocess read returned. Moving
/// the blocking work onto a `spawn_blocking` thread keeps the rest of the
/// app responsive while this is in flight.
#[tauri::command]
pub async fn hub_get_provider_quotas() -> Result<Vec<ProviderQuota>, String> {
    tauri::async_runtime::spawn_blocking(quota::all_provider_quotas)
        .await
        .map_err(|error| format!("provider quota fetch task panicked: {error}"))
}

/// `chat` (Codex) and `grok` fetch a live process/API on every call with no
/// staleness risk, so the frontend keeps their "live quota" label and skips
/// the refresh button. Every other provider gets a "last refreshed" label and
/// a manual refresh control (some, like Claude Code and Antigravity CLI,
/// expose no official usage-budget command, so their snapshot can only be
/// updated by an explicit refresh).
#[tauri::command]
pub async fn hub_refresh_provider_quota(agent_id: String) -> Result<ProviderQuota, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let quota = quota::probe_agent_quota(&agent_id);
        record_snapshot_best_effort(&quota);
        quota
    })
    .await
    .map_err(|error| format!("provider quota refresh task panicked: {error}"))
}
