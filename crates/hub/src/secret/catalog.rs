//! Typed static catalog of harness, tool, MCP, and provider credential/config fields (#285).
//!
//! Provides metadata for Settings UI rendering and Settings write command validation
//! (`field_id` validation in #283, Settings surface in #284, resolver integration in #287).
//!
//! Mirrors [`hub::mcp::external::CATALOG`](crate::mcp::external::CATALOG). Pure data + accessors,
//! no I/O, no secret reads.

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

/// Static registry of known credential and configuration fields across all providers,
/// harnesses, tools, and MCP servers.
pub const CATALOG: &[FieldSpec] = &[
    // --- Providers ---
    FieldSpec {
        id: "provider.deepseek.api_key",
        display_name: "DeepSeek API Key",
        owner_kind: OwnerKind::Provider,
        owner_key: "deepseek",
        env_var: Some("DEEPSEEK_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://platform.deepseek.com/api_keys"),
        notes: Some("API key for DeepSeek quota balance checks and direct inference."),
    },
    FieldSpec {
        id: "provider.deepseek.base_url",
        display_name: "DeepSeek Base URL",
        owner_kind: OwnerKind::Provider,
        owner_key: "deepseek",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: Some("https://api-docs.deepseek.com"),
        notes: Some("Base URL for DeepSeek API requests (default: https://api.deepseek.com)."),
    },
    FieldSpec {
        id: "provider.deepseek.model",
        display_name: "DeepSeek Model",
        owner_kind: OwnerKind::Provider,
        owner_key: "deepseek",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default DeepSeek model (default: deepseek/deepseek-v4-flash)."),
    },
    FieldSpec {
        id: "provider.muse.api_key",
        display_name: "Meta Model API Key",
        owner_kind: OwnerKind::Provider,
        owner_key: "muse",
        env_var: Some("MODEL_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://api.meta.ai"),
        notes: Some(
            "Bearer token for Meta Model API (LLM|... format), used for Muse Spark completions.",
        ),
    },
    FieldSpec {
        id: "provider.muse.base_url",
        display_name: "Meta Model API Base URL",
        owner_kind: OwnerKind::Provider,
        owner_key: "muse",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Endpoint for Meta Model API (default: https://api.meta.ai/v1)."),
    },
    FieldSpec {
        id: "provider.muse.model",
        display_name: "Meta Muse Model",
        owner_kind: OwnerKind::Provider,
        owner_key: "muse",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default model for Meta Model API (default: muse-spark-1.3)."),
    },
    FieldSpec {
        id: "provider.openai.api_key",
        display_name: "OpenAI API Key",
        owner_kind: OwnerKind::Provider,
        owner_key: "openai",
        env_var: Some("OPENAI_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://platform.openai.com/api-keys"),
        notes: Some("API key for OpenAI models and memory embeddings."),
    },
    FieldSpec {
        id: "provider.openai.base_url",
        display_name: "OpenAI Base URL",
        owner_kind: OwnerKind::Provider,
        owner_key: "openai",
        env_var: Some("CA_MEMORY_EMBEDDING_API_BASE"),
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some(
            "Endpoint base URL for OpenAI-compatible APIs (default: https://api.openai.com/v1).",
        ),
    },
    FieldSpec {
        id: "provider.openai.model",
        display_name: "OpenAI Model",
        owner_kind: OwnerKind::Provider,
        owner_key: "openai",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default model for OpenAI completions (default: gpt-4o)."),
    },
    FieldSpec {
        id: "provider.gemini.api_key",
        display_name: "Gemini API Key",
        owner_kind: OwnerKind::Provider,
        owner_key: "gemini",
        env_var: Some("GEMINI_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://aistudio.google.com/app/apikey"),
        notes: Some("API key for Google Gemini / Google AI Studio."),
    },
    FieldSpec {
        id: "provider.gemini.model",
        display_name: "Gemini Model",
        owner_kind: OwnerKind::Provider,
        owner_key: "gemini",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default model for Gemini (default: gemini-3.7-flash-medium)."),
    },
    FieldSpec {
        id: "provider.grok.api_key",
        display_name: "Grok API Key",
        owner_kind: OwnerKind::Provider,
        owner_key: "grok",
        env_var: Some("XAI_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://console.x.ai/"),
        notes: Some("API key for xAI Grok completions (separate from Grok CLI login session)."),
    },
    FieldSpec {
        id: "provider.grok.model",
        display_name: "Grok Model",
        owner_kind: OwnerKind::Provider,
        owner_key: "grok",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default model for Grok (default: grok-4.6)."),
    },
    FieldSpec {
        id: "provider.mistral.api_key",
        display_name: "Mistral API Key",
        owner_kind: OwnerKind::Provider,
        owner_key: "mistral",
        env_var: Some("MISTRAL_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://console.mistral.ai/api-keys/"),
        notes: Some("API key for Mistral models and Vibe CLI authentication."),
    },
    FieldSpec {
        id: "provider.mistral.model",
        display_name: "Mistral Model",
        owner_kind: OwnerKind::Provider,
        owner_key: "mistral",
        env_var: Some("VIBE_ACTIVE_MODEL"),
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default model for Mistral Vibe (default: mistral-medium-3.5)."),
    },
    FieldSpec {
        id: "provider.anthropic.api_key",
        display_name: "Anthropic API Key",
        owner_kind: OwnerKind::Provider,
        owner_key: "anthropic",
        env_var: Some("ANTHROPIC_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://console.anthropic.com/settings/keys"),
        notes: Some("API key for Anthropic Claude completions (separate from Claude Code login)."),
    },
    FieldSpec {
        id: "provider.anthropic.model",
        display_name: "Anthropic Model",
        owner_kind: OwnerKind::Provider,
        owner_key: "anthropic",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default Claude model (default: claude-3-7-sonnet-20250219)."),
    },
    FieldSpec {
        id: "provider.opencode.model",
        display_name: "OpenCode Model",
        owner_kind: OwnerKind::Provider,
        owner_key: "opencode",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default model for OpenCode (default: opencode-go/glm-5.3)."),
    },
    // --- MCP ---
    FieldSpec {
        id: "mcp.perplexity.api_key",
        display_name: "Perplexity API Key",
        owner_kind: OwnerKind::Mcp,
        owner_key: "perplexity",
        env_var: Some("PERPLEXITY_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://github.com/perplexityai/modelcontextprotocol"),
        notes: Some(
            "API key for official Perplexity MCP server. Stored securely in vault or read from environment.",
        ),
    },
    FieldSpec {
        id: "mcp.perplexity_web.session_login",
        display_name: "Perplexity Web Session Login",
        owner_kind: OwnerKind::Mcp,
        owner_key: "perplexity-web",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: Some("https://github.com/jacob-bd/perplexity-web-mcp"),
        notes: Some(
            "Interactive session login for Perplexity subscription (`pwm login`). Token stored at ~/.config/perplexity-web-mcp/token.",
        ),
    },
    // --- Harnesses ---
    FieldSpec {
        id: "harness.cursor.login_token",
        display_name: "Cursor Login Token",
        owner_kind: OwnerKind::Harness,
        owner_key: "cursor",
        env_var: Some("CURSOR_TOKEN"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://www.cursor.com"),
        notes: Some("Authentication token for the Cursor agent CLI."),
    },
    FieldSpec {
        id: "harness.muse.model",
        display_name: "Muse Code Model",
        owner_kind: OwnerKind::Harness,
        owner_key: "muse",
        env_var: None,
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Default model passed to `muse exec --model`."),
    },
    FieldSpec {
        id: "harness.muse.api_key",
        display_name: "Muse Code API Key",
        owner_kind: OwnerKind::Harness,
        owner_key: "muse",
        env_var: Some("MODEL_API_KEY"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://api.meta.ai"),
        notes: Some(
            "Model API key shared between Muse Spark provider and Muse Code harness.",
        ),
    },
    // --- Tools ---
    FieldSpec {
        id: "tool.memory.embedding_provider",
        display_name: "Memory Embedding Provider",
        owner_kind: OwnerKind::Tool,
        owner_key: "memory",
        env_var: Some("CA_MEMORY_EMBEDDING_PROVIDER"),
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some(
            "Embedding model provider for semantic memory recall (e.g. fastembed, openai).",
        ),
    },
    FieldSpec {
        id: "tool.memory.embedding_api_base",
        display_name: "Memory Embedding API Base",
        owner_kind: OwnerKind::Tool,
        owner_key: "memory",
        env_var: Some("CA_MEMORY_EMBEDDING_API_BASE"),
        secret: false,
        scope: Scope::Global,
        docs_url: None,
        notes: Some("Custom endpoint URL when using OpenAI-compatible embeddings."),
    },
];

/// Look up a field in the catalog by its unique id.
pub fn field(id: &str) -> Option<&'static FieldSpec> {
    CATALOG.iter().find(|f| f.id == id)
}

/// All fields belonging to the specified owner.
pub fn fields_for(owner_kind: OwnerKind, owner_key: &str) -> Vec<&'static FieldSpec> {
    CATALOG
        .iter()
        .filter(|f| f.owner_kind == owner_kind && f.owner_key == owner_key)
        .collect()
}

/// All fields belonging to the specified owner category.
pub fn fields_by_owner_kind(owner_kind: OwnerKind) -> Vec<&'static FieldSpec> {
    CATALOG
        .iter()
        .filter(|f| f.owner_kind == owner_kind)
        .collect()
}

/// All fields marked as secret (routed to write-only vault).
pub fn secret_fields() -> Vec<&'static FieldSpec> {
    CATALOG.iter().filter(|f| f.secret).collect()
}

/// Look up a field by its environment variable name.
pub fn field_by_env_var(env_var: &str) -> Option<&'static FieldSpec> {
    CATALOG.iter().find(|f| f.env_var == Some(env_var))
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
