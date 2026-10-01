//! Supabase Auth identity helpers (S11). Tokens remain vault-owned.
use super::{types::TrustedAccount, SyncError};
use crate::secret::{self, SecretString};
pub const SUPABASE_ACCESS_TOKEN_KEY: &str = "SUPABASE_ACCESS_TOKEN";
pub const SUPABASE_URL_KEY: &str = "SUPABASE_URL";
pub const SUPABASE_STORAGE_BUCKET_KEY: &str = "SUPABASE_STORAGE_BUCKET";
pub fn resolve_access_token() -> Option<SecretString> {
    secret::resolve(SUPABASE_ACCESS_TOKEN_KEY)
}
pub fn resolve_url() -> Option<String> {
    secret::resolve(SUPABASE_URL_KEY).and_then(|value| parse_project_url(value.expose()).ok())
}
pub fn resolve_bucket() -> Option<String> {
    secret::resolve(SUPABASE_STORAGE_BUCKET_KEY).map(|value| value.expose().to_string())
}
pub fn parse_project_url(value: &str) -> Result<String, SyncError> {
    let value = value.trim();
    let host = value
        .strip_prefix("https://")
        .filter(|host| !host.is_empty())
        .ok_or_else(|| SyncError::Invalid("supabase project url must use https".into()))?;
    if host.contains(['/', '?', '#', '@', '\\']) || host.chars().any(char::is_whitespace) {
        return Err(SyncError::Invalid("supabase project url is invalid".into()));
    }
    Ok(format!("https://{host}"))
}
pub fn parse_access_token(body: &str) -> Result<SecretString, SyncError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| SyncError::Invalid("supabase auth response is not json".into()))?;
    let token = value
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| SyncError::Invalid("supabase auth response missing access token".into()))?;
    Ok(SecretString::new(token))
}
pub fn supabase_account() -> Option<TrustedAccount> {
    resolve_access_token().map(|_| TrustedAccount {
        provider: "supabase".into(),
        label: Some("owner".into()),
        source: "auth".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_parse_is_redacted() {
        let t = parse_access_token(r#"{"access_token":"secret"}"#).unwrap();
        assert_eq!(format!("{t:?}"), "SecretString(***)");
        assert!(!format!("{}", parse_access_token("secret").unwrap_err()).contains("secret"));
    }

    #[test]
    fn project_url_is_https_origin_only() {
        assert_eq!(
            parse_project_url(" https://example.supabase.co ").unwrap(),
            "https://example.supabase.co"
        );
        for invalid in [
            "http://example.supabase.co",
            "https://user@example.supabase.co",
            "https://example.supabase.co/path",
            "https://example.supabase.co?key=no",
        ] {
            assert!(parse_project_url(invalid).is_err());
        }
    }

    #[test]
    fn catalog_ids_match_env_vars() {
        let token = crate::secret::catalog::field("tool.sync.supabase_access_token").unwrap();
        assert_eq!(token.env_var, Some(SUPABASE_ACCESS_TOKEN_KEY));
        assert!(token.secret);
        let url = crate::secret::catalog::field("tool.sync.supabase_url").unwrap();
        assert_eq!(url.env_var, Some(SUPABASE_URL_KEY));
        assert!(!url.secret);
        let bucket = crate::secret::catalog::field("tool.sync.supabase_storage_bucket").unwrap();
        assert_eq!(bucket.env_var, Some(SUPABASE_STORAGE_BUCKET_KEY));
    }
}
