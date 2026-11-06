//! LAN TCP authentication helpers (P6 / #332).
//!
//! Decision logic is pure so unauthenticated / mismatch / unset-token cases
//! can be unit-tested without a Tauri `AppHandle` or a live socket. The
//! vault secret is resolved by the caller; this module never logs it.

use std::path::Path;

/// P12 vault / env key for the LAN TCP pairing secret (`tool.tcp.auth_token`).
pub const TOKEN_VAULT_KEY: &str = "CA_TCP_AUTH_TOKEN";

/// Why a TCP client was refused. Safe to persist in audit metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    NoServerToken,
    MissingPresented,
    Mismatch,
    UnauthenticatedCommand,
}

impl RejectReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoServerToken => "no_server_token",
            Self::MissingPresented => "missing_presented",
            Self::Mismatch => "mismatch",
            Self::UnauthenticatedCommand => "unauthenticated_command",
        }
    }
}

/// Compare a presented token against the resolved vault/env secret.
/// Empty or missing expected value fails closed.
pub fn authorize(expected: Option<&str>, presented: Option<&str>) -> Result<(), RejectReason> {
    let expected = expected.filter(|value| !value.is_empty());
    let presented = presented.filter(|value| !value.is_empty());
    let Some(expected) = expected else {
        return Err(RejectReason::NoServerToken);
    };
    let Some(presented) = presented else {
        return Err(RejectReason::MissingPresented);
    };
    if constant_time_eq(expected.as_bytes(), presented.as_bytes()) {
        Ok(())
    } else {
        Err(RejectReason::Mismatch)
    }
}

/// Audit JSON for a rejection. Peer and reason only — never the token.
pub fn audit_process_json(peer: &str, reason: RejectReason) -> String {
    serde_json::json!({
        "peer": peer,
        "reason": reason.as_str(),
    })
    .to_string()
}

/// Best-effort HubStore write for a rejected TCP client.
pub fn record_reject(peer: &str, reason: RejectReason) {
    let home = hub::default_hub_home();
    let Ok(store) = hub::HubStore::open(&home) else {
        return;
    };
    let _ = store.record_audit_event(
        &home,
        Path::new("tcp/lan"),
        "tcp.auth_rejected",
        &audit_process_json(peer, reason),
        None,
    );
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut diff = left.len() ^ right.len();
    let max = left.len().max(right.len());
    for i in 0..max {
        let a = left.get(i).copied().unwrap_or(0);
        let b = right.get(i).copied().unwrap_or(0);
        diff |= (a ^ b) as usize;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_server_token_rejects_even_when_a_client_presents_one() {
        assert_eq!(
            authorize(None, Some("anything")),
            Err(RejectReason::NoServerToken)
        );
        assert_eq!(
            authorize(Some(""), Some("anything")),
            Err(RejectReason::NoServerToken)
        );
    }

    #[test]
    fn missing_presented_token_is_rejected() {
        assert_eq!(
            authorize(Some("secret"), None),
            Err(RejectReason::MissingPresented)
        );
        assert_eq!(
            authorize(Some("secret"), Some("")),
            Err(RejectReason::MissingPresented)
        );
    }

    #[test]
    fn mismatch_is_rejected_and_match_is_accepted() {
        assert_eq!(
            authorize(Some("secret"), Some("wrong")),
            Err(RejectReason::Mismatch)
        );
        assert_eq!(
            authorize(Some("secret"), Some("secre")),
            Err(RejectReason::Mismatch)
        );
        assert_eq!(authorize(Some("secret"), Some("secret")), Ok(()));
    }

    #[test]
    fn audit_json_has_peer_and_reason_but_no_token() {
        let json = audit_process_json("10.0.0.8:5555", RejectReason::Mismatch);
        let value: serde_json::Value = serde_json::from_str(&json).expect("json");
        assert_eq!(value["peer"], "10.0.0.8:5555");
        assert_eq!(value["reason"], "mismatch");
        assert!(value.get("token").is_none());
        assert!(!json.contains("secret"));
    }

    #[test]
    fn constant_time_eq_accepts_equal_slices_only() {
        assert!(constant_time_eq(b"ab", b"ab"));
        assert!(!constant_time_eq(b"ab", b"ac"));
        assert!(!constant_time_eq(b"ab", b"abc"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(constant_time_eq(b"", b""));
    }
}
