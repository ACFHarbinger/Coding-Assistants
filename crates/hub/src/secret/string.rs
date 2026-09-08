//! A string that does not leak.
//!
//! [`SecretString`] wraps [`zeroize::Zeroizing`] so the bytes are wiped on
//! drop, prints as `SecretString(***)` in `Debug`, and has **no** `Display`,
//! `Serialize`, or `Clone` — the only way out is [`SecretString::expose`],
//! which callers use to build one outbound `Authorization` header and then
//! drop.

use std::fmt;
use zeroize::Zeroizing;

/// A credential value in memory. Never serialized, never logged.
pub struct SecretString(Zeroizing<String>);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }

    /// The raw secret. Use it to build a request header and let the borrow
    /// end immediately; do not store the `&str`, format it, or log it.
    pub fn expose(&self) -> &str {
        self.0.as_str()
    }

    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***)")
    }
}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes() == other.0.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::SecretString;

    #[test]
    fn debug_is_redacted() {
        let secret = SecretString::new("sk-live-abcdef123456");
        assert_eq!(format!("{secret:?}"), "SecretString(***)");
        assert!(!format!("{secret:#?}").contains("abcdef"));
    }

    #[test]
    fn expose_returns_the_value_and_is_empty_detects_blank() {
        assert_eq!(SecretString::new("v").expose(), "v");
        assert!(SecretString::new("   ").is_empty());
        assert!(!SecretString::new("x").is_empty());
    }
}
