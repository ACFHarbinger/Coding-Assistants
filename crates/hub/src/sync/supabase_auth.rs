//! Supabase Auth identity helpers (S11). Tokens remain vault-owned.
use super::{types::TrustedAccount, SyncError};
use crate::secret::{self, SecretString};
pub const SUPABASE_ACCESS_TOKEN_KEY: &str = "SUPABASE_ACCESS_TOKEN";
pub const SUPABASE_URL_KEY: &str = "SUPABASE_URL";
pub fn resolve_access_token() -> Option<SecretString> {
    secret::resolve(SUPABASE_ACCESS_TOKEN_KEY)
}
pub fn resolve_url() -> Option<String> {
    secret::resolve(SUPABASE_URL_KEY).map(|v| v.expose().to_string())
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
}
