//! Dropbox identity helpers (S12). Tokens remain vault-owned.

use super::{types::TrustedAccount, SyncError};
use crate::secret::{self, SecretString};

pub const DROPBOX_ACCESS_TOKEN_KEY: &str = "DROPBOX_ACCESS_TOKEN";

pub fn resolve_access_token() -> Option<SecretString> {
    secret::resolve(DROPBOX_ACCESS_TOKEN_KEY)
}

pub fn parse_access_token(body: &str) -> Result<SecretString, SyncError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| SyncError::Invalid("dropbox token response is not json".into()))?;
    let token = ["access_token", "accessToken"]
        .iter()
        .find_map(|key| value.get(*key).and_then(|item| item.as_str()))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| SyncError::Invalid("dropbox token response missing access token".into()))?;
    Ok(SecretString::new(token))
}

pub fn dropbox_account() -> Option<TrustedAccount> {
    resolve_access_token().map(|_| TrustedAccount {
        provider: "dropbox".into(),
        label: Some("owner".into()),
        source: "auth".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_parse_is_redacted() {
        let token = parse_access_token(r#"{"access_token":"secret"}"#).unwrap();
        assert_eq!(format!("{token:?}"), "SecretString(***)");
        let camel = parse_access_token(r#"{"accessToken":"secret"}"#).unwrap();
        assert_eq!(format!("{camel:?}"), "SecretString(***)");
        assert!(!format!("{}", parse_access_token("secret").unwrap_err()).contains("secret"));
    }

    #[test]
    fn catalog_ids_match_env_vars() {
        let token = crate::secret::catalog::field("tool.sync.dropbox_access_token").unwrap();
        assert_eq!(token.env_var, Some(DROPBOX_ACCESS_TOKEN_KEY));
        assert!(token.secret);
    }

    #[test]
    fn trusted_account_json_has_no_token() {
        let json = serde_json::to_string(&TrustedAccount {
            provider: "dropbox".into(),
            label: Some("owner".into()),
            source: "auth".into(),
        })
        .unwrap();
        assert!(!json.contains("token"));
        assert!(!json.contains("email"));
    }
}
