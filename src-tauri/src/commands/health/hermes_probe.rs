//! Cheap Hermes Agent health probe (C14.16 / #322).
//!
//! Auth signal is `~/.hermes/auth.json`: `providers.<name>.access_token`
//! presence per provider (the active provider decides the headline state),
//! mirroring the Qwen login-marker probe. Secret values are never read into
//! `detail`. No marker → binary-presence only.

use super::super::creative_tools::resolve_binary;
use super::{health, Ids, ProviderHealth};

pub(crate) const HERMES: Ids = Ids {
    agent_id: "hermes",
    provider: "nous",
    title: "Hermes Agent",
};

/// Logged-in providers in an already-parsed `auth.json`.
pub(crate) fn logged_in_providers(auth: &serde_json::Value) -> Vec<String> {
    auth.get("providers")
        .and_then(|providers| providers.as_object())
        .map(|providers| {
            providers
                .iter()
                .filter(|(_, entry)| {
                    entry
                        .get("access_token")
                        .and_then(|token| token.as_str())
                        .is_some_and(|token| !token.trim().is_empty())
                })
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn hermes_health() -> ProviderHealth {
    let installed = resolve_binary("hermes").is_some();
    let path = hub::hermes_home_dir().join("auth.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return health(
            &HERMES,
            installed,
            None,
            None,
            if installed {
                "`hermes` is installed; no ~/.hermes/auth.json yet — run `hermes login` or use it once"
            } else {
                "`hermes` is not on PATH and no Hermes login was found"
            },
        );
    };
    let Ok(auth) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return health(
            &HERMES,
            installed,
            None,
            None,
            "~/.hermes/auth.json is present but unparseable",
        );
    };
    hermes_health_from(installed, &auth)
}

/// Testable core: decide Hermes health from an already-parsed `auth.json`
/// without touching the filesystem.
pub(crate) fn hermes_health_from(installed: bool, auth: &serde_json::Value) -> ProviderHealth {
    let providers = logged_in_providers(auth);
    if providers.is_empty() {
        return health(
            &HERMES,
            installed,
            Some(false),
            None,
            if installed {
                "hermes is installed but no provider holds an access token; log in via the Nous Portal"
            } else {
                "`hermes` is not on PATH and no Hermes login was found"
            },
        );
    }
    health(
        &HERMES,
        installed,
        Some(true),
        None,
        if installed {
            "Hermes Agent is installed and logged in"
        } else {
            "a Hermes login exists but the `hermes` CLI is not on PATH"
        },
    )
}
