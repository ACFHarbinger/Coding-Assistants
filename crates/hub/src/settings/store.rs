use super::model::{
    EffectiveHarnessSettings, EffectiveOrchestrationPolicy, EffectiveSettings, FieldStatus,
    HarnessSettings, LinkSuggestionMode, OrchestrationOverride, OrchestrationPolicy,
    ProfileSnapshot, ProviderProfile, SandboxStrictness, SettingsError, SettingsField,
    SettingsSnapshot, TuiSettings, WorkspaceOverride, CURRENT_SETTINGS_SCHEMA,
    DEFAULT_BACKUP_RETENTION,
};
use super::profiles::{
    default_efforts_from_table, default_models_from_table, default_profiles_from_table,
    effective_harnesses, harnesses_from_document, profiles_from_document, validate_harness,
    validate_profile, validate_profile_name, validate_provider, write_default_efforts,
    write_default_models, write_default_profiles, write_harness_fields, write_profile_fields,
};
use chrono::Utc;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};

const SETTINGS_FILE: &str = "settings.toml";
const TMP_FILE: &str = "settings.toml.tmp";
const BACKUP_DIR: &str = "settings-backups";

/// How `settings.toml` was interpreted at open time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStatus {
    Missing,
    Loaded,
    Invalid { reason: String },
    Unreadable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsLoad {
    pub path: PathBuf,
    pub status: LoadStatus,
}

/// On-disk settings file. Load never writes. Save is atomic.
pub struct SettingsStore {
    home: PathBuf,
    document: DocumentMut,
    snapshot: SettingsSnapshot,
    workspaces: BTreeMap<String, WorkspaceOverride>,
    profiles: BTreeMap<String, ProviderProfile>,
    harnesses: BTreeMap<String, HarnessSettings>,
    load: SettingsLoad,
}

mod document;
mod general;
mod orchestration;
mod persistence;
mod workspace;

use orchestration::*;
