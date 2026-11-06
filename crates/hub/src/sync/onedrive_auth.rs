//! OneDrive identity helpers (S12). Tokens remain vault-owned.

use super::{types::TrustedAccount, SyncError};
use crate::secret::{self, SecretString};

pub const ONEDRIVE_REFRESH_TOKEN_KEY: &str = "ONEDRIVE_REFRESH_TOKEN";
pub const ONEDRIVE_CLIENT_ID_KEY: &str = "ONEDRIVE_CLIENT_ID";
pub const TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
pub const APP_FOLDER_SCOPE: &str = "offline_access Files.ReadWrite.AppFolder";

pub fn resolve_refresh_token() -> Option<SecretString> {
    secret::resolve(ONEDRIVE_REFRESH_TOKEN_KEY)
}

pub fn resolve_client_id() -> Option<String> {
    secret::resolve(ONEDRIVE_CLIENT_ID_KEY).map(|value| value.expose().trim().to_string())
}

pub fn parse_access_token(body: &str) -> Result<SecretString, SyncError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| SyncError::Invalid("onedrive token response is not json".into()))?;
    let token = ["access_token", "accessToken"]
        .iter()
        .find_map(|key| value.get(*key).and_then(|item| item.as_str()))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| SyncError::Invalid("onedrive token response missing access token".into()))?;
    Ok(SecretString::new(token))
}

pub fn token_request_form(refresh: &str, client_id: &str) -> String {
    format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}&scope={}",
        form_encode(refresh),
        form_encode(client_id),
        form_encode(APP_FOLDER_SCOPE)
    )
}

pub fn onedrive_account() -> Option<TrustedAccount> {
    resolve_refresh_token().map(|_| TrustedAccount {
        provider: "onedrive".into(),
        label: Some("owner".into()),
        source: "auth".into(),
    })
}

fn form_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
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

    #[test]
    fn token_parse_is_redacted() {
        let token = parse_access_token(r#"{"access_token":"secret"}"#).unwrap();
        assert_eq!(format!("{token:?}"), "SecretString(***)");
        let camel = parse_access_token(r#"{"accessToken":"secret"}"#).unwrap();
        assert_eq!(format!("{camel:?}"), "SecretString(***)");
        assert!(!format!("{}", parse_access_token("secret").unwrap_err()).contains("secret"));
    }

    #[test]
    fn token_form_is_refresh_grant_without_wrapping_the_replica_key() {
        let form = token_request_form("refresh-secret", "client-id");
        assert!(form.contains("grant_type=refresh_token"));
        assert!(form.contains("Files.ReadWrite.AppFolder"));
        assert!(!form.contains("cloud-sync"));
        assert_eq!(
            TOKEN_URL,
            "https://login.microsoftonline.com/common/oauth2/v2.0/token"
        );
    }

    #[test]
    fn catalog_ids_match_env_vars() {
        let refresh = crate::secret::catalog::field("tool.sync.onedrive_refresh_token").unwrap();
        assert_eq!(refresh.env_var, Some(ONEDRIVE_REFRESH_TOKEN_KEY));
        assert!(refresh.secret);
        let client = crate::secret::catalog::field("tool.sync.onedrive_client_id").unwrap();
        assert_eq!(client.env_var, Some(ONEDRIVE_CLIENT_ID_KEY));
        assert!(!client.secret);
    }

    #[test]
    fn trusted_account_json_has_no_token() {
        let json = serde_json::to_string(&TrustedAccount {
            provider: "onedrive".into(),
            label: Some("owner".into()),
            source: "auth".into(),
        })
        .unwrap();
        assert!(!json.contains("token"));
        assert!(!json.contains("email"));
    }
}
