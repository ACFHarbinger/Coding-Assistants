//! Catalog field types (split from `catalog.rs` to stay under the 500-LoC cap).

use serde::{Deserialize, Serialize};

/// The category of component that owns a credential or configuration field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    Harness,
    Provider,
    Tool,
    Mcp,
}

impl OwnerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Harness => "harness",
            Self::Provider => "provider",
            Self::Tool => "tool",
            Self::Mcp => "mcp",
        }
    }
}

/// The persistence and override scope of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Global,
    Workspace,
}

impl Scope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Workspace => "workspace",
        }
    }
}

/// Static description of one credential or configuration field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldSpec {
    /// Stable field identifier (e.g. `"provider.deepseek.api_key"`).
    pub id: &'static str,
    /// Human-readable label for UI rendering.
    pub display_name: &'static str,
    /// Component category owning this field.
    pub owner_kind: OwnerKind,
    /// Identifier of the specific owner (e.g. `"deepseek"`, `"muse"`, `"cursor"`, `"perplexity"`).
    pub owner_key: &'static str,
    /// Name of the environment variable checked as a fallback by the credential resolver.
    pub env_var: Option<&'static str>,
    /// `true` for secrets routed to the write-only vault; `false` for plain config values.
    pub secret: bool,
    /// Global or workspace-scoped setting.
    pub scope: Scope,
    /// Upstream documentation link for setup guidance.
    pub docs_url: Option<&'static str>,
    /// Informational copy shown in Settings UI. Never a secret.
    pub notes: Option<&'static str>,
}

impl FieldSpec {
    /// The vault key to use when reading or writing this field in [`crate::secret`].
    /// Defaults to [`Self::env_var`] if set, or [`Self::id`].
    pub const fn vault_key(&self) -> &'static str {
        match self.env_var {
            Some(var) => var,
            None => self.id,
        }
    }
}
