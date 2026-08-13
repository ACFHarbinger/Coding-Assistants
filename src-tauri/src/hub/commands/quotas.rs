//! Static and aggregate provider quota commands.
use super::quota_claude::claude_quota;
use super::quota_codex::{
    codex_quota, now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow,
};
use super::quota_grok::grok_quota;
fn gemini_quota() -> ProviderQuota {
    let now = now_unix();
    let windows = vec![
        // Gemini Model Family
        ProviderQuotaWindow {
            label: "Weekly Limit Remaining".into(),
            family: Some("Gemini Model Family".into()),
            used_percent: 66,
            remaining_percent: 34,
            resets_at: Some(now + 108 * 3600 + 55 * 60),
            window_minutes: Some(7 * 24 * 60),
        },
        ProviderQuotaWindow {
            label: "Five Hour Limit Remaining".into(),
            family: Some("Gemini Model Family".into()),
            used_percent: 0,
            remaining_percent: 100,
            resets_at: None,
            window_minutes: Some(5 * 60),
        },
        // Other Model Families (Claude & GPT models in Antigravity)
        ProviderQuotaWindow {
            label: "Weekly Limit Remaining".into(),
            family: Some("Other Model Families".into()),
            used_percent: 100,
            remaining_percent: 0,
            resets_at: Some(now + 27 * 3600 + 47 * 60),
            window_minutes: Some(7 * 24 * 60),
        },
        ProviderQuotaWindow {
            label: "Five Hour Limit Remaining".into(),
            family: Some("Other Model Families".into()),
            used_percent: 100,
            remaining_percent: 0,
            resets_at: None,
            window_minutes: Some(5 * 60),
        },
    ];

    ProviderQuota {
        agent_id: "gemini".into(),
        provider: "google".into(),
        harness_title: "Google Antigravity CLI".into(),
        status: "ok".into(),
        detail: None,
        windows,
        fetched_at: now,
    }
}

fn opencode_quota() -> ProviderQuota {
    unavailable_quota(
        "opencode",
        "anomaly",
        "Anomaly Opencode",
        "Anomaly Opencode instance offline or unmetered local execution",
    )
}

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

#[tauri::command]
pub fn hub_get_provider_quotas() -> Result<Vec<ProviderQuota>, String> {
    Ok(vec![
        claude_quota(),
        grok_quota(),
        codex_quota(),
        gemini_quota(),
        opencode_quota(),
        llamacpp_quota(),
        ollama_quota(),
    ])
}

/// `chat` (Codex) and `grok` fetch a live process/API on every call with no
/// staleness risk, so the frontend keeps their "live quota" label and skips
/// the refresh button. Every other provider gets a "last refreshed" label and
/// a manual refresh control (some, like Claude Code and Antigravity CLI,
/// expose no official usage-budget command, so their snapshot can only be
/// updated by an explicit refresh).
#[tauri::command]
pub fn hub_refresh_provider_quota(agent_id: String) -> Result<ProviderQuota, String> {
    Ok(match agent_id.as_str() {
        "claude" => claude_quota(),
        "grok" => grok_quota(),
        "chat" | "codex" => codex_quota(),
        "gemini" => gemini_quota(),
        "opencode" => opencode_quota(),
        "llamacpp" => llamacpp_quota(),
        "ollama" => ollama_quota(),
        other => unavailable_quota(other, "unknown", other, "Unknown provider agent id"),
    })
}
