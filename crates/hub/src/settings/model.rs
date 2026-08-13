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

/// Validated settings fields owned by S1. Later slices add more keys without
/// changing this load/save contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub schema_version: u32,
    pub backup_retention: u32,
    pub default_workspace: Option<String>,
    pub default_session: Option<String>,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA,
            backup_retention: DEFAULT_BACKUP_RETENTION,
            default_workspace: None,
            default_session: None,
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

/// Fields a workspace override may set. One variant today; later slices add
/// more without changing the patch/reset contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsField {
    BackupRetention,
    DefaultWorkspace,
    DefaultSession,
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
}

impl WorkspaceOverride {
    pub fn is_empty(&self) -> bool {
        self.backup_retention.is_none()
            && self.default_session.is_none()
            && self.default_profiles.is_empty()
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        if let Some(retention) = self.backup_retention {
            if !(MIN_BACKUP_RETENTION..=MAX_BACKUP_RETENTION).contains(&retention) {
                return Err(SettingsError::Invalid(format!(
                    "storage.backup_retention {retention} is outside {MIN_BACKUP_RETENTION}..={MAX_BACKUP_RETENTION}"
                )));
            }
        }
        Ok(())
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

/// Global per-harness process settings. A workspace does not copy these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSettings {
    pub harness: String,
    pub executable: String,
    pub workdir: Option<String>,
    pub capture_polling: bool,
    pub inject_permission: bool,
}

impl HarnessSettings {
    pub fn default_for(harness: &str) -> Result<Self, SettingsError> {
        let id = crate::HarnessId::parse(harness)
            .map_err(|err| SettingsError::Invalid(err.to_string()))?;
        Ok(Self {
            harness: id.as_str().to_string(),
            executable: id.executable().to_string(),
            workdir: None,
            capture_polling: true,
            inject_permission: true,
        })
    }
}

/// Effective harness view: global process settings plus the workspace's
/// selected default profile, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveHarnessSettings {
    pub harness: String,
    pub executable: String,
    pub workdir: Option<String>,
    pub capture_polling: bool,
    pub inject_permission: bool,
    pub default_profile: Option<String>,
    pub default_profile_status: FieldStatus,
    pub default_profile_badge: Option<String>,
}
