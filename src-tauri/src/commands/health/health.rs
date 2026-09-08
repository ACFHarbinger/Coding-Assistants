//! Typed provider **health** — the cheap `status` half of `platform.md` P3.
//!
//! The quota adapters (`commands/quota/*`) answer "how much budget is left",
//! which costs a real HTTP round-trip or a multi-second subprocess (`agy
//! --print "/usage"` alone allows 25s). That is too expensive to poll for a
//! readiness dot, and process discovery alone says nothing about whether the
//! CLI is *logged in*. This module answers the cheaper question: is the
//! provider's binary present, is a credential on disk, and — where the
//! credential carries an expiry we can read without a network call — is it
//! still valid.
//!
//! Rules, mirrored from the quota adapters:
//! - **No usage call, no long-timeout network.** `endpoint_reachable` stays
//!   `None` unless a sub-second check is explicitly wired for that provider
//!   (none are, today).
//! - **Secret hygiene.** A credential value never enters `detail`, a log, a
//!   `Debug` impl, or an error string. Only presence / expiry / reachability
//!   cross IPC.
//! - **Never panic.** Every path degrades to `installed: false` /
//!   `authenticated: None` with an actionable, secret-free `detail`.

use super::creative_tools::resolve_binary;
use chrono::{TimeZone, Utc};
use std::path::PathBuf;

/// One provider's cheap health snapshot.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderHealth {
    /// Stable agent id used across the app (`claude`, `chat`, `gemini`, …).
    pub agent_id: String,
    /// Provider/vendor slug (`anthropic`, `openai`, `google`, …).
    pub provider: String,
    /// Human-readable harness/provider title for the UI.
    pub harness_title: String,
    /// The CLI this provider needs resolves on `PATH`. `true` for pure-HTTP
    /// providers that need no local binary (e.g. DeepSeek).
    pub installed: bool,
    /// `Some(true)`  — a credential is present and (where cheaply checkable)
    ///                 unexpired.
    /// `Some(false)` — the credential is definitely missing or expired.
    /// `None`        — cannot tell without a network call / interactive login.
    pub authenticated: Option<bool>,
    /// ISO-8601 credential expiry, when the on-disk credential carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_expires_at: Option<String>,
    /// Only populated when a sub-second reachability probe is wired for the
    /// provider. `None` means "not checked", not "unreachable".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_reachable: Option<bool>,
    /// Actionable, secret-free one-liner for the UI.
    pub detail: String,
    /// ISO-8601 timestamp of this probe.
    pub checked_at: String,
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
}

/// Render a UNIX epoch (seconds) as ISO-8601, or `None` if out of range.
fn epoch_secs_to_iso(secs: i64) -> Option<String> {
    match Utc.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => Some(dt.to_rfc3339()),
        _ => None,
    }
}

struct Ids {
    agent_id: &'static str,
    provider: &'static str,
    title: &'static str,
}

/// Binary present but auth undeterminable without a network / interactive step.
fn binary_only(ids: &Ids, bin: &str) -> ProviderHealth {
    match resolve_binary(bin) {
        Some(_) => health(
            ids,
            true,
            None,
            None,
            format!("`{bin}` is installed; run it once to confirm login"),
        ),
        None => not_installed(ids, bin),
    }
}

fn not_installed(ids: &Ids, bin: &str) -> ProviderHealth {
    health(
        ids,
        false,
        Some(false),
        None,
        format!("`{bin}` was not found on PATH"),
    )
}

fn health(
    ids: &Ids,
    installed: bool,
    authenticated: Option<bool>,
    auth_expires_at: Option<String>,
    detail: impl Into<String>,
) -> ProviderHealth {
    ProviderHealth {
        agent_id: ids.agent_id.into(),
        provider: ids.provider.into(),
        harness_title: ids.title.into(),
        installed,
        authenticated,
        auth_expires_at,
        endpoint_reachable: None,
        detail: detail.into(),
        checked_at: now_iso(),
    }
}

// --- per-provider probes -------------------------------------------------

const CLAUDE: Ids = Ids {
    agent_id: "claude",
    provider: "anthropic",
    title: "Anthropic Claude Code",
};

/// `~/.claude/.credentials.json` → `claudeAiOauth.expiresAt` (epoch **ms**).
/// Presence + expiry only; the OAuth token itself is never read into a string
/// that could be logged.
fn claude_health() -> ProviderHealth {
    let installed = resolve_binary("claude").is_some();
    let path = home_dir().join(".claude").join(".credentials.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return health(
            &CLAUDE,
            installed,
            Some(false),
            None,
            "Not logged in to Claude Code (no ~/.claude/.credentials.json)",
        );
    };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return health(
            &CLAUDE,
            installed,
            None,
            None,
            "~/.claude/.credentials.json is present but unparseable",
        );
    };
    let expires_ms = json
        .get("claudeAiOauth")
        .and_then(|o| o.get("expiresAt"))
        .and_then(serde_json::Value::as_i64);
    match expires_ms {
        Some(ms) => {
            let iso = epoch_secs_to_iso(ms / 1000);
            let expired = ms <= Utc::now().timestamp_millis();
            let detail = if expired {
                "Claude Code OAuth token expired; run `claude` to refresh login"
            } else {
                "Claude Code is logged in"
            };
            health(&CLAUDE, installed, Some(!expired), iso, detail)
        }
        None => health(
            &CLAUDE,
            installed,
            Some(true),
            None,
            "Claude Code credential present (no expiry field to check)",
        ),
    }
}

const CODEX: Ids = Ids {
    agent_id: "chat",
    provider: "openai",
    title: "OpenAI Codex",
};

/// Codex CLI keeps its login at `~/.codex/auth.json` (mode 0600). Presence
/// only — the on-disk schema is not a verified contract, so nothing is parsed.
fn codex_health() -> ProviderHealth {
    let installed = resolve_binary("codex").is_some();
    if !installed {
        return not_installed(&CODEX, "codex");
    }
    let present = home_dir().join(".codex").join("auth.json").is_file();
    health(
        &CODEX,
        true,
        Some(present),
        None,
        if present {
            "codex is installed and ~/.codex/auth.json is present"
        } else {
            "codex is installed but not logged in (no ~/.codex/auth.json); run `codex login`"
        },
    )
}

const GEMINI: Ids = Ids {
    agent_id: "gemini",
    provider: "google",
    title: "Google Antigravity CLI",
};

/// `agy` keeps its auth inside its own state and exposes no cheap, stable
/// on-disk login marker (checked against `~/.gemini/antigravity-cli/*`), so
/// health is binary-presence only — `authenticated` stays `None`.
fn gemini_health() -> ProviderHealth {
    binary_only(&GEMINI, "agy")
}

const GROK: Ids = Ids {
    agent_id: "grok",
    provider: "xai",
    title: "xAI Grok Build",
};

/// `~/.grok/auth.json` holds a session token under an `https://…` scope key.
/// Reuse the quota adapter's extractor to decide presence; the token string
/// is dropped immediately and never surfaced.
fn grok_health() -> ProviderHealth {
    let installed = resolve_binary("grok").is_some();
    let path = home_dir().join(".grok").join("auth.json");
    let has_token = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .map(|v| grok_token_present(&v))
        .unwrap_or(false);
    health(
        &GROK,
        installed,
        Some(has_token),
        None,
        if has_token {
            "Grok is logged in"
        } else {
            "Not logged in to Grok (no session token in ~/.grok/auth.json); run `grok login`"
        },
    )
}

/// True when any `https://…` scope in the auth file carries a plausibly-long
/// token string. Mirrors `quota/grok.rs` without importing its private items.
fn grok_token_present(value: &serde_json::Value) -> bool {
    fn long_str(v: &serde_json::Value) -> bool {
        v.as_str().map(|s| s.len() > 16).unwrap_or(false)
    }
    let serde_json::Value::Object(map) = value else {
        return false;
    };
    for (key, scope) in map {
        if !key.starts_with("https://") {
            continue;
        }
        if long_str(scope) {
            return true;
        }
        if let serde_json::Value::Object(inner) = scope {
            for k in [
                "key",
                "access_token",
                "accessToken",
                "id_token",
                "idToken",
                "token",
                "bearer",
            ] {
                if inner.get(k).map(long_str).unwrap_or(false) {
                    return true;
                }
            }
        }
    }
    false
}

const OPENCODE: Ids = Ids {
    agent_id: "opencode",
    provider: "opencode-go",
    title: "OpenCode Go",
};

/// OpenCode stores provider credentials at
/// `~/.local/share/opencode/auth.json`. Presence only.
fn opencode_health() -> ProviderHealth {
    let installed = resolve_binary("opencode").is_some();
    if !installed {
        return not_installed(&OPENCODE, "opencode");
    }
    let present = home_dir().join(".local/share/opencode/auth.json").is_file();
    health(
        &OPENCODE,
        true,
        Some(present),
        None,
        if present {
            "opencode is installed and has stored credentials"
        } else {
            "opencode is installed but has no stored credentials; run `opencode auth login`"
        },
    )
}

const DEEPSEEK: Ids = Ids {
    agent_id: "deepseek",
    provider: "deepseek",
    title: "DeepSeek",
};

/// Pure HTTP provider — no binary. `authenticated` follows whether
/// `DEEPSEEK_API_KEY` resolves through the vault or the environment. The key
/// value is never touched here, only its presence.
fn deepseek_health() -> ProviderHealth {
    let has_key = hub::secret::resolve("DEEPSEEK_API_KEY").is_some();
    health(
        &DEEPSEEK,
        true,
        Some(has_key),
        None,
        if has_key {
            "DEEPSEEK_API_KEY is configured"
        } else {
            "DEEPSEEK_API_KEY is not set; add it in Settings → Credentials or export it"
        },
    )
}

const MISTRAL: Ids = Ids {
    agent_id: "mistral",
    provider: "mistral",
    title: "Mistral Vibe",
};

fn mistral_health() -> ProviderHealth {
    binary_only(&MISTRAL, "vibe")
}

const MUSE: Ids = Ids {
    agent_id: "muse",
    provider: "meta",
    title: "Meta Muse",
};

fn muse_health_with(installed: bool, has_key: bool) -> ProviderHealth {
    health(
        &MUSE,
        installed,
        Some(has_key),
        None,
        match (installed, has_key) {
            (true, true) => "muse is installed and MODEL_API_KEY is configured",
            (true, false) => "muse is installed but MODEL_API_KEY is not set",
            (false, true) => "MODEL_API_KEY is configured but the `muse` CLI is not on PATH",
            (false, false) => "`muse` is not on PATH and MODEL_API_KEY is not set",
        },
    )
}

/// Muse Code health branch (#294): `muse` binary presence plus `MODEL_API_KEY`
/// presence for the shared Meta bucket.
fn muse_health() -> ProviderHealth {
    let installed = resolve_binary("muse").is_some();
    let has_key = hub::secret::resolve("MODEL_API_KEY").is_some();
    muse_health_with(installed, has_key)
}

const CURSOR: Ids = Ids {
    agent_id: "cursor",
    provider: "cursor",
    title: "Cursor Agent",
};

fn cursor_health_with(
    installed: bool,
    auth: super::quota_cursor::CursorAuthDetails,
) -> ProviderHealth {
    let authenticated = if auth.token_present {
        Some(!auth.is_expired)
    } else {
        Some(false)
    };
    let detail = auth.detail(installed);
    health(
        &CURSOR,
        installed,
        authenticated,
        auth.auth_expires_at,
        detail,
    )
}

/// Cursor Agent health branch (#294): resolve the `agent`/`cursor-agent` binary
/// and check credentials via the #290-hardened reader (`CURSOR_TOKEN` or `auth.json`),
/// including JWT / on-disk auth expiry.
fn cursor_health() -> ProviderHealth {
    let installed = resolve_binary("agent").is_some() || resolve_binary("cursor-agent").is_some();
    let auth = super::quota_cursor::cursor_auth_details();
    cursor_health_with(installed, auth)
}

fn local_runtime_health(ids: &Ids, bin: &str) -> ProviderHealth {
    let installed = resolve_binary(bin).is_some();
    health(
        ids,
        installed,
        None,
        None,
        if installed {
            "Local runtime binary is present; models run offline and unmetered"
        } else {
            "Local runtime binary not found on PATH"
        },
    )
}

const OLLAMA: Ids = Ids {
    agent_id: "ollama",
    provider: "local",
    title: "Local Ollama",
};
const LLAMACPP: Ids = Ids {
    agent_id: "llamacpp",
    provider: "local",
    title: "Local Llama.cpp",
};

fn ollama_health() -> ProviderHealth {
    local_runtime_health(&OLLAMA, "ollama")
}

fn llamacpp_health() -> ProviderHealth {
    local_runtime_health(&LLAMACPP, "llama-server")
}

fn probe(agent_id: &str) -> ProviderHealth {
    match agent_id {
        "claude" => claude_health(),
        "chat" | "codex" => codex_health(),
        "gemini" => gemini_health(),
        "grok" => grok_health(),
        "opencode" => opencode_health(),
        "deepseek" => deepseek_health(),
        "mistral" | "vibe" => mistral_health(),
        "muse" => muse_health(),
        "cursor" => cursor_health(),
        "ollama" => ollama_health(),
        "llamacpp" => llamacpp_health(),
        other => ProviderHealth {
            agent_id: other.into(),
            provider: "unknown".into(),
            harness_title: other.into(),
            installed: false,
            authenticated: None,
            auth_expires_at: None,
            endpoint_reachable: None,
            detail: "Unknown provider agent id".into(),
            checked_at: now_iso(),
        },
    }
}

const ALL_AGENT_IDS: &[&str] = &[
    "claude", "grok", "chat", "cursor", "gemini", "opencode", "deepseek", "muse", "mistral",
    "llamacpp", "ollama",
];

/// Cheap by design (a few `stat`/`read` calls, no network), but still uses
/// `spawn_blocking` for parity with `hub_get_provider_quotas` and to keep
/// filesystem stalls (e.g. a slow `$HOME` on a network mount) off the IPC
/// thread.
#[tauri::command]
pub async fn hub_get_provider_health() -> Result<Vec<ProviderHealth>, String> {
    tauri::async_runtime::spawn_blocking(|| ALL_AGENT_IDS.iter().map(|id| probe(id)).collect())
        .await
        .map_err(|error| format!("provider health task panicked: {error}"))
}

#[tauri::command]
pub async fn hub_refresh_provider_health(agent_id: String) -> Result<ProviderHealth, String> {
    tauri::async_runtime::spawn_blocking(move || probe(&agent_id))
        .await
        .map_err(|error| format!("provider health refresh task panicked: {error}"))
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;
