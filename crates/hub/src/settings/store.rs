use super::model::{
    EffectiveHarnessSettings, EffectiveOrchestrationPolicy, EffectiveSettings, FieldStatus,
    HarnessSettings, OrchestrationOverride, OrchestrationPolicy, ProfileSnapshot, ProviderProfile,
    SandboxStrictness, SettingsError, SettingsField, SettingsSnapshot, TuiSettings,
    WorkspaceOverride, CURRENT_SETTINGS_SCHEMA, DEFAULT_BACKUP_RETENTION,
};
use super::profiles::{
    default_profiles_from_table, effective_harnesses, harnesses_from_document,
    profiles_from_document, validate_harness, validate_profile, validate_profile_name,
    validate_provider, write_default_profiles, write_harness_fields, write_profile_fields,
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

impl SettingsStore {
    pub fn open(home: impl AsRef<Path>) -> Self {
        let home = home.as_ref().to_path_buf();
        let path = home.join(SETTINGS_FILE);
        match fs::read_to_string(&path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Self {
                home,
                document: default_document(),
                snapshot: SettingsSnapshot::default(),
                workspaces: BTreeMap::new(),
                profiles: BTreeMap::new(),
                harnesses: BTreeMap::new(),
                load: SettingsLoad {
                    path,
                    status: LoadStatus::Missing,
                },
            },
            Err(err) => Self {
                home,
                document: default_document(),
                snapshot: SettingsSnapshot::default(),
                workspaces: BTreeMap::new(),
                profiles: BTreeMap::new(),
                harnesses: BTreeMap::new(),
                load: SettingsLoad {
                    path,
                    status: LoadStatus::Unreadable {
                        reason: err.to_string(),
                    },
                },
            },
            Ok(raw) => match parse_document(&raw) {
                Ok((document, snapshot, workspaces, profiles, harnesses)) => Self {
                    home,
                    document,
                    snapshot,
                    workspaces,
                    profiles,
                    harnesses,
                    load: SettingsLoad {
                        path,
                        status: LoadStatus::Loaded,
                    },
                },
                Err(err) => Self {
                    home,
                    document: default_document(),
                    snapshot: SettingsSnapshot::default(),
                    workspaces: BTreeMap::new(),
                    profiles: BTreeMap::new(),
                    harnesses: BTreeMap::new(),
                    load: SettingsLoad {
                        path,
                        status: LoadStatus::Invalid {
                            reason: err.to_string(),
                        },
                    },
                },
            },
        }
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn path(&self) -> &Path {
        &self.load.path
    }

    pub fn load(&self) -> &SettingsLoad {
        &self.load
    }

    pub fn snapshot(&self) -> &SettingsSnapshot {
        &self.snapshot
    }

    pub fn workspace_override(&self, workspace: &str) -> Option<&WorkspaceOverride> {
        self.workspaces.get(workspace)
    }

    /// Merge the global snapshot with `workspace`'s override, if any. `None`
    /// resolves the global-only view. The caller decides which workspace
    /// identity string to pass; this never resolves symlinks itself.
    pub fn effective(&self, workspace: Option<&str>) -> EffectiveSettings {
        let over = workspace.and_then(|w| self.workspaces.get(w));
        let (backup_retention, backup_retention_status) =
            match over.and_then(|o| o.backup_retention) {
                Some(value) => (value, FieldStatus::Override),
                None => (self.snapshot.backup_retention, FieldStatus::Inherited),
            };
        let (default_session, default_session_status) =
            match over.and_then(|o| o.default_session.clone()) {
                Some(value) => (Some(value), FieldStatus::Override),
                None => (
                    self.snapshot.default_session.clone(),
                    FieldStatus::Inherited,
                ),
            };
        let profiles = self
            .profiles
            .values()
            .map(ProviderProfile::snapshot)
            .collect();
        let harnesses = effective_harnesses(
            &self.harnesses,
            &self.profiles,
            over.map(|o| &o.default_profiles),
        );
        let orchestration =
            effective_orchestration(&self.snapshot.orchestration, over.map(|o| &o.orchestration));
        EffectiveSettings {
            schema_version: self.snapshot.schema_version,
            workspace: workspace.map(str::to_string),
            backup_retention,
            backup_retention_status,
            default_workspace: self.snapshot.default_workspace.clone(),
            default_workspace_status: FieldStatus::Inherited,
            default_session,
            default_session_status,
            profiles,
            harnesses,
            orchestration,
            tui: self.snapshot.tui.clone(),
        }
    }

    pub fn set_backup_retention(&mut self, retention: u32) -> Result<(), SettingsError> {
        let mut next = self.snapshot.clone();
        next.backup_retention = retention;
        next.validate()?;
        self.snapshot = next;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_default_workspace(&mut self, workspace: Option<&str>) -> Result<(), SettingsError> {
        let mut next = self.snapshot.clone();
        next.default_workspace = workspace.map(str::to_string);
        next.validate()?;
        self.snapshot = next;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_default_session(&mut self, session: Option<&str>) -> Result<(), SettingsError> {
        let mut next = self.snapshot.clone();
        next.default_session = session.map(str::to_string);
        next.validate()?;
        self.snapshot = next;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_confirm_new_enrollment(&mut self, value: bool) -> Result<(), SettingsError> {
        self.snapshot.orchestration.confirm_new_enrollment = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_confirm_broadcast(&mut self, value: bool) -> Result<(), SettingsError> {
        self.snapshot.orchestration.confirm_broadcast = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_auto_enrollment_allowed(&mut self, value: bool) -> Result<(), SettingsError> {
        self.snapshot.orchestration.auto_enrollment_allowed = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_sandbox_strictness(
        &mut self,
        value: SandboxStrictness,
    ) -> Result<(), SettingsError> {
        self.snapshot.orchestration.sandbox_strictness = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_retention_days(&mut self, days: Option<u32>) -> Result<(), SettingsError> {
        let mut next = self.snapshot.orchestration.clone();
        next.retention_days = days;
        next.validate()?;
        self.snapshot.orchestration = next;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_export_enabled(&mut self, value: bool) -> Result<(), SettingsError> {
        self.snapshot.orchestration.export_enabled = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_tui_prefix_chord(&mut self, chord: &str) -> Result<(), SettingsError> {
        let trimmed = chord.trim().to_lowercase();
        if trimmed.is_empty() {
            return Err(SettingsError::Invalid(
                "prefix_chord must not be empty".into(),
            ));
        }
        self.snapshot.tui.prefix_chord = trimmed;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_tui_unicode_fallback(&mut self, value: bool) -> Result<(), SettingsError> {
        self.snapshot.tui.unicode_fallback = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_tui_bell_notification(&mut self, value: bool) -> Result<(), SettingsError> {
        self.snapshot.tui.bell_notification = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_tui_high_contrast(&mut self, value: bool) -> Result<(), SettingsError> {
        self.snapshot.tui.high_contrast = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    /// Set a workspace-local override. Does not save; call [`Self::save`]
    /// to persist and pick up atomic-write/backup handling.
    pub fn set_workspace_backup_retention(
        &mut self,
        workspace: &str,
        retention: u32,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.backup_retention = Some(retention);
        over.validate()?;
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn set_workspace_default_session(
        &mut self,
        workspace: &str,
        session: Option<&str>,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.default_session = session.map(str::to_string);
        over.validate()?;
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn set_workspace_confirm_new_enrollment(
        &mut self,
        workspace: &str,
        value: bool,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.orchestration.confirm_new_enrollment = Some(value);
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn set_workspace_confirm_broadcast(
        &mut self,
        workspace: &str,
        value: bool,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.orchestration.confirm_broadcast = Some(value);
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn set_workspace_auto_enrollment_allowed(
        &mut self,
        workspace: &str,
        value: bool,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.orchestration.auto_enrollment_allowed = Some(value);
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn set_workspace_sandbox_strictness(
        &mut self,
        workspace: &str,
        value: SandboxStrictness,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.orchestration.sandbox_strictness = Some(value);
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn set_workspace_retention_days(
        &mut self,
        workspace: &str,
        days: u32,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.orchestration.retention_days = Some(days);
        over.validate()?;
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn set_workspace_export_enabled(
        &mut self,
        workspace: &str,
        value: bool,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.orchestration.export_enabled = Some(value);
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn list_profiles(&self) -> Vec<ProfileSnapshot> {
        self.profiles
            .values()
            .map(ProviderProfile::snapshot)
            .collect()
    }

    pub fn profile(&self, name: &str) -> Option<&ProviderProfile> {
        self.profiles.get(name)
    }

    pub fn upsert_profile(&mut self, profile: ProviderProfile) -> Result<(), SettingsError> {
        validate_profile(&profile)?;
        self.profiles.insert(profile.name.clone(), profile);
        write_profile_fields(&mut self.document, &self.profiles);
        Ok(())
    }

    pub fn rename_profile(&mut self, from: &str, to: &str) -> Result<(), SettingsError> {
        let to = validate_profile_name(to)?;
        if from == to {
            return Ok(());
        }
        if self.profiles.contains_key(&to) {
            return Err(SettingsError::Conflict(format!(
                "profile {to} already exists"
            )));
        }
        let mut profile = self
            .profiles
            .remove(from)
            .ok_or_else(|| SettingsError::Invalid(format!("unknown profile {from}")))?;
        profile.name = to.clone();
        self.profiles.insert(to.clone(), profile);
        for over in self.workspaces.values_mut() {
            for selected in over.default_profiles.values_mut() {
                if selected == from {
                    *selected = to.clone();
                }
            }
        }
        write_profile_fields(&mut self.document, &self.profiles);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    /// Remove profile configuration only. Never deletes a keychain secret.
    /// Workspace default selections that pointed at this profile are cleared.
    pub fn remove_profile(&mut self, name: &str) -> Result<ProviderProfile, SettingsError> {
        let profile = self
            .profiles
            .remove(name)
            .ok_or_else(|| SettingsError::Invalid(format!("unknown profile {name}")))?;
        for over in self.workspaces.values_mut() {
            over.default_profiles.retain(|_, selected| selected != name);
        }
        write_profile_fields(&mut self.document, &self.profiles);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(profile)
    }

    pub fn set_harness_settings(&mut self, settings: HarnessSettings) -> Result<(), SettingsError> {
        validate_harness(&settings)?;
        self.harnesses.insert(settings.harness.clone(), settings);
        write_harness_fields(&mut self.document, &self.harnesses);
        Ok(())
    }

    pub fn harness_settings(&self, harness: &str) -> Result<HarnessSettings, SettingsError> {
        let harness = validate_provider(harness)?;
        Ok(self
            .harnesses
            .get(&harness)
            .cloned()
            .unwrap_or(HarnessSettings::default_for(&harness)?))
    }

    pub fn set_workspace_default_profile(
        &mut self,
        workspace: &str,
        harness: &str,
        profile: &str,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let harness = validate_provider(harness)?;
        let profile = validate_profile_name(profile)?;
        if !self.profiles.contains_key(&profile) {
            return Err(SettingsError::Invalid(format!("unknown profile {profile}")));
        }
        if self.profiles[&profile].provider != harness {
            return Err(SettingsError::Invalid(format!(
                "profile {profile} is for provider {}, not {harness}",
                self.profiles[&profile].provider
            )));
        }
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.default_profiles.insert(harness, profile);
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn reset_workspace_default_profile(
        &mut self,
        workspace: &str,
        harness: &str,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let harness = validate_provider(harness)?;
        if let Some(over) = self.workspaces.get_mut(&workspace) {
            over.default_profiles.remove(&harness);
            if over.is_empty() {
                self.workspaces.remove(&workspace);
            }
            write_workspace_fields(&mut self.document, &self.workspaces);
        }
        Ok(())
    }

    pub fn effective_harness(
        &self,
        workspace: Option<&str>,
        harness: &str,
    ) -> Result<EffectiveHarnessSettings, SettingsError> {
        let harness = validate_provider(harness)?;
        Ok(effective_harnesses(
            &self.harnesses,
            &self.profiles,
            workspace.and_then(|path| self.workspaces.get(path).map(|over| &over.default_profiles)),
        )
        .into_iter()
        .find(|entry| entry.harness == harness)
        .unwrap_or_else(|| {
            let settings = HarnessSettings::default_for(&harness).expect("validated provider");
            EffectiveHarnessSettings {
                harness: settings.harness,
                executable: settings.executable,
                workdir: settings.workdir,
                capture_polling: settings.capture_polling,
                inject_permission: settings.inject_permission,
                default_profile: None,
                default_profile_status: FieldStatus::Inherited,
                default_profile_badge: None,
            }
        }))
    }

    /// Clear one field of a workspace override, falling back to the global
    /// default. Removes the workspace entry entirely once it has no
    /// overridden fields left. Does not save.
    pub fn reset_workspace_field(
        &mut self,
        workspace: &str,
        field: SettingsField,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        if let Some(over) = self.workspaces.get_mut(&workspace) {
            match field {
                SettingsField::BackupRetention => over.backup_retention = None,
                SettingsField::DefaultWorkspace => {}
                SettingsField::DefaultSession => over.default_session = None,
                SettingsField::ConfirmNewEnrollment => {
                    over.orchestration.confirm_new_enrollment = None
                }
                SettingsField::ConfirmBroadcast => over.orchestration.confirm_broadcast = None,
                SettingsField::AutoEnrollmentAllowed => {
                    over.orchestration.auto_enrollment_allowed = None
                }
                SettingsField::SandboxStrictness => over.orchestration.sandbox_strictness = None,
                SettingsField::RetentionDays => over.orchestration.retention_days = None,
                SettingsField::ExportEnabled => over.orchestration.export_enabled = None,
            }
            if over.is_empty() {
                self.workspaces.remove(&workspace);
            }
            write_workspace_fields(&mut self.document, &self.workspaces);
        }
        Ok(())
    }

    /// Atomic save. Refuses to overwrite a malformed original so recovery can
    /// inspect or restore it. Use [`Self::quarantine_invalid_and_save`] to
    /// replace a broken file after copying it aside.
    pub fn save(&mut self) -> Result<(), SettingsError> {
        if let LoadStatus::Invalid { reason } | LoadStatus::Unreadable { reason } =
            &self.load.status
        {
            return Err(SettingsError::Conflict(format!(
                "refusing to overwrite unusable settings at {}: {reason}",
                self.load.path.display()
            )));
        }
        self.write_atomically(true)
    }

    /// Move a malformed `settings.toml` aside, then write validated defaults.
    pub fn quarantine_invalid_and_save(&mut self) -> Result<PathBuf, SettingsError> {
        match &self.load.status {
            LoadStatus::Invalid { .. } | LoadStatus::Unreadable { .. } => {}
            _ => {
                return Err(SettingsError::Conflict(
                    "quarantine is only for unusable settings files".into(),
                ));
            }
        }
        if self.load.path.exists() {
            let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
            let quarantine = self.home.join(format!("settings.toml.invalid.{stamp}"));
            fs::rename(&self.load.path, &quarantine)?;
            self.load.status = LoadStatus::Missing;
            self.write_atomically(false)?;
            Ok(quarantine)
        } else {
            self.load.status = LoadStatus::Missing;
            self.write_atomically(false)?;
            Ok(self.load.path.clone())
        }
    }

    pub fn list_backups(&self) -> Result<Vec<PathBuf>, SettingsError> {
        let dir = self.home.join(BACKUP_DIR);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut backups: Vec<PathBuf> = fs::read_dir(&dir)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("settings-") && name.ends_with(".toml"))
            })
            .collect();
        backups.sort();
        backups.reverse();
        Ok(backups)
    }

    /// Replace `settings.toml` with a timestamped last-known-good backup.
    pub fn restore_backup(&mut self, backup: impl AsRef<Path>) -> Result<(), SettingsError> {
        let backup = backup.as_ref();
        let backup_dir = self
            .home
            .join(BACKUP_DIR)
            .canonicalize()
            .unwrap_or_else(|_| self.home.join(BACKUP_DIR));
        let backup_canon = backup.canonicalize().map_err(|err| {
            SettingsError::Invalid(format!("cannot read backup {}: {err}", backup.display()))
        })?;
        if !backup_canon.starts_with(&backup_dir) {
            return Err(SettingsError::Invalid(
                "backup path must be inside the settings-backups directory".into(),
            ));
        }
        let raw = fs::read_to_string(&backup_canon)?;
        let (document, snapshot, workspaces, profiles, harnesses) = parse_document(&raw)?;
        snapshot.validate()?;
        self.document = document;
        self.snapshot = snapshot;
        self.workspaces = workspaces;
        self.profiles = profiles;
        self.harnesses = harnesses;
        self.load.status = LoadStatus::Loaded;
        self.write_atomically(false)
    }

    fn write_atomically(&mut self, backup_current: bool) -> Result<(), SettingsError> {
        self.snapshot.validate()?;
        for (path, over) in &self.workspaces {
            over.validate()
                .map_err(|err| SettingsError::Invalid(format!("workspace {path}: {err}")))?;
        }
        for profile in self.profiles.values() {
            validate_profile(profile)?;
        }
        for settings in self.harnesses.values() {
            validate_harness(settings)?;
        }
        write_snapshot_fields(&mut self.document, &self.snapshot);
        write_workspace_fields(&mut self.document, &self.workspaces);
        write_profile_fields(&mut self.document, &self.profiles);
        write_harness_fields(&mut self.document, &self.harnesses);
        fs::create_dir_all(&self.home)?;
        if backup_current
            && self.load.path.exists()
            && matches!(self.load.status, LoadStatus::Loaded)
        {
            self.backup_current()?;
        }
        let tmp = self.home.join(TMP_FILE);
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp)?;
            file.write_all(self.document.to_string().as_bytes())?;
            file.sync_all()?;
        }
        fsync_dir(&self.home)?;
        replace_file(&tmp, &self.load.path)?;
        fsync_dir(&self.home)?;
        self.load.status = LoadStatus::Loaded;
        self.prune_backups()?;
        Ok(())
    }

    fn backup_current(&self) -> Result<(), SettingsError> {
        let dir = self.home.join(BACKUP_DIR);
        fs::create_dir_all(&dir)?;
        let now = Utc::now();
        let dest = dir.join(format!(
            "settings-{}{:03}Z.toml",
            now.format("%Y%m%dT%H%M%S"),
            now.timestamp_subsec_millis()
        ));
        fs::copy(&self.load.path, &dest)?;
        if let Ok(file) = File::open(&dest) {
            let _ = file.sync_all();
        }
        fsync_dir(&dir)?;
        Ok(())
    }

    fn prune_backups(&self) -> Result<(), SettingsError> {
        let backups = self.list_backups()?;
        for stale in backups
            .into_iter()
            .skip(self.snapshot.backup_retention as usize)
        {
            let _ = fs::remove_file(stale);
        }
        Ok(())
    }
}

fn default_document() -> DocumentMut {
    let mut document = DocumentMut::new();
    document.insert("schema_version", value(i64::from(CURRENT_SETTINGS_SCHEMA)));
    let mut storage = Table::new();
    storage["backup_retention"] = value(i64::from(DEFAULT_BACKUP_RETENTION));
    document.insert("storage", Item::Table(storage));
    document
}

fn write_snapshot_fields(document: &mut DocumentMut, snapshot: &SettingsSnapshot) {
    document["schema_version"] = value(i64::from(snapshot.schema_version));
    if !document.contains_key("storage") {
        document["storage"] = Item::Table(Table::new());
    }
    document["storage"]["backup_retention"] = value(i64::from(snapshot.backup_retention));

    if let Some(ref default_ws) = snapshot.default_workspace {
        if !document.contains_key("general") {
            document["general"] = Item::Table(Table::new());
        }
        document["general"]["default_workspace"] = value(default_ws.as_str());
    } else if document
        .get("general")
        .and_then(Item::as_table)
        .is_some_and(|t| t.contains_key("default_workspace"))
    {
        document["general"]["default_workspace"] = Item::None;
    }

    if let Some(ref default_sess) = snapshot.default_session {
        if !document.contains_key("general") {
            document["general"] = Item::Table(Table::new());
        }
        document["general"]["default_session"] = value(default_sess.as_str());
    } else if document
        .get("general")
        .and_then(Item::as_table)
        .is_some_and(|t| t.contains_key("default_session"))
    {
        document["general"]["default_session"] = Item::None;
    }

    if !document.contains_key("orchestration") {
        document["orchestration"] = Item::Table(Table::new());
    }
    let orch = &snapshot.orchestration;
    document["orchestration"]["confirm_new_enrollment"] = value(orch.confirm_new_enrollment);
    document["orchestration"]["confirm_broadcast"] = value(orch.confirm_broadcast);
    document["orchestration"]["auto_enrollment_allowed"] = value(orch.auto_enrollment_allowed);
    document["orchestration"]["sandbox_strictness"] = value(orch.sandbox_strictness.as_str());
    document["orchestration"]["export_enabled"] = value(orch.export_enabled);
    if let Some(days) = orch.retention_days {
        document["orchestration"]["retention_days"] = value(i64::from(days));
    } else if document
        .get("orchestration")
        .and_then(Item::as_table)
        .is_some_and(|t| t.contains_key("retention_days"))
    {
        document["orchestration"]["retention_days"] = Item::None;
    }

    if !document.contains_key("tui") {
        document["tui"] = Item::Table(Table::new());
    }
    let tui = &snapshot.tui;
    document["tui"]["prefix_chord"] = value(tui.prefix_chord.as_str());
    document["tui"]["unicode_fallback"] = value(tui.unicode_fallback);
    document["tui"]["bell_notification"] = value(tui.bell_notification);
    document["tui"]["high_contrast"] = value(tui.high_contrast);
}

/// Rebuild the `[[workspace]]` array-of-tables from `workspaces` on every
/// save, mirroring how `write_snapshot_fields` unconditionally overwrites
/// `[storage]`. Comments inside a rewritten workspace block do not survive a
/// save that touches it; top-level document comments are unaffected.
fn write_workspace_fields(
    document: &mut DocumentMut,
    workspaces: &BTreeMap<String, WorkspaceOverride>,
) {
    if workspaces.is_empty() {
        document.remove("workspace");
        return;
    }
    let mut array = ArrayOfTables::new();
    for (path, over) in workspaces {
        let mut table = Table::new();
        table["path"] = value(path.as_str());
        if let Some(retention) = over.backup_retention {
            table["backup_retention"] = value(i64::from(retention));
        }
        if let Some(ref sess) = over.default_session {
            table["default_session"] = value(sess.as_str());
        }
        write_default_profiles(&mut table, &over.default_profiles);
        write_orchestration_override(&mut table, &over.orchestration);
        array.push(table);
    }
    document["workspace"] = Item::ArrayOfTables(array);
}

/// Merge global orchestration policy with an optional workspace override.
fn effective_orchestration(
    global: &OrchestrationPolicy,
    over: Option<&OrchestrationOverride>,
) -> EffectiveOrchestrationPolicy {
    fn merge<T: Copy>(global: T, over: Option<T>) -> (T, FieldStatus) {
        match over {
            Some(value) => (value, FieldStatus::Override),
            None => (global, FieldStatus::Inherited),
        }
    }
    let (confirm_new_enrollment, confirm_new_enrollment_status) = merge(
        global.confirm_new_enrollment,
        over.and_then(|o| o.confirm_new_enrollment),
    );
    let (confirm_broadcast, confirm_broadcast_status) = merge(
        global.confirm_broadcast,
        over.and_then(|o| o.confirm_broadcast),
    );
    let (auto_enrollment_allowed, auto_enrollment_allowed_status) = merge(
        global.auto_enrollment_allowed,
        over.and_then(|o| o.auto_enrollment_allowed),
    );
    let (sandbox_strictness, sandbox_strictness_status) = merge(
        global.sandbox_strictness,
        over.and_then(|o| o.sandbox_strictness),
    );
    let (retention_days, retention_days_status) = match over.and_then(|o| o.retention_days) {
        Some(value) => (Some(value), FieldStatus::Override),
        None => (global.retention_days, FieldStatus::Inherited),
    };
    let (export_enabled, export_enabled_status) =
        merge(global.export_enabled, over.and_then(|o| o.export_enabled));
    EffectiveOrchestrationPolicy {
        confirm_new_enrollment,
        confirm_new_enrollment_status,
        confirm_broadcast,
        confirm_broadcast_status,
        auto_enrollment_allowed,
        auto_enrollment_allowed_status,
        sandbox_strictness,
        sandbox_strictness_status,
        retention_days,
        retention_days_status,
        export_enabled,
        export_enabled_status,
    }
}

fn write_orchestration_override(table: &mut Table, over: &OrchestrationOverride) {
    if over.is_empty() {
        table.remove("orchestration");
        return;
    }
    let mut inline = toml_edit::InlineTable::new();
    if let Some(v) = over.confirm_new_enrollment {
        inline.insert("confirm_new_enrollment", value(v).into_value().unwrap());
    }
    if let Some(v) = over.confirm_broadcast {
        inline.insert("confirm_broadcast", value(v).into_value().unwrap());
    }
    if let Some(v) = over.auto_enrollment_allowed {
        inline.insert("auto_enrollment_allowed", value(v).into_value().unwrap());
    }
    if let Some(v) = over.sandbox_strictness {
        inline.insert(
            "sandbox_strictness",
            value(v.as_str()).into_value().unwrap(),
        );
    }
    if let Some(v) = over.retention_days {
        inline.insert("retention_days", value(i64::from(v)).into_value().unwrap());
    }
    if let Some(v) = over.export_enabled {
        inline.insert("export_enabled", value(v).into_value().unwrap());
    }
    table["orchestration"] = Item::Value(toml_edit::Value::InlineTable(inline));
}

fn orchestration_override_from_table(
    table: &Table,
) -> Result<OrchestrationOverride, SettingsError> {
    let Some(inner) = table.get("orchestration").and_then(Item::as_inline_table) else {
        if table.get("orchestration").is_some() {
            return Err(SettingsError::Invalid(
                "orchestration override must be an inline table".into(),
            ));
        }
        return Ok(OrchestrationOverride::default());
    };
    let confirm_new_enrollment = inner
        .get("confirm_new_enrollment")
        .and_then(|v| v.as_bool());
    let confirm_broadcast = inner.get("confirm_broadcast").and_then(|v| v.as_bool());
    let auto_enrollment_allowed = inner
        .get("auto_enrollment_allowed")
        .and_then(|v| v.as_bool());
    let sandbox_strictness = match inner.get("sandbox_strictness").and_then(|v| v.as_str()) {
        Some(raw) => Some(SandboxStrictness::parse(raw).ok_or_else(|| {
            SettingsError::Invalid(format!(
                "orchestration.sandbox_strictness {raw:?} is unknown"
            ))
        })?),
        None => None,
    };
    let retention_days = match inner.get("retention_days") {
        Some(v) => Some(u32_from_i64(
            v.as_integer().ok_or_else(|| {
                SettingsError::Invalid("orchestration.retention_days must be an integer".into())
            })?,
            "orchestration.retention_days",
        )?),
        None => None,
    };
    let export_enabled = inner.get("export_enabled").and_then(|v| v.as_bool());
    Ok(OrchestrationOverride {
        confirm_new_enrollment,
        confirm_broadcast,
        auto_enrollment_allowed,
        sandbox_strictness,
        retention_days,
        export_enabled,
    })
}

fn normalize_workspace(workspace: &str) -> Result<String, SettingsError> {
    let trimmed = workspace.trim();
    if trimmed.is_empty() {
        return Err(SettingsError::Invalid(
            "workspace path must not be empty".into(),
        ));
    }
    Ok(trimmed.to_string())
}

#[allow(clippy::type_complexity)]
fn parse_document(
    raw: &str,
) -> Result<
    (
        DocumentMut,
        SettingsSnapshot,
        BTreeMap<String, WorkspaceOverride>,
        BTreeMap<String, ProviderProfile>,
        BTreeMap<String, HarnessSettings>,
    ),
    SettingsError,
> {
    let document = raw
        .parse::<DocumentMut>()
        .map_err(|err| SettingsError::Invalid(err.to_string()))?;
    let snapshot = snapshot_from_document(&document)?;
    snapshot.validate()?;
    let workspaces = workspaces_from_document(&document)?;
    for (path, over) in &workspaces {
        over.validate()
            .map_err(|err| SettingsError::Invalid(format!("workspace {path}: {err}")))?;
    }
    let profiles = profiles_from_document(&document)?;
    let harnesses = harnesses_from_document(&document)?;
    Ok((document, snapshot, workspaces, profiles, harnesses))
}

fn workspaces_from_document(
    document: &DocumentMut,
) -> Result<BTreeMap<String, WorkspaceOverride>, SettingsError> {
    let mut map = BTreeMap::new();
    let Some(array) = document.get("workspace").and_then(Item::as_array_of_tables) else {
        return Ok(map);
    };
    for table in array.iter() {
        let path = table
            .get("path")
            .and_then(Item::as_str)
            .ok_or_else(|| SettingsError::Invalid("workspace entry missing path".into()))?
            .to_string();
        if path.trim().is_empty() {
            return Err(SettingsError::Invalid(
                "workspace path must not be empty".into(),
            ));
        }
        let backup_retention = match table.get("backup_retention") {
            Some(item) => Some(u32_from_i64(
                item.as_integer().ok_or_else(|| {
                    SettingsError::Invalid(format!(
                        "workspace {path} backup_retention must be an integer"
                    ))
                })?,
                "workspace.backup_retention",
            )?),
            None => None,
        };
        let default_session = table
            .get("default_session")
            .and_then(Item::as_str)
            .map(str::to_string);
        let default_profiles = default_profiles_from_table(table)?;
        let orchestration = orchestration_override_from_table(table)
            .map_err(|err| SettingsError::Invalid(format!("workspace {path}: {err}")))?;
        if map
            .insert(
                path.clone(),
                WorkspaceOverride {
                    backup_retention,
                    default_session,
                    default_profiles,
                    orchestration,
                },
            )
            .is_some()
        {
            return Err(SettingsError::Invalid(format!(
                "duplicate workspace override for {path}"
            )));
        }
    }
    Ok(map)
}

fn snapshot_from_document(document: &DocumentMut) -> Result<SettingsSnapshot, SettingsError> {
    let schema_version = integer_key(document.as_table(), "schema_version")?;
    let storage = document
        .get("storage")
        .and_then(Item::as_table)
        .ok_or_else(|| SettingsError::Invalid("missing [storage] table".into()))?;
    let backup_retention = integer_key(storage, "backup_retention")?;

    let general = document.get("general").and_then(Item::as_table);
    let default_workspace = general
        .and_then(|g| g.get("default_workspace"))
        .and_then(Item::as_str)
        .map(str::to_string);
    let default_session = general
        .and_then(|g| g.get("default_session"))
        .and_then(Item::as_str)
        .map(str::to_string);

    let orchestration = document
        .get("orchestration")
        .and_then(Item::as_table)
        .map(orchestration_policy_from_table)
        .transpose()?
        .unwrap_or_default();

    let tui = document
        .get("tui")
        .and_then(Item::as_table)
        .map(tui_settings_from_table)
        .transpose()?
        .unwrap_or_default();

    Ok(SettingsSnapshot {
        schema_version: u32_from_i64(schema_version, "schema_version")?,
        backup_retention: u32_from_i64(backup_retention, "storage.backup_retention")?,
        default_workspace,
        default_session,
        orchestration,
        tui,
    })
}

fn tui_settings_from_table(table: &Table) -> Result<TuiSettings, SettingsError> {
    let defaults = TuiSettings::default();
    let prefix_chord = match table.get("prefix_chord").and_then(Item::as_str) {
        Some(s) => s.to_string(),
        None => defaults.prefix_chord,
    };
    let unicode_fallback = bool_key_or(table, "unicode_fallback", defaults.unicode_fallback)?;
    let bell_notification = bool_key_or(table, "bell_notification", defaults.bell_notification)?;
    let high_contrast = bool_key_or(table, "high_contrast", defaults.high_contrast)?;
    Ok(TuiSettings {
        prefix_chord,
        unicode_fallback,
        bell_notification,
        high_contrast,
    })
}

fn orchestration_policy_from_table(table: &Table) -> Result<OrchestrationPolicy, SettingsError> {
    let defaults = OrchestrationPolicy::default();
    let confirm_new_enrollment = bool_key_or(
        table,
        "confirm_new_enrollment",
        defaults.confirm_new_enrollment,
    )?;
    let confirm_broadcast = bool_key_or(table, "confirm_broadcast", defaults.confirm_broadcast)?;
    let auto_enrollment_allowed = bool_key_or(
        table,
        "auto_enrollment_allowed",
        defaults.auto_enrollment_allowed,
    )?;
    let export_enabled = bool_key_or(table, "export_enabled", defaults.export_enabled)?;
    let sandbox_strictness = match table.get("sandbox_strictness").and_then(Item::as_str) {
        Some(raw) => SandboxStrictness::parse(raw).ok_or_else(|| {
            SettingsError::Invalid(format!(
                "orchestration.sandbox_strictness {raw:?} is unknown"
            ))
        })?,
        None => defaults.sandbox_strictness,
    };
    let retention_days = match table.get("retention_days") {
        Some(item) => Some(u32_from_i64(
            item.as_integer().ok_or_else(|| {
                SettingsError::Invalid("orchestration.retention_days must be an integer".into())
            })?,
            "orchestration.retention_days",
        )?),
        None => None,
    };
    Ok(OrchestrationPolicy {
        confirm_new_enrollment,
        confirm_broadcast,
        auto_enrollment_allowed,
        sandbox_strictness,
        retention_days,
        export_enabled,
    })
}

fn bool_key_or(table: &Table, key: &str, fallback: bool) -> Result<bool, SettingsError> {
    match table.get(key) {
        Some(item) => item.as_bool().ok_or_else(|| {
            SettingsError::Invalid(format!("orchestration.{key} must be a boolean"))
        }),
        None => Ok(fallback),
    }
}

fn integer_key(table: &Table, key: &str) -> Result<i64, SettingsError> {
    table
        .get(key)
        .and_then(Item::as_integer)
        .ok_or_else(|| SettingsError::Invalid(format!("missing or non-integer {key}")))
}

fn u32_from_i64(value: i64, key: &str) -> Result<u32, SettingsError> {
    u32::try_from(value)
        .map_err(|_| SettingsError::Invalid(format!("{key} {value} is out of range")))
}

fn replace_file(tmp: &Path, dest: &Path) -> Result<(), SettingsError> {
    match fs::rename(tmp, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            let _ = fs::remove_file(dest);
            fs::rename(tmp, dest).map_err(SettingsError::from)
        }
    }
}

fn fsync_dir(path: &Path) -> std::io::Result<()> {
    let file = File::open(path)?;
    file.sync_all()
}
