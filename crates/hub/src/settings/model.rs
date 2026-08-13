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
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA,
            backup_retention: DEFAULT_BACKUP_RETENTION,
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
}

/// Per-workspace overrides. Only fields present here differ from the global
/// default; an absent field means "inherited". The workspace identity is the
/// user-selected path string, kept exactly as given (not symlink-resolved),
/// so distinct paths to the same repository can carry separate overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOverride {
    pub backup_retention: Option<u32>,
}

impl WorkspaceOverride {
    pub fn is_empty(&self) -> bool {
        self.backup_retention.is_none()
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
}
