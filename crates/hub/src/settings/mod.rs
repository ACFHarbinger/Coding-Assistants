//! Versioned `settings.toml` store (S1 / #127) plus workspace-override
//! scope resolution (S2 / #128).
//!
//! The desktop Settings window and its real business fields (theme,
//! provider profiles, etc.) belong to later slices; this module loads,
//! validates, atomically writes, and recovers the on-disk file, and merges
//! global defaults with a workspace override deterministically. Typed IPC
//! and audit fan-out live in `src-tauri/src/hub/commands/settings.rs` and
//! `crates/hub/src/store/policies/settings_audit.rs`.

mod model;
mod store;

pub use model::{
    EffectiveSettings, FieldStatus, SettingsError, SettingsField, SettingsSnapshot,
    WorkspaceOverride, CURRENT_SETTINGS_SCHEMA, DEFAULT_BACKUP_RETENTION, MAX_BACKUP_RETENTION,
    MIN_BACKUP_RETENTION,
};
pub use store::{LoadStatus, SettingsLoad, SettingsStore};

#[cfg(test)]
mod tests;
