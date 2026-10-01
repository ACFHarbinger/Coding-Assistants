//! Firebase Auth identity (S10 / #100).
//!
//! Resolves catalog credentials and parses Identity Toolkit / Secure Token
//! bodies. No network in tests. Presence-only [`TrustedAccount`] — no email,
//! uid, or token. Reserved identity/key-envelope recovery path stays unused
//! in v1; an Auth session never uploads or wraps the replica key.

use crate::secret::{self, SecretString};

use super::types::TrustedAccount;
use super::SyncError;

pub const REFRESH_TOKEN_KEY: &str = "FIREBASE_REFRESH_TOKEN";
pub const API_KEY: &str = "FIREBASE_API_KEY";
pub const STORAGE_BUCKET_KEY: &str = "FIREBASE_STORAGE_BUCKET";
pub const SECURE_TOKEN_URL: &str = "https://securetoken.googleapis.com/v1/token";

pub fn resolve_refresh_token() -> Option<SecretString> {
    secret::resolve(REFRESH_TOKEN_KEY)
}

pub fn resolve_api_key() -> Option<SecretString> {
    secret::resolve(API_KEY)
}

pub fn resolve_bucket() -> Option<String> {
    secret::resolve(STORAGE_BUCKET_KEY).map(|value| value.expose().to_string())
}

/// Pull `idToken` / `id_token` from a token-endpoint JSON body. The raw body
/// is not returned and must not be logged.
pub fn parse_id_token(body: &str) -> Result<SecretString, SyncError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| SyncError::Invalid("firebase token response is not json".into()))?;
    let token = value
        .get("idToken")
        .and_then(|item| item.as_str())
        .or_else(|| value.get("id_token").and_then(|item| item.as_str()))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| SyncError::Invalid("firebase token response missing id token".into()))?;
    Ok(SecretString::new(token))
}

pub fn token_request_form(refresh: &str) -> String {
    format!(
        "grant_type=refresh_token&refresh_token={}",
        form_encode(refresh)
    )
}

/// Presence-only. Token stays in the P12 vault.
pub fn firebase_account() -> Option<TrustedAccount> {
    resolve_refresh_token().map(|_| TrustedAccount {
        provider: "firebase".into(),
        label: Some("owner".into()),
        source: "auth".into(),
    })
}

fn form_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forbidden_key_name() -> String {
        format!("{}{}{}", "cloud-sync", ".", "key")
    }

    #[test]
    fn parse_id_token_does_not_surface_the_secret_in_errors() {
        let camel = parse_id_token(
            r#"{"idToken":"eyJhbGciOi.secret-id","refreshToken":"keep-out","expiresIn":"3600"}"#,
        )
        .unwrap();
        assert_eq!(camel.expose(), "eyJhbGciOi.secret-id");
        assert_eq!(format!("{camel:?}"), "SecretString(***)");
        let snake =
            parse_id_token(r#"{"id_token":"snake.secret-id","token_type":"Bearer"}"#).unwrap();
        assert_eq!(snake.expose(), "snake.secret-id");
        assert_eq!(format!("{snake:?}"), "SecretString(***)");
        let err = parse_id_token("not-json-eyJhbGciOi.secret-id").unwrap_err();
        assert!(!format!("{err:?}").contains("eyJ"));
        assert!(!format!("{err}").contains("secret-id"));
        assert!(!format!("{err}").contains("eyJhbGciOi"));
    }

    #[test]
    fn token_form_is_refresh_grant() {
        let form = token_request_form("refresh-secret");
        assert!(form.contains("grant_type=refresh_token"));
        assert!(form.contains("refresh_token=refresh-secret"));
    }

    #[test]
    fn catalog_ids_match_env_vars() {
        let refresh = crate::secret::catalog::field("tool.sync.firebase_refresh_token").unwrap();
        assert_eq!(refresh.env_var, Some(REFRESH_TOKEN_KEY));
        assert!(refresh.secret);
        let api = crate::secret::catalog::field("tool.sync.firebase_api_key").unwrap();
        assert_eq!(api.env_var, Some(API_KEY));
        assert!(api.secret);
        let bucket = crate::secret::catalog::field("tool.sync.firebase_storage_bucket").unwrap();
        assert_eq!(bucket.env_var, Some(STORAGE_BUCKET_KEY));
        assert!(!bucket.secret);
        assert_eq!(
            SECURE_TOKEN_URL,
            "https://securetoken.googleapis.com/v1/token"
        );
    }

    #[test]
    fn trusted_account_json_has_no_token_key_or_email() {
        let account = TrustedAccount {
            provider: "firebase".into(),
            label: Some("owner".into()),
            source: "auth".into(),
        };
        let json = serde_json::to_string(&account).unwrap();
        assert_eq!(account.provider, "firebase");
        assert_eq!(account.source, "auth");
        assert!(!json.contains("token"));
        assert!(!json.contains("key"));
        assert!(!json.contains("email"));
        assert!(!json.contains("uid"));
        assert!(!json.contains(&forbidden_key_name()));
        if let Some(live) = firebase_account() {
            assert_eq!(live.provider, "firebase");
            assert_eq!(live.source, "auth");
            let live_json = serde_json::to_string(&live).unwrap();
            assert!(!live_json.contains("token"));
            assert!(!live_json.contains("email"));
        }
        let src = include_str!("firebase_auth.rs");
        assert!(!src.contains(&forbidden_key_name()));
    }

    #[test]
    fn storage_rules_are_restrictive() {
        let rules = include_str!("../../../../docs/moon/firebase-storage.rules");
        assert!(rules.contains("request.auth != null"));
        assert!(rules.contains("[0-9a-f]{64}"));
        assert!(!rules.contains(&forbidden_key_name()));
        assert!(!rules.contains("allow read, write: if true"));
    }
}
