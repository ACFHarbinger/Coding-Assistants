//! Google Drive OAuth refresh (S3 / #93).
//!
//! Browser loopback is S4. This module only resolves a stored refresh token
//! and parses a token endpoint body. Values never go in `Debug` or errors.

use crate::secret::{self, SecretString};

use super::SyncError;

pub const REFRESH_TOKEN_KEY: &str = "GOOGLE_DRIVE_REFRESH_TOKEN";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const DRIVE_APPDATA_SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";

pub fn resolve_refresh_token() -> Option<SecretString> {
    secret::resolve(REFRESH_TOKEN_KEY)
}

/// Pull `access_token` out of a token-endpoint JSON body. The raw body is not
/// returned and must not be logged.
pub fn parse_access_token(body: &str) -> Result<SecretString, SyncError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| SyncError::Invalid("google token response is not json".into()))?;
    let token = value
        .get("access_token")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| SyncError::Invalid("google token response missing access_token".into()))?;
    Ok(SecretString::new(token))
}

pub fn token_request_form(refresh: &str, client_id: &str, client_secret: &str) -> String {
    format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}&client_secret={}",
        form_encode(refresh),
        form_encode(client_id),
        form_encode(client_secret)
    )
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

    #[test]
    fn parse_access_token_does_not_surface_the_secret_in_errors() {
        let token = parse_access_token(
            r#"{"access_token":"ya29.secret-access","token_type":"Bearer","expires_in":3600}"#,
        )
        .unwrap();
        assert_eq!(token.expose(), "ya29.secret-access");
        assert_eq!(format!("{token:?}"), "SecretString(***)");
        let err = parse_access_token("not-json").unwrap_err();
        assert!(!format!("{err:?}").contains("ya29"));
        assert!(!format!("{err}").contains("ya29"));
    }

    #[test]
    fn token_form_includes_refresh_grant_but_debug_of_result_is_the_caller_problem() {
        let form = token_request_form("refresh-secret", "client", "cs");
        assert!(form.contains("grant_type=refresh_token"));
        assert!(form.contains("refresh-secret"));
    }

    #[test]
    fn refresh_token_key_matches_catalog() {
        let field = crate::secret::catalog::field("tool.sync.google_refresh_token").unwrap();
        assert_eq!(field.env_var, Some(REFRESH_TOKEN_KEY));
        assert!(field.secret);
        assert_eq!(TOKEN_URL, "https://oauth2.googleapis.com/token");
        assert_eq!(
            DRIVE_APPDATA_SCOPE,
            "https://www.googleapis.com/auth/drive.appdata"
        );
    }
}
