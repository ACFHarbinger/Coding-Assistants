use super::model_harness::EffectiveHarnessSettings;
use super::orchestration::{
    EffectiveOrchestrationPolicy, OrchestrationOverride, OrchestrationPolicy,
};
use super::TuiSettings;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid settings: {0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
}

/// Current on-disk schema. Unknown or missing versions fail validation.
pub const CURRENT_SETTINGS_SCHEMA: u32 = 1;
/// Default number of timestamped last-known-good backups.
pub const DEFAULT_BACKUP_RETENTION: u32 = 3;
/// Inclusive lower bound for `storage.backup_retention`.
pub const MIN_BACKUP_RETENTION: u32 = 1;
/// Inclusive upper bound for `storage.backup_retention`.
pub const MAX_BACKUP_RETENTION: u32 = 20;
/// Default number of recalled memories injected into an agent prompt.
pub const DEFAULT_MEMORY_RECALL_LIMIT: u8 = 5;
/// A small ceiling keeps recalled context useful without crowding out the task.
pub const MAX_MEMORY_RECALL_LIMIT: u8 = 20;
/// Default seconds between background provider-usage refreshes, when the
/// background refresh is enabled at all (it is off by default).
pub const DEFAULT_QUOTA_AUTO_REFRESH_SECS: u32 = 300;
/// Inclusive lower bound for `orchestration.quota_auto_refresh_interval_secs`
/// — anything tighter risks hammering a slow adapter every poll.
pub const MIN_QUOTA_AUTO_REFRESH_SECS: u32 = 30;
/// Inclusive upper bound — an hour between refreshes is already "barely on".
pub const MAX_QUOTA_AUTO_REFRESH_SECS: u32 = 3600;

/// Backend used to create memory-search embeddings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProvider {
    #[default]
    Local,
    Openai,
}

impl EmbeddingProvider {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "local" => Some(Self::Local),
            "openai" => Some(Self::Openai),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Openai => "openai",
        }
    }
}

/// Validated settings fields owned by S1. Later slices add more keys without
/// changing this load/save contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub schema_version: u32,
    pub backup_retention: u32,
    pub default_workspace: Option<String>,
    pub default_session: Option<String>,
    pub embedding_provider: EmbeddingProvider,
    pub orchestration: OrchestrationPolicy,
    pub tui: TuiSettings,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA,
            backup_retention: DEFAULT_BACKUP_RETENTION,
            default_workspace: None,
            default_session: None,
            embedding_provider: EmbeddingProvider::Local,
            orchestration: OrchestrationPolicy::default(),
            tui: TuiSettings::default(),
        }
    }
}

impl SettingsSnapshot {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.schema_version != CURRENT_SETTINGS_SCHEMA {
            return Err(SettingsError::Invalid(format!(
                "unsupported schema_version {} (expected {CURRENT_SETTINGS_SCHEMA})",
                self.schema_version
            )));
        }
        if !(MIN_BACKUP_RETENTION..=MAX_BACKUP_RETENTION).contains(&self.backup_retention) {
            return Err(SettingsError::Invalid(format!(
                "storage.backup_retention {} is outside {MIN_BACKUP_RETENTION}..={MAX_BACKUP_RETENTION}",
                self.backup_retention
            )));
        }
        self.orchestration.validate()?;
        Ok(())
    }
}

/// Whether an effective field came from the global default or a workspace
/// override (S2 / #128 scope resolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldStatus {
    Inherited,
    Override,
}

/// Fields a workspace override may set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsField {
    BackupRetention,
    DefaultWorkspace,
    DefaultSession,
    ConfirmNewEnrollment,
    ConfirmBroadcast,
    AutoEnrollmentAllowed,
    SandboxStrictness,
    RetentionDays,
    ExportEnabled,
    LinkSuggestionMode,
    MemoryRecallEnabled,
    MemoryRecallLimit,
}

/// Per-workspace overrides. Only fields present here differ from the global
/// default; an absent field means "inherited". The workspace identity is the
/// user-selected path string, kept exactly as given (not symlink-resolved),
/// so distinct paths to the same repository can carry separate overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOverride {
    pub backup_retention: Option<u32>,
    pub default_session: Option<String>,
    /// Harness id → global profile name. The workspace never copies profile
    /// fields; it only selects a named default.
    #[serde(default)]
    pub default_profiles: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub default_models: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub default_efforts: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub orchestration: OrchestrationOverride,
}

impl WorkspaceOverride {
    pub fn is_empty(&self) -> bool {
        self.backup_retention.is_none()
            && self.default_session.is_none()
            && self.default_profiles.is_empty()
            && self.default_models.is_empty()
            && self.default_efforts.is_empty()
            && self.orchestration.is_empty()
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        if let Some(retention) = self.backup_retention {
            if !(MIN_BACKUP_RETENTION..=MAX_BACKUP_RETENTION).contains(&retention) {
                return Err(SettingsError::Invalid(format!(
                    "storage.backup_retention {retention} is outside {MIN_BACKUP_RETENTION}..={MAX_BACKUP_RETENTION}"
                )));
            }
        }
        self.orchestration.validate()?;
        Ok(())
    }
}

/// Sandbox strictness for tool execution. A coarse, ordinary-tier control;
/// per-tool allow/deny lists are Advanced-tier future work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStrictness {
    Strict,
    #[default]
    Standard,
    Permissive,
}

impl SandboxStrictness {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Standard => "standard",
            Self::Permissive => "permissive",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "strict" => Some(Self::Strict),
            "standard" => Some(Self::Standard),
            "permissive" => Some(Self::Permissive),
            _ => None,
        }
    }
}

/// Whether newly written memories get candidate links to related existing
/// memories proposed automatically (M-links). `Off` means links are only
/// ever created by an explicit `link_memories` call — no proposer runs.
/// `Suggest` surfaces candidates for a human/agent to confirm before an edge
/// is written. `Auto` writes edges above a similarity/match threshold
/// immediately, attributed to `created_by = "system:auto-link"` rather than
/// whichever agent's memory triggered the suggestion, so provenance still
/// distinguishes a drawn connection from a computed one even in this mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LinkSuggestionMode {
    #[default]
    Off,
    Suggest,
    Auto,
}

impl LinkSuggestionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Suggest => "suggest",
            Self::Auto => "auto",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "suggest" => Some(Self::Suggest),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// Global defaults merged with an optional workspace override — the typed,
/// redacted shape returned to the frontend. Field-status pills let React
/// show "Inherited" vs "Workspace Override" without re-deriving the merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveSettings {
    pub schema_version: u32,
    pub workspace: Option<String>,
    pub backup_retention: u32,
    pub backup_retention_status: FieldStatus,
    pub default_workspace: Option<String>,
    pub default_workspace_status: FieldStatus,
    pub default_session: Option<String>,
    pub default_session_status: FieldStatus,
    #[serde(default)]
    pub profiles: Vec<ProfileSnapshot>,
    #[serde(default)]
    pub harnesses: Vec<EffectiveHarnessSettings>,
    pub orchestration: EffectiveOrchestrationPolicy,
    pub tui: TuiSettings,
}

/// How a profile obtains credentials. Never carries a secret value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretSourceKind {
    Keychain,
    EnvVar,
    ProviderLogin,
}

/// Stored secret *reference*. Compatible with a later OS keychain or
/// encrypted-vault backend; the settings file only keeps the kind plus an
/// opaque id or environment-variable name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SecretReference {
    Keychain { id: String },
    EnvVar { name: String },
    ProviderLogin,
}

impl SecretReference {
    pub fn kind(&self) -> SecretSourceKind {
        match self {
            Self::Keychain { .. } => SecretSourceKind::Keychain,
            Self::EnvVar { .. } => SecretSourceKind::EnvVar,
            Self::ProviderLogin => SecretSourceKind::ProviderLogin,
        }
    }

    /// Non-sensitive badge for Settings UI. Never includes a credential.
    pub fn badge(&self) -> String {
        match self {
            Self::Keychain { .. } => "Stored in System Keychain".into(),
            Self::EnvVar { name } => format!("Env Var ${name}"),
            Self::ProviderLogin => "Existing provider login".into(),
        }
    }
}

/// Global named provider profile. Fields are non-secret configuration plus
/// a secret reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub secret: SecretReference,
}

impl ProviderProfile {
    pub fn snapshot(&self) -> ProfileSnapshot {
        ProfileSnapshot {
            name: self.name.clone(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
            secret_source: self.secret.kind(),
            secret_badge: self.secret.badge(),
        }
    }
}

/// Redacted profile shown to clients. Env-var *names* are allowed; values
/// and keychain material are not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSnapshot {
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub secret_source: SecretSourceKind,
    pub secret_badge: String,
}
