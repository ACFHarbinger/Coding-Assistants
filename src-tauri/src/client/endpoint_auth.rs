//! Per-endpoint API key for an attached OpenAI-compatible service (#88).
//!
//! The vault slot is `endpoint.` plus a hash of the validated URL. The role
//! stores the address only. Two roles that use the same address share the
//! key. A different address is a different slot. Disconnect does not clear
//! it. The secret is never returned to the UI.

use serde::Serialize;

use crate::client::endpoint_link::validate_openai_compatible_endpoint;

const KEY_PREFIX: &str = "endpoint.";
const MAX_API_KEY_LEN: usize = 4096;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct EndpointCredentialStatus {
    pub is_set: bool,
}

/// Vault key for this exact validated address. Invalid addresses fail
/// before any vault read.
pub fn endpoint_secret_key(endpoint: &str) -> Result<String, String> {
    let endpoint = validate_openai_compatible_endpoint(endpoint)?;
    Ok(format!("{KEY_PREFIX}{:016x}", fnv1a64(endpoint.as_bytes())))
}

/// Hosted OpenAI reads `OPENAI_API_KEY`. Any other direct HTTP base,
/// including an attached endpoint, reads that address's own vault slot.
/// An attached OpenAI base therefore does not receive the hosted key.
pub fn http_api_key(
    require_hosted_openai_key: bool,
    api_base: &str,
) -> Option<hub::secret::SecretString> {
    let key = if require_hosted_openai_key {
        "OPENAI_API_KEY".to_string()
    } else {
        endpoint_secret_key(api_base).ok()?
    };
    hub::secret::resolve(&key)
}

pub fn endpoint_credential_state(endpoint: &str) -> Result<EndpointCredentialStatus, String> {
    let key = endpoint_secret_key(endpoint)?;
    Ok(EndpointCredentialStatus {
        is_set: hub::secret::secret_status(&key).is_set,
    })
}

pub fn save_endpoint_api_key(
    endpoint: &str,
    api_key: &str,
) -> Result<EndpointCredentialStatus, String> {
    let key = endpoint_secret_key(endpoint)?;
    if !acceptable_api_key(api_key) {
        return Err("Enter an API key without line breaks, or clear the saved key.".to_string());
    }
    hub::secret::set_secret(&key, api_key.trim())
        .map_err(|_| "Could not save the API key.".to_string())?;
    endpoint_credential_state(endpoint)
}

pub fn clear_endpoint_api_key(endpoint: &str) -> Result<EndpointCredentialStatus, String> {
    let key = endpoint_secret_key(endpoint)?;
    hub::secret::clear_secret(&key).map_err(|_| "Could not clear the API key.".to_string())?;
    endpoint_credential_state(endpoint)
}

fn acceptable_api_key(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= MAX_API_KEY_LEN
        && !value
            .chars()
            .any(|character| character == '\0' || character == '\n' || character == '\r')
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[tauri::command]
pub fn endpoint_credential_status(endpoint: String) -> Result<EndpointCredentialStatus, String> {
    endpoint_credential_state(&endpoint)
}

#[tauri::command]
pub fn endpoint_credential_set(
    endpoint: String,
    api_key: String,
) -> Result<EndpointCredentialStatus, String> {
    save_endpoint_api_key(&endpoint, &api_key)
}

#[tauri::command]
pub fn endpoint_credential_clear(endpoint: String) -> Result<EndpointCredentialStatus, String> {
    clear_endpoint_api_key(&endpoint)
}

#[cfg(test)]
pub(crate) fn tests_env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::endpoint_link::ENDPOINT_URL_ERROR;

    struct HomeGuard {
        previous_home: Option<std::ffi::OsString>,
        previous_backend: Option<std::ffi::OsString>,
        previous_openai: Option<std::ffi::OsString>,
    }

    impl HomeGuard {
        fn file_vault(dir: &std::path::Path) -> Self {
            let guard = Self {
                previous_home: std::env::var_os("CA_HOME"),
                previous_backend: std::env::var_os("CA_SECRET_BACKEND"),
                previous_openai: std::env::var_os("OPENAI_API_KEY"),
            };
            std::env::set_var("CA_HOME", dir);
            std::env::set_var("CA_SECRET_BACKEND", "file");
            std::env::remove_var("OPENAI_API_KEY");
            guard
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            restore("CA_HOME", self.previous_home.as_deref());
            restore("CA_SECRET_BACKEND", self.previous_backend.as_deref());
            restore("OPENAI_API_KEY", self.previous_openai.as_deref());
        }
    }

    fn restore(name: &str, value: Option<&std::ffi::OsStr>) {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }

    #[test]
    fn the_same_address_shares_one_slot_and_another_address_does_not() {
        let _lock = tests_env_lock();
        let dir = tempfile::tempdir().unwrap();
        let _home = HomeGuard::file_vault(dir.path());
        let first = "http://127.0.0.1:1234/v1";
        let second = "http://127.0.0.1:1235/v1";
        assert_ne!(
            endpoint_secret_key(first).unwrap(),
            endpoint_secret_key(second).unwrap()
        );
        assert_eq!(
            endpoint_secret_key(first).unwrap(),
            endpoint_secret_key("  http://127.0.0.1:1234/v1  ").unwrap()
        );
        assert!(!endpoint_credential_state(first).unwrap().is_set);
        let saved = save_endpoint_api_key(first, "  role-a-key  ").unwrap();
        assert!(saved.is_set);
        assert!(endpoint_credential_state(first).unwrap().is_set);
        assert!(!endpoint_credential_state(second).unwrap().is_set);
        let resolved = http_api_key(false, first).unwrap();
        assert_eq!(resolved.expose(), "role-a-key");
        assert!(http_api_key(false, second).is_none());
        std::env::set_var("OPENAI_API_KEY", "hosted-secret");
        let hosted = http_api_key(true, first).unwrap();
        assert_eq!(hosted.expose(), "hosted-secret");
        assert_ne!(hosted.expose(), resolved.expose());
        assert!(!clear_endpoint_api_key(first).unwrap().is_set);
        assert!(http_api_key(false, first).is_none());
    }

    #[test]
    fn rejected_keys_and_urls_do_not_echo_the_secret() {
        let _lock = tests_env_lock();
        let dir = tempfile::tempdir().unwrap();
        let _home = HomeGuard::file_vault(dir.path());
        let secret = "super-secret-value";
        let broken = format!("bad\n{secret}");
        let error = save_endpoint_api_key("http://127.0.0.1:9/v1", &broken).unwrap_err();
        assert!(!error.contains(secret));
        assert!(!error.contains('\n'));
        let url_error =
            save_endpoint_api_key("http://user:secret@127.0.0.1/v1", secret).unwrap_err();
        assert_eq!(url_error, ENDPOINT_URL_ERROR);
        assert!(!url_error.contains(secret));
        assert!(save_endpoint_api_key("http://127.0.0.1:9/v1", "   ").is_err());
        assert!(
            !endpoint_credential_state("http://127.0.0.1:9/v1")
                .unwrap()
                .is_set
        );
    }
}
