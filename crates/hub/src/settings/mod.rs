//! Versioned `settings.toml` store (S1 / #127) plus workspace-override
//! scope resolution (S2 / #128).
//!
//! S4 adds global named provider profiles and validated harness process
//! settings. The desktop Settings window and its Agents tab belong to later
//! UI slices. Typed IPC and audit fan-out live in
//! `src-tauri/src/hub/commands/settings.rs` and
//! `crates/hub/src/store/policies/settings_audit.rs`.

mod model;
mod profiles;
mod store;
mod tui;
pub(crate) mod validation;

pub use model::{
    EffectiveHarnessSettings, EffectiveOrchestrationPolicy, EffectiveSettings, EmbeddingProvider,
    FieldStatus, HarnessSettings, LinkSuggestionMode, OrchestrationOverride, OrchestrationPolicy,
    ProfileSnapshot, ProviderProfile, SandboxStrictness, SecretReference, SecretSourceKind,
    SettingsError, SettingsField, SettingsSnapshot, WorkspaceOverride, CURRENT_SETTINGS_SCHEMA,
    DEFAULT_BACKUP_RETENTION, MAX_BACKUP_RETENTION, MIN_BACKUP_RETENTION,
};
pub use store::{LoadStatus, SettingsLoad, SettingsStore};
pub use tui::TuiSettings;

#[cfg(test)]
mod tests;
