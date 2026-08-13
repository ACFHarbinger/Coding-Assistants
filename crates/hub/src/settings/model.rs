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
