//! The OS-keychain [`SecretBackend`] — Windows Credential Manager, macOS
//! Keychain, Linux Secret Service (KWallet / GNOME Keyring) — via the
//! `keyring` crate.
//!
//! Every CA credential is one keychain entry under the service
//! [`KEYCHAIN_SERVICE`] keyed by the credential key. The stored password is
//! a small JSON [`Envelope`] carrying the secret plus a set-time timestamp,
//! so [`SecretBackend::status`] can report `updated_at` without a second
//! source of truth. A hand-provisioned entry whose value is not an envelope
//! is read as the bare secret with no timestamp.

use super::{now_unix, SecretBackend, SecretError, SecretSource, SecretString, KEYCHAIN_SERVICE};

pub struct KeyringBackend;

impl KeyringBackend {
    pub fn new() -> Self {
        Self
    }

    fn entry(key: &str) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(KEYCHAIN_SERVICE, key)
            .map_err(|error| SecretError::Backend(error.to_string()))
    }
}

impl Default for KeyringBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// JSON stored as the keychain password.
#[derive(serde::Serialize, serde::Deserialize)]
struct Envelope {
    secret: String,
    #[serde(default)]
    updated_at: Option<i64>,
}

impl Envelope {
    /// Parse a stored password. Anything that is not our JSON envelope is
    /// treated as a bare secret set outside the app (no timestamp).
    fn parse(raw: &str) -> (String, Option<i64>) {
        match serde_json::from_str::<Envelope>(raw) {
            Ok(envelope) => (envelope.secret, envelope.updated_at),
            Err(_) => (raw.to_string(), None),
        }
    }
}

/// Map a `keyring` error to ours, folding "the store is locked / unreachable"
/// into [`SecretError::Unavailable`] so callers can distinguish it from a
/// genuine backend fault.
fn map_error(error: keyring::Error) -> SecretError {
    match error {
        keyring::Error::NoStorageAccess(inner) | keyring::Error::PlatformFailure(inner) => {
            SecretError::Unavailable(inner.to_string())
        }
        other => SecretError::Backend(other.to_string()),
    }
}

impl SecretBackend for KeyringBackend {
    fn kind(&self) -> SecretSource {
        SecretSource::Keychain
    }

    fn available(&self) -> bool {
        // On Linux, `sync-secret-service` talks to the session bus. With no
        // `DBUS_SESSION_BUS_ADDRESS` (headless CI, a bare login shell) libdbus
        // would try to autolaunch one and stall; refuse early instead.
        #[cfg(target_os = "linux")]
        if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
            return false;
        }
        match Self::entry("__ca_probe__") {
            Ok(entry) => !matches!(
                entry.get_password(),
                Err(keyring::Error::NoStorageAccess(_)) | Err(keyring::Error::PlatformFailure(_))
            ),
            Err(_) => false,
        }
    }

    fn set(&self, key: &str, secret: &str) -> Result<(), SecretError> {
        let envelope = serde_json::to_string(&Envelope {
            secret: secret.to_string(),
            updated_at: Some(now_unix()),
        })
        .map_err(|error| SecretError::Backend(error.to_string()))?;
        Self::entry(key)?.set_password(&envelope).map_err(map_error)
    }

    fn get(&self, key: &str) -> Result<Option<SecretString>, SecretError> {
        match Self::entry(key)?.get_password() {
            Ok(raw) => Ok(Some(SecretString::new(Envelope::parse(&raw).0))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(other) => Err(map_error(other)),
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(other) => Err(map_error(other)),
        }
    }

    fn status(&self, key: &str) -> Result<(bool, Option<i64>), SecretError> {
        match Self::entry(key)?.get_password() {
            Ok(raw) => Ok((true, Envelope::parse(&raw).1)),
            Err(keyring::Error::NoEntry) => Ok((false, None)),
            Err(keyring::Error::NoStorageAccess(_)) | Err(keyring::Error::PlatformFailure(_)) => {
                Ok((false, None))
            }
            Err(other) => Err(map_error(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Envelope, KeyringBackend};
    use crate::secret::SecretBackend;

    #[test]
    fn envelope_round_trips_and_tolerates_a_bare_secret() {
        let json = serde_json::to_string(&Envelope {
            secret: "sk-1".into(),
            updated_at: Some(1_725_000_000),
        })
        .unwrap();
        assert_eq!(
            Envelope::parse(&json),
            ("sk-1".to_string(), Some(1_725_000_000))
        );

        // A value provisioned by hand in the OS keychain, not by this app.
        assert_eq!(
            Envelope::parse("plain-api-key"),
            ("plain-api-key".to_string(), None)
        );
    }

    #[test]
    fn available_never_panics_without_a_session_keychain() {
        // Headless CI: this must return a bool quickly, not hang or panic.
        let _ = KeyringBackend::new().available();
    }
}
