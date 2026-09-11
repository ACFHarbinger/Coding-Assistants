//! Static and aggregate provider quota commands.
use super::quota_claude::claude_quota;
use super::quota_codex::{
    codex_quota, now_unix, unavailable_quota, ProviderQuota, ProviderQuotaLocalUsage,
};
use super::quota_cursor::cursor_quota;
use super::quota_deepseek::deepseek_quota;
use super::quota_gemini::gemini_quota;
use super::quota_grok::grok_quota;
use super::quota_kimi_usage;
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

/// Mistral quota is two independent halves that merge here. The CLI's own
/// on-disk session accounting (`vibe_usage.rs`) reads truthfully on every
/// plan tier for the cost of a few file reads; the Admin-API budget half
/// (`quota_mistral`, S3) fills `windows`/`balance_info` when
/// `MISTRAL_ADMIN_API_KEY` is configured. Neither half can fail the other:
/// an admin miss still reports local usage, and a machine with no sessions
/// yet still reports a working admin budget.
fn mistral_quota() -> ProviderQuota {
    compose_mistral_quota(
        super::quota_mistral::mistral_admin_budget(),
        super::quota_vibe_usage::local_usage(),
    )
}

/// Pure merge of the two Mistral halves so tests can drive a capped admin
/// response beside local usage without network or disk. `Ok` fills
/// `windows`/`balance`/`balance_info` beside `local_usage`; `Err` keeps the
/// local-only read-out with the admin reason in its detail; both failing
/// stays `unavailable` with both reasons named.
pub(crate) fn compose_mistral_quota(
    admin: Result<super::quota_mistral::MistralAdminBudget, String>,
    local_usage: Option<ProviderQuotaLocalUsage>,
) -> ProviderQuota {
    match admin {
        Ok(budget) => ProviderQuota {
            agent_id: "mistral".into(),
            provider: "mistral".into(),
            harness_title: "Mistral Vibe".into(),
            status: "ok".into(),
            detail: budget.detail,
            windows: budget.windows,
            fetched_at: now_unix(),
            balance: budget.balance,
            balance_info: budget.balance_info,
            local_usage,
        },
        Err(admin_detail) => {
            let Some(local_usage) = local_usage else {
                return unavailable_quota(
                    "mistral",
                    "mistral",
                    "Mistral Vibe",
                    format!(
                        "{admin_detail} No Mistral Vibe sessions recorded yet either; \
                         run `vibe` once (or `vibe --setup` if unauthenticated)"
                    ),
                );
            };
            ProviderQuota {
                agent_id: "mistral".into(),
                provider: "mistral".into(),
                harness_title: "Mistral Vibe".into(),
                status: "ok".into(),
                // The local totals are not a budget, so say what is missing
                // rather than implying they are one.
                detail: Some(format!(
                    "Locally recorded Vibe session usage. {admin_detail}"
                )),
                windows: Vec::new(),
                fetched_at: now_unix(),
                balance: None,
                balance_info: None,
                local_usage: Some(local_usage),
            }
        }
    }
}

/// Kimi quota is the free local half only, for now. The CLI writes a
/// per-turn token record into every session's `wire.jsonl`, so a truthful
/// local read-out is available on every plan tier for the cost of a few
/// file reads: no network, no model turn, nothing to gate behind
/// `allow_metered_quota_probes`. The plan-budget half (`/oauth/usage`
/// weekly + rolling windows, pay-as-you-go balance) is served through the
/// Kimi Code local daemon (`kimi web` → `GET /api/v1/oauth/usage`), not a
/// stable direct endpoint — see the `quota_kimi_usage` module header and
/// #311 — so there is no admin half to merge yet the way `mistral_quota()`
/// merges its two halves. Neither direction fails the other by
/// construction: no sessions yet stays `unavailable` with both reasons
/// named, exactly like the Mistral neither-half case.
pub(crate) fn kimi_quota() -> ProviderQuota {
    match quota_kimi_usage::local_usage() {
        Some(local_usage) => ProviderQuota {
            agent_id: "kimi".into(),
            provider: "kimi".into(),
            harness_title: "Kimi".into(),
            status: "ok".into(),
            // The local totals are not a budget, so say what is missing
            // rather than implying they are one.
            detail: Some(
                "Locally recorded Kimi session usage. Plan-budget windows are \
                 not read here: Moonshot serves account usage through the Kimi \
                 Code local daemon (`kimi web` → /api/v1/oauth/usage), not a \
                 stable direct endpoint (see #311)."
                    .into(),
            ),
            windows: Vec::new(),
            fetched_at: now_unix(),
            balance: None,
            balance_info: None,
            local_usage: Some(local_usage),
        },
        None => unavailable_quota(
            "kimi",
            "kimi",
            "Kimi",
            "No Kimi Code sessions recorded yet; run `kimi` once. \
             Plan-budget windows are not read here: Moonshot serves account \
             usage through the Kimi Code local daemon (`kimi web` → \
             /api/v1/oauth/usage), not a stable direct endpoint (see #311).",
        ),
    }
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
            kimi_quota(),
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
        "kimi" => kimi_quota(),
        "llamacpp" => llamacpp_quota(),
        "ollama" => ollama_quota(),
        other => unavailable_quota(other, "unknown", other, "Unknown provider agent id"),
    })
    .await
    .map_err(|error| format!("provider quota refresh task panicked: {error}"))
}
