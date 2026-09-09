//! Per-provider cheap health probes. Kept in this sibling so `health.rs`
//! stays under the 500-LoC cap (#304).
use super::super::creative_tools::resolve_binary;
use super::{binary_only, epoch_secs_to_iso, health, home_dir, not_installed, Ids, ProviderHealth};
use chrono::Utc;
use std::path::PathBuf;

// --- per-provider probes -------------------------------------------------

pub(crate) const CLAUDE: Ids = Ids {
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
pub(crate) fn claude_health() -> ProviderHealth {
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
pub(crate) fn claude_health_from(installed: bool, json: &serde_json::Value) -> ProviderHealth {
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

pub(crate) const CODEX: Ids = Ids {
    agent_id: "chat",
    provider: "openai",
    title: "OpenAI Codex",
};

/// Codex CLI keeps its login at `~/.codex/auth.json` (mode 0600). Presence
/// only — the on-disk schema is not a verified contract, so nothing is parsed.
pub(crate) fn codex_health() -> ProviderHealth {
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

pub(crate) const GEMINI: Ids = Ids {
    agent_id: "gemini",
    provider: "google",
    title: "Google Antigravity CLI",
};

/// `agy` keeps its auth inside its own state and exposes no cheap, stable
/// on-disk login marker (checked against `~/.gemini/antigravity-cli/*`), so
/// health is binary-presence only — `authenticated` stays `None`.
pub(crate) fn gemini_health() -> ProviderHealth {
    binary_only(&GEMINI, "agy")
}

pub(crate) const GROK: Ids = Ids {
    agent_id: "grok",
    provider: "xai",
    title: "xAI Grok Build",
};

/// `~/.grok/auth.json` holds a session token under an `https://…` scope key.
/// Reuse the quota adapter's extractor to decide presence; the token string
/// is dropped immediately and never surfaced.
pub(crate) fn grok_health() -> ProviderHealth {
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
pub(crate) fn grok_token_present(value: &serde_json::Value) -> bool {
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

pub(crate) const OPENCODE: Ids = Ids {
    agent_id: "opencode",
    provider: "opencode-go",
    title: "OpenCode Go",
};

/// OpenCode stores provider credentials at
/// `~/.local/share/opencode/auth.json`. Presence only.
pub(crate) fn opencode_health() -> ProviderHealth {
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

pub(crate) const DEEPSEEK: Ids = Ids {
    agent_id: "deepseek",
    provider: "deepseek",
    title: "DeepSeek",
};

/// Pure HTTP provider — no binary. `authenticated` follows whether
/// `DEEPSEEK_API_KEY` resolves through the vault or the environment. The key
/// value is never touched here, only its presence.
pub(crate) fn deepseek_health() -> ProviderHealth {
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

pub(crate) const MISTRAL: Ids = Ids {
    agent_id: "mistral",
    provider: "mistral",
    title: "Mistral Vibe",
};

pub(crate) fn mistral_health_with(
    installed: bool,
    facts: super::super::quota_vibe_usage::VibeWhoamiFacts,
) -> ProviderHealth {
    health(
        &MISTRAL,
        installed,
        Some(facts.authenticated),
        None,
        facts.detail(installed),
    )
}

/// Mistral Vibe harness health (#304): `vibe` binary plus the CLI's own
/// `whoami_cache.json`. Roster id is `mistral`; harness key remains `vibe`.
pub(crate) fn mistral_health() -> ProviderHealth {
    let installed = resolve_binary("vibe").is_some();
    let facts = super::super::quota_vibe_usage::vibe_whoami_facts();
    mistral_health_with(installed, facts)
}

pub(crate) const MUSE: Ids = Ids {
    agent_id: "muse",
    provider: "meta",
    title: "Meta Muse",
};

pub(crate) fn muse_health_with(installed: bool, logged_in: bool) -> ProviderHealth {
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
pub(crate) fn muse_config_dir() -> PathBuf {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => home_dir().join(".config"),
    }
    .join("muse")
}

/// True when `muse/auth.json` holds at least one signed-in provider.
pub(crate) fn muse_logged_in() -> bool {
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
pub(crate) fn muse_health() -> ProviderHealth {
    muse_health_with(resolve_binary("muse").is_some(), muse_logged_in())
}

pub(crate) const CURSOR: Ids = Ids {
    agent_id: "cursor",
    provider: "cursor",
    title: "Cursor Agent",
};

pub(crate) fn cursor_health_with(
    installed: bool,
    auth: super::super::quota_cursor::CursorAuthDetails,
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
pub(crate) fn cursor_health() -> ProviderHealth {
    let installed = resolve_binary("agent").is_some() || resolve_binary("cursor-agent").is_some();
    let auth = super::super::quota_cursor::cursor_auth_details();
    cursor_health_with(installed, auth)
}

pub(crate) fn local_runtime_health(ids: &Ids, bin: &str) -> ProviderHealth {
    local_runtime_health_multi(ids, &[bin])
}

/// Like [`local_runtime_health`] but accepts several acceptable binary
/// names — a local runtime is "installed" if **any** of them is on PATH.
/// The `detail` names what was found, or lists everything that was looked
/// for so a still-red dot is self-diagnosing.
pub(crate) fn local_runtime_health_multi(ids: &Ids, bins: &[&str]) -> ProviderHealth {
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

pub(crate) const OLLAMA: Ids = Ids {
    agent_id: "ollama",
    provider: "local",
    title: "Local Ollama",
};
pub(crate) const LLAMACPP: Ids = Ids {
    agent_id: "llamacpp",
    provider: "local",
    title: "Local Llama.cpp",
};

pub(crate) fn ollama_health() -> ProviderHealth {
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
pub(crate) const LLAMACPP_BINS: &[&str] = &["llama-server", "llama-cli", "llama-run", "llama"];

pub(crate) fn llamacpp_health() -> ProviderHealth {
    local_runtime_health_multi(&LLAMACPP, LLAMACPP_BINS)
}
