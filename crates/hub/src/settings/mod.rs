//! Versioned `settings.toml` store (Settings S1 / #127).
//!
//! Typed IPC, workspace-override resolution, and the desktop Settings window
//! belong to later slices. This module only loads, validates, atomically
//! writes, and recovers the on-disk file.

mod model;
mod store;

pub use model::{
    SettingsError, SettingsSnapshot, CURRENT_SETTINGS_SCHEMA, DEFAULT_BACKUP_RETENTION,
    MAX_BACKUP_RETENTION, MIN_BACKUP_RETENTION,
};
pub use store::{LoadStatus, SettingsLoad, SettingsStore};

#[cfg(test)]
mod tests;
