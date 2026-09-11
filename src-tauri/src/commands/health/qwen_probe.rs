//! Cheap Qwen Code health probe (C14.13 / #308). Split from `probes.rs`
//! for the 500-LoC cap.

use super::super::creative_tools::resolve_binary;
use super::{health, home_dir, not_installed, Ids, ProviderHealth};

pub(crate) const QWEN: Ids = Ids {
    agent_id: "qwen",
    provider: "alibaba",
    title: "Qwen Code",
};

/// Cheap Qwen Code auth: `~/.qwen/oauth_creds.json` token presence, else a
/// configured `modelProviders.*.envKey` whose env var is set. Never reads
/// the secret value into `detail`. No marker → binary-presence only.
pub(crate) fn qwen_health_with(installed: bool, auth: QwenAuth) -> ProviderHealth {
    match auth {
        QwenAuth::LoggedIn => health(
            &QWEN,
            installed,
            Some(true),
            None,
            if installed {
                "Qwen Code is installed and a login marker is present"
            } else {
                "a Qwen login marker exists but the `qwen` CLI is not on PATH"
            },
        ),
        QwenAuth::LoggedOut => health(
            &QWEN,
            installed,
            Some(false),
            None,
            if installed {
                "qwen is installed but not logged in; set the provider env key or re-login"
            } else {
                "`qwen` is not on PATH and no Qwen login was found"
            },
        ),
        QwenAuth::Unknown => {
            if installed {
                health(
                    &QWEN,
                    true,
                    None,
                    None,
                    "`qwen` is installed; run it once to confirm login",
                )
            } else {
                not_installed(&QWEN, "qwen")
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum QwenAuth {
    LoggedIn,
    LoggedOut,
    Unknown,
}

fn qwen_oauth_logged_in(json: &serde_json::Value) -> bool {
    ["access_token", "refresh_token", "token"]
        .iter()
        .any(|key| {
            json.get(*key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
        })
}

fn qwen_env_key_configured(settings: &serde_json::Value) -> Option<bool> {
    let providers = settings.get("modelProviders")?.as_object()?;
    let mut saw_key = false;
    for value in providers.values() {
        let entries = match value {
            serde_json::Value::Array(items) => items.as_slice(),
            other => std::slice::from_ref(other),
        };
        for entry in entries {
            let Some(name) = entry
                .get("envKey")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            saw_key = true;
            if std::env::var(name).is_ok_and(|value| !value.trim().is_empty()) {
                return Some(true);
            }
        }
    }
    saw_key.then_some(false)
}

pub(crate) fn qwen_auth_state() -> QwenAuth {
    let home = home_dir().join(".qwen");
    if let Ok(bytes) = std::fs::read(home.join("oauth_creds.json")) {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if qwen_oauth_logged_in(&json) {
                return QwenAuth::LoggedIn;
            }
        }
    }
    match std::fs::read(home.join("settings.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|json| qwen_env_key_configured(&json))
    {
        Some(true) => QwenAuth::LoggedIn,
        Some(false) => QwenAuth::LoggedOut,
        None => QwenAuth::Unknown,
    }
}

pub(crate) fn qwen_health() -> ProviderHealth {
    qwen_health_with(resolve_binary("qwen").is_some(), qwen_auth_state())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_json_detects_token_presence_without_reading_values_into_detail() {
        let present = serde_json::json!({"access_token": "secret-token-value"});
        assert!(qwen_oauth_logged_in(&present));
        let empty = serde_json::json!({"access_token": "  "});
        assert!(!qwen_oauth_logged_in(&empty));
        let missing = serde_json::json!({"note": "no token"});
        assert!(!qwen_oauth_logged_in(&missing));
    }

    #[test]
    fn env_key_config_reports_presence_not_the_secret() {
        let settings = serde_json::json!({
            "modelProviders": {
                "openai": [{"id": "p1", "envKey": "QWEN_PROBE_TEST_KEY"}]
            }
        });
        std::env::remove_var("QWEN_PROBE_TEST_KEY");
        assert_eq!(qwen_env_key_configured(&settings), Some(false));
        std::env::set_var("QWEN_PROBE_TEST_KEY", "not-logged");
        assert_eq!(qwen_env_key_configured(&settings), Some(true));
        std::env::remove_var("QWEN_PROBE_TEST_KEY");
        let none = serde_json::json!({"modelProviders": {}});
        assert_eq!(qwen_env_key_configured(&none), None);
    }
}
