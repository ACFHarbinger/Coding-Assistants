//! Static and aggregate provider quota commands.
use super::quota_claude::claude_quota;
use super::quota_codex::{codex_quota, unavailable_quota, ProviderQuota};
use super::quota_cursor::cursor_quota;
use super::quota_deepseek::deepseek_quota;
use super::quota_gemini::gemini_quota;
use super::quota_grok::grok_quota;
use super::quota_muse::muse_quota;
use super::quota_opencode::opencode_quota;

fn llamacpp_quota() -> ProviderQuota {
    unavailable_quota(
        "llamacpp",
        "local",
        "Local Llama.cpp",
        "Local llama.cpp server offline or unmetered local execution",
    )
}

fn ollama_quota() -> ProviderQuota {
    unavailable_quota(
        "ollama",
        "local",
        "Local Ollama",
        "Local Ollama server offline or unmetered local execution",
    )
}

fn mistral_quota() -> ProviderQuota {
    unavailable_quota(
        "mistral",
        "mistral",
        "Mistral Vibe",
        "vibe CLI does not expose a usage-budget command; run vibe --setup if unauthenticated",
    )
}

/// `orchestration.allow_metered_quota_probes`, read once per refresh. The
/// `gemini` / `opencode` / `muse` adapters are the only ones whose usage
/// read costs the user tokens (a model turn or a minimal completion); when
/// this is off they short-circuit to "unavailable" instead of spending
/// anything. Every other adapter reads a free endpoint and ignores this.
/// A missing or unreadable settings file degrades to the safe-for-cost
/// default of `true` — same as a fresh install.
fn metered_quota_probes_allowed() -> bool {
    hub::SettingsStore::open(hub::default_hub_home())
        .effective(None)
        .orchestration
        .allow_metered_quota_probes
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
    tauri::async_runtime::spawn_blocking(|| {
        let allow_metered = metered_quota_probes_allowed();
        vec![
            claude_quota(),
            grok_quota(),
            codex_quota(),
            cursor_quota(),
            gemini_quota(allow_metered),
            opencode_quota(allow_metered),
            deepseek_quota(),
            muse_quota(allow_metered),
            mistral_quota(),
            llamacpp_quota(),
            ollama_quota(),
        ]
    })
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
    tauri::async_runtime::spawn_blocking(move || match agent_id.as_str() {
        "claude" => claude_quota(),
        "grok" => grok_quota(),
        "chat" | "codex" => codex_quota(),
        "cursor" => cursor_quota(),
        "gemini" => gemini_quota(metered_quota_probes_allowed()),
        "opencode" => opencode_quota(metered_quota_probes_allowed()),
        "deepseek" => deepseek_quota(),
        "muse" => muse_quota(metered_quota_probes_allowed()),
        "mistral" | "vibe" => mistral_quota(),
        "llamacpp" => llamacpp_quota(),
        "ollama" => ollama_quota(),
        other => unavailable_quota(other, "unknown", other, "Unknown provider agent id"),
    })
    .await
    .map_err(|error| format!("provider quota refresh task panicked: {error}"))
}
