//! Aggregate provider probes: the static local-runtime rows, the composite
//! Mistral/Kimi reads, the metered-probe cost gate, and the per-agent /
//! all-provider entry points both callers (desktop IPC, headless CLI
//! schedule tick) use.
use crate::claude::claude_quota;
use crate::codex::{
    codex_quota, now_unix, unavailable_quota, ProviderQuota, ProviderQuotaLocalUsage,
};
use crate::cursor::cursor_quota;
use crate::deepseek::deepseek_quota;
use crate::gemini::gemini_quota;
use crate::grok::grok_quota;
use crate::hermes::hermes_quota;
use crate::kimi_usage;
use crate::muse::muse_quota;
use crate::opencode::opencode_quota;
use crate::qwen::qwen_quota;

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
        crate::mistral::mistral_admin_budget(),
        crate::vibe_usage::local_usage(),
    )
}

/// Pure merge of the two Mistral halves so tests can drive a capped admin
/// response beside local usage without network or disk. `Ok` fills
/// `windows`/`balance`/`balance_info` beside `local_usage`; `Err` keeps the
/// local-only read-out with the admin reason in its detail; both failing
/// stays `unavailable` with both reasons named.
pub(crate) fn compose_mistral_quota(
    admin: Result<crate::mistral::MistralAdminBudget, String>,
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
    match kimi_usage::local_usage() {
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

/// `orchestration.allow_metered_quota_probes`, read once per refresh from
/// the settings store under `home`. The `gemini` / `opencode` / `muse`
/// adapters are the only ones whose usage read costs the user tokens (a
/// model turn or a minimal completion); when this is off they short-circuit
/// to "unavailable" instead of spending anything. Every other adapter reads
/// a free endpoint and ignores this. A missing or unreadable settings file
/// degrades to the safe-for-cost default of `true` — same as a fresh
/// install. `home` is a parameter (not always `hub::default_hub_home()`)
/// so headless callers opened against a `--home` override read the same
/// settings store as the hub they tick.
fn metered_quota_probes_allowed_at(home: &std::path::Path) -> bool {
    hub::SettingsStore::open(home)
        .effective(None)
        .orchestration
        .allow_metered_quota_probes
}

/// Every provider probe in one pass, in the fixed order the Usage tab has
/// always rendered.
///
/// **Blocking — callers must offload.** `codex_quota` and `gemini_quota`
/// spawn a real subprocess and block reading its stdout (gemini_quota's
/// `agy` call alone allows up to 25s). The desktop command runs this on a
/// `spawn_blocking` thread — confirmed live, 2026-08-14: running it inline
/// on the IPC dispatch thread froze the whole window, not just the Usage
/// panel. Headless `ca schedule tick` calls the per-agent
/// [`probe_agent_quota_at`] instead, only for agents a due task targets.
pub fn all_provider_quotas() -> Vec<ProviderQuota> {
    all_provider_quotas_at(&hub::default_hub_home())
}

/// [`all_provider_quotas`] reading `allow_metered_quota_probes` from the
/// settings store under `home`.
pub fn all_provider_quotas_at(home: &std::path::Path) -> Vec<ProviderQuota> {
    let allow_metered = metered_quota_probes_allowed_at(home);
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
        qwen_quota(),
        kimi_quota(),
        hermes_quota(),
        llamacpp_quota(),
        ollama_quota(),
    ]
}

/// Sync per-agent probe used by the schedule budget bridge (in-app S6
/// poller and headless S5c tick alike). Unknown ids stay `unavailable`
/// rather than inventing a percent remaining. **Blocking** for the
/// subprocess-backed adapters; offload when called from an async context.
pub fn probe_agent_quota(agent_id: &str) -> ProviderQuota {
    probe_agent_quota_at(&hub::default_hub_home(), agent_id)
}

/// [`probe_agent_quota`] reading `allow_metered_quota_probes` from the
/// settings store under `home`.
pub fn probe_agent_quota_at(home: &std::path::Path, agent_id: &str) -> ProviderQuota {
    let allow_metered = metered_quota_probes_allowed_at(home);
    match agent_id {
        "claude" => claude_quota(),
        "grok" => grok_quota(),
        "chat" | "codex" => codex_quota(),
        "cursor" => cursor_quota(),
        "gemini" => gemini_quota(allow_metered),
        "opencode" => opencode_quota(allow_metered),
        "deepseek" => deepseek_quota(),
        "muse" => muse_quota(allow_metered),
        "mistral" | "vibe" => mistral_quota(),
        "qwen" => qwen_quota(),
        "kimi" => kimi_quota(),
        "hermes" | "hermes-agent" => hermes_quota(),
        "llamacpp" => llamacpp_quota(),
        "ollama" => ollama_quota(),
        other => unavailable_quota(other, "unknown", other, "Unknown provider agent id"),
    }
}
