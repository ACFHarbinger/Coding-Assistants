//! Static and aggregate provider quota commands.
use super::quota_claude::claude_quota;
use super::quota_codex::{codex_quota, unavailable_quota, ProviderQuota};
use super::quota_gemini::gemini_quota;
use super::quota_grok::grok_quota;

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

fn deepseek_quota() -> ProviderQuota {
    unavailable_quota(
        "deepseek",
        "deepseek",
        "DeepSeek via OpenCode",
        "OpenCode does not expose a DeepSeek usage-budget command",
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

#[tauri::command]
pub fn hub_get_provider_quotas() -> Result<Vec<ProviderQuota>, String> {
    Ok(vec![
        claude_quota(),
        grok_quota(),
        codex_quota(),
        gemini_quota(),
        opencode_quota(),
        deepseek_quota(),
        mistral_quota(),
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
        "deepseek" => deepseek_quota(),
        "mistral" | "vibe" => mistral_quota(),
        "llamacpp" => llamacpp_quota(),
        "ollama" => ollama_quota(),
        other => unavailable_quota(other, "unknown", other, "Unknown provider agent id"),
    })
}
