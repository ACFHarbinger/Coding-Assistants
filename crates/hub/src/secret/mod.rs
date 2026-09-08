//! Unified credential storage and resolution (`platform.md` P12, #282).
//!
//! One function reads a credential anywhere in the app: [`resolve`]. A
//! stored vault entry wins; `std::env::var` is the fallback. Callers get a
//! [`SecretString`] — redacted in `Debug`, zeroized on drop — and hand it
//! straight to an outbound `Authorization` header.
//!
//! Storage is a [`SecretBackend`]. This slice ships the OS-keychain
//! implementor ([`KeyringBackend`]); #289 adds an encrypted-file implementor
//! for machines with no secret service, plugging into the same trait.
//! `CA_SECRET_BACKEND=keychain|file` forces one; otherwise the keychain is
//! used when it is reachable and the file backend otherwise.
//!
//! Nothing here lets a secret cross an IPC boundary: [`SecretStatus`] is the
//! entire non-secret surface — presence, source, and last-updated.

pub mod catalog;
mod keyring_backend;
mod string;

pub use catalog::{field, fields_for, FieldSpec, OwnerKind, Scope, CATALOG};
pub use keyring_backend::KeyringBackend;
pub use string::SecretString;

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// OS-keychain service name for every CA credential.
const KEYCHAIN_SERVICE: &str = "coding-assistants";

/// Env var that pins the backend for tests / unusual hosts.
const BACKEND_OVERRIDE_ENV: &str = "CA_SECRET_BACKEND";

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    /// The store exists but is locked or its service is unreachable. The
    /// credential may still be set — this is not "absent".
    #[error("secret store unavailable: {0}")]
    Unavailable(String),
    /// A genuine fault talking to the store.
    #[error("secret store error: {0}")]
    Backend(String),
    #[error("invalid credential key: {0}")]
    InvalidKey(String),
}

/// Where a resolved credential came from. Crosses IPC; the value never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretSource {
    Keychain,
    File,
    EnvVar,
    None,
}

/// The complete non-secret description of one credential slot — all that a
/// frontend is ever told about a stored secret.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretStatus {
    pub key: String,
    pub source: SecretSource,
    pub is_set: bool,
    /// Unix seconds when the vault entry was last written; `None` for an
    /// env-var-backed or unset slot, or a hand-provisioned keychain entry.
    pub updated_at: Option<i64>,
}

/// A pluggable secret store. Implementors: [`KeyringBackend`] (#282) and the
/// encrypted-file backend (#289). `get` is crate-internal — a secret leaves
/// the process only as an outbound provider request.
pub trait SecretBackend: Send + Sync {
    fn kind(&self) -> SecretSource;
    /// Whether this backend can operate on this machine right now. Must not
    /// block for long or panic when the store is missing.
    fn available(&self) -> bool;
    fn set(&self, key: &str, secret: &str) -> Result<(), SecretError>;
    fn get(&self, key: &str) -> Result<Option<SecretString>, SecretError>;
    fn delete(&self, key: &str) -> Result<(), SecretError>;
    /// `(is_set, updated_at)` without returning the value.
    fn status(&self, key: &str) -> Result<(bool, Option<i64>), SecretError>;
}

/// Placeholder for the #289 encrypted-file backend. Until that lands it is
/// never selected automatically (its [`available`](SecretBackend::available)
/// is `false`); an explicit `CA_SECRET_BACKEND=file` gets an honest error
/// rather than a silent no-op.
struct UnavailableFileBackend;

impl SecretBackend for UnavailableFileBackend {
    fn kind(&self) -> SecretSource {
        SecretSource::File
    }
    fn available(&self) -> bool {
        false
    }
    fn set(&self, _key: &str, _secret: &str) -> Result<(), SecretError> {
        Err(SecretError::Unavailable(
            "encrypted-file secret backend not yet available (#289)".into(),
        ))
    }
    fn get(&self, _key: &str) -> Result<Option<SecretString>, SecretError> {
        Ok(None)
    }
    fn delete(&self, _key: &str) -> Result<(), SecretError> {
        Ok(())
    }
    fn status(&self, _key: &str) -> Result<(bool, Option<i64>), SecretError> {
        Ok((false, None))
    }
}

/// A credential key: an env-var name (`DEEPSEEK_API_KEY`) or a catalog id
/// (`provider.deepseek.api_key`). Bounded and free of separators that would
/// let it escape the keychain namespace.
pub fn validate_key(key: &str) -> Result<(), SecretError> {
    let ok = !key.is_empty()
        && key.len() <= 128
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'));
    if ok {
        Ok(())
    } else {
        Err(SecretError::InvalidKey(key.to_string()))
    }
}

/// The backend this process should use right now.
///
/// The auto-detect probe (does the OS keychain answer?) is a real
/// keychain round-trip, so its result is cached for the process lifetime:
/// a keychain that was locked or unreachable at first use needs an app
/// restart to be picked up. An explicit `CA_SECRET_BACKEND` is honoured on
/// every call and never cached.
fn active_backend() -> Box<dyn SecretBackend> {
    match std::env::var(BACKEND_OVERRIDE_ENV).ok().as_deref() {
        Some("file") => Box::new(UnavailableFileBackend),
        Some("keychain") => Box::new(KeyringBackend::new()),
        _ => {
            static KEYCHAIN_USABLE: OnceLock<bool> = OnceLock::new();
            if *KEYCHAIN_USABLE.get_or_init(|| KeyringBackend::new().available()) {
                Box::new(KeyringBackend::new())
            } else {
                Box::new(UnavailableFileBackend)
            }
        }
    }
}

/// Resolve `key` to a live credential: an active-backend entry wins, then
/// `std::env::var`. `None` when neither has a non-empty value.
pub fn resolve(key: &str) -> Option<SecretString> {
    resolve_with(active_backend().as_ref(), |k| std::env::var(k).ok(), key)
}

fn resolve_with(
    backend: &dyn SecretBackend,
    env: impl Fn(&str) -> Option<String>,
    key: &str,
) -> Option<SecretString> {
    if let Ok(Some(secret)) = backend.get(key) {
        if !secret.is_empty() {
            return Some(secret);
        }
    }
    env(key)
        .map(SecretString::new)
        .filter(|secret| !secret.is_empty())
}

/// Which source [`resolve`] would use for `key`, without fetching the value.
pub fn resolved_source(key: &str) -> SecretSource {
    let backend = active_backend();
    if matches!(backend.status(key), Ok((true, _))) {
        return backend.kind();
    }
    if std::env::var(key)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return SecretSource::EnvVar;
    }
    SecretSource::None
}

/// Store (or replace) `value` for `key` in the active backend. Callers pass
/// the value straight from IPC and drop it; it is not returned.
pub fn set_secret(key: &str, value: &str) -> Result<SecretStatus, SecretError> {
    validate_key(key)?;
    if value.trim().is_empty() {
        return Err(SecretError::InvalidKey(format!("{key}: empty value")));
    }
    let backend = active_backend();
    backend.set(key, value)?;
    Ok(status_from(backend.as_ref(), key))
}

/// Remove `key` from the active backend. The resolver then falls back to
/// `std::env::var`. Idempotent — clearing an absent key is `Ok`.
pub fn clear_secret(key: &str) -> Result<SecretStatus, SecretError> {
    validate_key(key)?;
    let backend = active_backend();
    backend.delete(key)?;
    Ok(status_from(backend.as_ref(), key))
}

/// Non-secret status for `key`. Never errors: an unreachable store reads as
/// "not set here" and the env fallback still shows through.
pub fn secret_status(key: &str) -> SecretStatus {
    if validate_key(key).is_err() {
        return SecretStatus {
            key: key.to_string(),
            source: SecretSource::None,
            is_set: false,
            updated_at: None,
        };
    }
    status_from(active_backend().as_ref(), key)
}

fn status_from(backend: &dyn SecretBackend, key: &str) -> SecretStatus {
    if let Ok((true, updated_at)) = backend.status(key) {
        return SecretStatus {
            key: key.to_string(),
            source: backend.kind(),
            is_set: true,
            updated_at,
        };
    }
    let env_set = std::env::var(key)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    SecretStatus {
        key: key.to_string(),
        source: if env_set {
            SecretSource::EnvVar
        } else {
            SecretSource::None
        },
        is_set: env_set,
        updated_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory backend so resolver tests touch neither the OS keychain nor
    /// the process environment.
    struct MockBackend {
        kind: SecretSource,
        entries: Mutex<HashMap<String, (String, Option<i64>)>>,
    }

    impl MockBackend {
        fn new(kind: SecretSource) -> Self {
            Self {
                kind,
                entries: Mutex::new(HashMap::new()),
            }
        }
        fn with(kind: SecretSource, key: &str, value: &str) -> Self {
            let backend = Self::new(kind);
            backend
                .entries
                .lock()
                .unwrap()
                .insert(key.into(), (value.into(), Some(1_725_000_000)));
            backend
        }
    }

    impl SecretBackend for MockBackend {
        fn kind(&self) -> SecretSource {
            self.kind
        }
        fn available(&self) -> bool {
            true
        }
        fn set(&self, key: &str, secret: &str) -> Result<(), SecretError> {
            self.entries
                .lock()
                .unwrap()
                .insert(key.into(), (secret.into(), Some(1_725_000_000)));
            Ok(())
        }
        fn get(&self, key: &str) -> Result<Option<SecretString>, SecretError> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .get(key)
                .map(|(v, _)| SecretString::new(v.clone())))
        }
        fn delete(&self, key: &str) -> Result<(), SecretError> {
            self.entries.lock().unwrap().remove(key);
            Ok(())
        }
        fn status(&self, key: &str) -> Result<(bool, Option<i64>), SecretError> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .get(key)
                .map(|(_, ts)| (true, *ts))
                .unwrap_or((false, None)))
        }
    }

    #[test]
    fn backend_entry_wins_over_the_environment() {
        let backend = MockBackend::with(SecretSource::Keychain, "K", "from-vault");
        let resolved = resolve_with(&backend, |_| Some("from-env".into()), "K").unwrap();
        assert_eq!(resolved.expose(), "from-vault");
    }

    #[test]
    fn environment_is_the_fallback_when_the_backend_is_empty() {
        let backend = MockBackend::new(SecretSource::Keychain);
        let resolved =
            resolve_with(&backend, |k| (k == "K").then(|| "from-env".into()), "K").unwrap();
        assert_eq!(resolved.expose(), "from-env");
    }

    #[test]
    fn a_blank_value_on_either_side_is_treated_as_absent() {
        let backend = MockBackend::with(SecretSource::Keychain, "K", "   ");
        // Blank vault entry is skipped, blank env is skipped → None.
        assert!(resolve_with(&backend, |_| Some("  ".into()), "K").is_none());
        // Blank vault entry, real env → env wins.
        let resolved = resolve_with(&backend, |_| Some("real".into()), "K").unwrap();
        assert_eq!(resolved.expose(), "real");
    }

    #[test]
    fn nothing_anywhere_resolves_to_none() {
        let backend = MockBackend::new(SecretSource::File);
        assert!(resolve_with(&backend, |_| None, "K").is_none());
    }

    #[test]
    fn key_validation_bounds_the_namespace() {
        assert!(validate_key("DEEPSEEK_API_KEY").is_ok());
        assert!(validate_key("provider.deepseek.api_key").is_ok());
        assert!(validate_key("mcp:perplexity-web").is_ok());
        assert!(validate_key("").is_err());
        assert!(validate_key("has space").is_err());
        assert!(validate_key("evil/../path").is_err());
        assert!(validate_key(&"x".repeat(129)).is_err());
    }

    #[test]
    fn unknown_key_status_is_not_set_and_never_errors() {
        // A key the process env definitely does not define.
        let status = secret_status("CA_P12_DEFINITELY_UNSET_KEY");
        assert!(!status.is_set);
        assert_eq!(status.source, SecretSource::None);
        assert!(status.updated_at.is_none());
    }

    #[test]
    fn file_backend_stub_is_never_auto_selected_and_fails_loudly_when_forced() {
        let stub = UnavailableFileBackend;
        assert!(!stub.available());
        assert!(stub.set("K", "v").is_err());
        assert!(matches!(stub.get("K"), Ok(None)));
    }
}
