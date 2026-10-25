//! Kimi Code OAuth credential, config.toml, and token-refresh helpers for `/usage`.

use super::super::quota_codex::now_unix;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(super) const DEFAULT_GLOBAL_BASE_URL: &str = "https://api.kimi.ai/coding/v1";
pub(super) const DEFAULT_CN_BASE_URL: &str = "https://api.kimi.com/coding/v1";
const DEFAULT_GLOBAL_OAUTH_HOST: &str = "https://auth.kimi.ai";
const DEFAULT_CN_OAUTH_HOST: &str = "https://auth.kimi.com";
const KIMI_OAUTH_CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct KimiProviderConfig {
    pub base_url: Option<String>,
    pub oauth_key: Option<String>,
    pub oauth_host: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct KimiStoredToken {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub expires_at: i64,
    #[serde(default)]
    pub scope: String,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: i64,
}

fn default_token_type() -> String {
    "Bearer".into()
}

fn unquote_toml_string(raw: &str) -> Option<String> {
    let t = raw.trim();
    let s = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))?
        .trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Lightweight parser for `[providers."managed:kimi-code"]` and `.oauth` in `config.toml`.
pub(crate) fn parse_kimi_config_toml(raw: &str) -> KimiProviderConfig {
    let mut section = "";
    let mut cfg = KimiProviderConfig::default();
    for line in raw.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(h) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = h.trim();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let Some(val) = unquote_toml_string(v) else {
            continue;
        };
        match (section, k.trim()) {
            (r#"providers."managed:kimi-code""# | "providers.'managed:kimi-code'", "base_url") => {
                cfg.base_url = Some(val)
            }
            (r#"providers."managed:kimi-code""# | "providers.'managed:kimi-code'", "api_key") => {
                cfg.api_key = Some(val)
            }
            (
                r#"providers."managed:kimi-code".oauth"# | "providers.'managed:kimi-code'.oauth",
                "key",
            ) => cfg.oauth_key = Some(val),
            (
                r#"providers."managed:kimi-code".oauth"# | "providers.'managed:kimi-code'.oauth",
                "oauth_host",
            ) => cfg.oauth_host = Some(val),
            _ => {}
        }
    }
    cfg
}

pub(crate) fn resolve_token_storage_name(oauth_key: Option<&str>) -> Option<String> {
    let key = oauth_key.unwrap_or("oauth/kimi-code").trim();
    if key == "kimi-code" || key == "oauth/kimi-code" {
        return Some("kimi-code".into());
    }
    let stem = key.strip_prefix("oauth/").unwrap_or(key);
    (!stem.is_empty() && !stem.contains('/') && !stem.starts_with('.')).then(|| stem.to_string())
}

pub(super) fn find_credential_file(home: &Path, oauth_key: Option<&str>) -> Option<PathBuf> {
    let creds_dir = home.join("credentials");
    if let Some(stem) = resolve_token_storage_name(oauth_key) {
        let candidate = creds_dir.join(format!("{stem}.json"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let default_file = creds_dir.join("kimi-code.json");
    if default_file.is_file() {
        return Some(default_file);
    }
    let mut candidates: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(&creds_dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                return None;
            }
            let meta = entry.metadata().ok().filter(|m| m.is_file())?;
            Some((meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), path))
        })
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().next().map(|(_, path)| path)
}

pub(super) fn resolve_base_urls(home: &Path, cfg: &KimiProviderConfig) -> Vec<String> {
    for candidate in [
        std::env::var("KIMI_CODE_BASE_URL").ok(),
        cfg.base_url.clone(),
    ]
    .into_iter()
    .flatten()
    {
        let trimmed = candidate.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            return vec![trimmed.to_string()];
        }
    }
    let cn_first = cfg
        .oauth_host
        .as_deref()
        .map(|h| h.contains("kimi.com"))
        .unwrap_or_else(|| {
            std::fs::read_to_string(home.join("region"))
                .map(|s| s.trim().eq_ignore_ascii_case("mainland-cn"))
                .unwrap_or(false)
        });
    if cn_first {
        vec![DEFAULT_CN_BASE_URL.into(), DEFAULT_GLOBAL_BASE_URL.into()]
    } else {
        vec![DEFAULT_GLOBAL_BASE_URL.into(), DEFAULT_CN_BASE_URL.into()]
    }
}

pub(super) fn resolve_oauth_host(cfg: &KimiProviderConfig, base_url: &str) -> String {
    for candidate in [
        std::env::var("KIMI_CODE_OAUTH_HOST").ok(),
        std::env::var("KIMI_OAUTH_HOST").ok(),
        cfg.oauth_host.clone(),
    ]
    .into_iter()
    .flatten()
    {
        let trimmed = candidate.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if base_url.contains("kimi.com") {
        DEFAULT_CN_OAUTH_HOST.into()
    } else {
        DEFAULT_GLOBAL_OAUTH_HOST.into()
    }
}

pub(super) fn save_refreshed_token(path: &Path, token: &KimiStoredToken) {
    let Ok(serialized) = serde_json::to_string_pretty(token) else {
        return;
    };
    let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
    if std::fs::write(&tmp, format!("{serialized}\n")).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        let _ = std::fs::rename(&tmp, path);
    }
}

pub(super) fn refresh_oauth_token(
    client: &reqwest::blocking::Client,
    oauth_host: &str,
    refresh_token: &str,
) -> Result<KimiStoredToken, String> {
    let url = format!("{}/api/oauth/token", oauth_host.trim_end_matches('/'));
    let response = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", "kimi-code-cli/2.3.0")
        .header("X-Msh-Platform", "kimi_cli")
        .form(&[
            ("client_id", KIMI_OAUTH_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .map_err(|e| format!("Kimi OAuth token refresh failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => {
                "Kimi Code OAuth session expired or was revoked; run `kimi login` to re-authenticate"
                    .into()
            }
            _ => format!("Kimi OAuth token refresh returned HTTP {status}"),
        });
    }
    let payload: Value = response
        .json()
        .map_err(|e| format!("Unexpected Kimi OAuth refresh response: {e}"))?;
    let access_token = payload
        .get("access_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Kimi OAuth refresh response missing access_token".to_string())?;
    let next_refresh = payload
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(refresh_token);
    let expires_in = payload
        .get("expires_in")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .filter(|n| *n > 0)
        .unwrap_or(900);

    Ok(KimiStoredToken {
        access_token: access_token.into(),
        refresh_token: next_refresh.into(),
        expires_at: now_unix() + expires_in,
        scope: payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        token_type: payload
            .get("token_type")
            .and_then(Value::as_str)
            .unwrap_or("Bearer")
            .into(),
        expires_in,
    })
}
