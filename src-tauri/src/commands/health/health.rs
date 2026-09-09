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

/// `~/.claude/.credentials.json` → `claudeAiOauth`. Presence + expiry only;
/// the OAuth token itself is never read into a string that could be logged.
///
/// `claudeAiOauth.expiresAt` is the **short-lived access token** TTL (hours).
/// Claude Code silently refreshes that token from `refreshToken` on its own,
/// so `expiresAt` is *not* a re-login deadline — the value the user actually
/// has to act on is `refreshTokenExpiresAt` (days out). Surfacing `expiresAt`
/// made the readiness chip count down "expires in Nh" and then flip to "auth
/// expired" while the session was in fact fine.
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
    claude_health_from(installed, &json)
}

/// Testable core: decide Claude Code health from an already-parsed
/// credentials JSON without touching the filesystem.
fn claude_health_from(installed: bool, json: &serde_json::Value) -> ProviderHealth {
    let oauth = json.get("claudeAiOauth");
    let field_ms = |name: &str| {
        oauth
            .and_then(|o| o.get(name))
            .and_then(serde_json::Value::as_i64)
    };
    let has_refresh_token = oauth
        .and_then(|o| o.get("refreshToken"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|s| !s.trim().is_empty());
    let now_ms = Utc::now().timestamp_millis();

    if has_refresh_token {
        // The refresh token is what has to be renewed by hand. Report *its*
        // expiry (or none, if the field is absent) and treat the session as
        // live regardless of the access token's `expiresAt`.
        let refresh_ms = field_ms("refreshTokenExpiresAt");
        let refresh_expired = refresh_ms.is_some_and(|ms| ms <= now_ms);
        let iso = refresh_ms.and_then(|ms| epoch_secs_to_iso(ms / 1000));
        return if refresh_expired {
            health(
                &CLAUDE,
                installed,
                Some(false),
                iso,
                "Claude Code login has fully expired; run `claude` to sign in again",
            )
        } else {
            health(
                &CLAUDE,
                installed,
                Some(true),
                iso,
                "Claude Code is logged in (the access token refreshes automatically)",
            )
        };
    }

    // No refresh token stored: the access token's `expiresAt` really is the
    // deadline.
    match field_ms("expiresAt") {
        Some(ms) => {
            let iso = epoch_secs_to_iso(ms / 1000);
            let expired = ms <= now_ms;
            let detail = if expired {
                "Claude Code access token expired and no refresh token is stored; run `claude` to sign in again"
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

fn muse_health_with(installed: bool, logged_in: bool) -> ProviderHealth {
    health(
        &MUSE,
        installed,
        Some(logged_in),
        None,
        match (installed, logged_in) {
            (true, true) => "Muse Code is installed and logged in",
            (true, false) => "muse is installed but not logged in; run `muse` and sign in",
            (false, true) => "a Muse login exists but the `muse` CLI is not on PATH",
            (false, false) => "`muse` is not on PATH and no Muse login was found",
        },
    )
}

/// Muse Code stores its interactive login under
/// `$XDG_CONFIG_HOME/muse/auth.json` (default `~/.config/muse/auth.json`) as
/// a `providers.<vendor>` object per signed-in provider.
fn muse_config_dir() -> PathBuf {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => home_dir().join(".config"),
    }
    .join("muse")
}

/// True when `muse/auth.json` holds at least one signed-in provider.
fn muse_logged_in() -> bool {
    let path = muse_config_dir().join("auth.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|value| {
            value
                .get("providers")
                .and_then(serde_json::Value::as_object)
                .map(|providers| !providers.is_empty())
        })
        .unwrap_or(false)
}

/// Muse Code harness health: `muse` binary presence plus the harness's own
/// on-disk login. **Not** `MODEL_API_KEY` — that credential is for the Meta
/// Model API *inference* provider (orchestration roles), a separate concern;
/// keying harness readiness off it made a logged-in harness read as
/// "needs login" (#294 follow-up).
fn muse_health() -> ProviderHealth {
    muse_health_with(resolve_binary("muse").is_some(), muse_logged_in())
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
    local_runtime_health_multi(ids, &[bin])
}

/// Like [`local_runtime_health`] but accepts several acceptable binary
/// names — a local runtime is "installed" if **any** of them is on PATH.
/// The `detail` names what was found, or lists everything that was looked
/// for so a still-red dot is self-diagnosing.
fn local_runtime_health_multi(ids: &Ids, bins: &[&str]) -> ProviderHealth {
    let found = bins.iter().find(|bin| resolve_binary(bin).is_some());
    health(
        ids,
        found.is_some(),
        None,
        None,
        match found {
            Some(bin) => {
                format!("Local runtime binary `{bin}` is present; models run offline and unmetered")
            }
            None => format!(
                "No local runtime binary found on PATH (looked for: {})",
                bins.join(", ")
            ),
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

/// llama.cpp ships its tools under different names depending on how it was
/// installed: the current build (Homebrew, most distro packages, a CMake
/// `install`) names them `llama-server` / `llama-cli` / `llama-run`, and
/// some wrappers expose a bare `llama`. Any one of these on PATH means the
/// runtime is installed — matching against `llama-server` alone left a
/// freshly-installed llama.cpp showing a red "not installed" dot. The
/// pre-2024 names (`server`, `main`) are deliberately not probed: they are
/// too generic to match safely against an arbitrary PATH.
const LLAMACPP_BINS: &[&str] = &["llama-server", "llama-cli", "llama-run", "llama"];

fn llamacpp_health() -> ProviderHealth {
    local_runtime_health_multi(&LLAMACPP, LLAMACPP_BINS)
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
