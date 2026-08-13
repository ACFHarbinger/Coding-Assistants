use super::model::{
    EffectiveSettings, FieldStatus, SettingsError, SettingsField, SettingsSnapshot,
    WorkspaceOverride, CURRENT_SETTINGS_SCHEMA, DEFAULT_BACKUP_RETENTION,
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
                load: SettingsLoad {
                    path,
                    status: LoadStatus::Unreadable {
                        reason: err.to_string(),
                    },
                },
            },
            Ok(raw) => match parse_document(&raw) {
                Ok((document, snapshot, workspaces)) => Self {
                    home,
                    document,
                    snapshot,
                    workspaces,
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
        EffectiveSettings {
            schema_version: self.snapshot.schema_version,
            workspace: workspace.map(str::to_string),
            backup_retention,
            backup_retention_status,
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
        Ok(())
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
            }
            if over.is_empty() {
                self.workspaces.remove(&workspace);
            }
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
        let (document, snapshot, workspaces) = parse_document(&raw)?;
        snapshot.validate()?;
        self.document = document;
        self.snapshot = snapshot;
        self.workspaces = workspaces;
        self.load.status = LoadStatus::Loaded;
        self.write_atomically(false)
    }

    fn write_atomically(&mut self, backup_current: bool) -> Result<(), SettingsError> {
        self.snapshot.validate()?;
        for (path, over) in &self.workspaces {
            over.validate()
                .map_err(|err| SettingsError::Invalid(format!("workspace {path}: {err}")))?;
        }
        write_snapshot_fields(&mut self.document, &self.snapshot);
        write_workspace_fields(&mut self.document, &self.workspaces);
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
        array.push(table);
    }
    document["workspace"] = Item::ArrayOfTables(array);
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

fn parse_document(
    raw: &str,
) -> Result<
    (
        DocumentMut,
        SettingsSnapshot,
        BTreeMap<String, WorkspaceOverride>,
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
    Ok((document, snapshot, workspaces))
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
        if map
            .insert(path.clone(), WorkspaceOverride { backup_retention })
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
    Ok(SettingsSnapshot {
        schema_version: u32_from_i64(schema_version, "schema_version")?,
        backup_retention: u32_from_i64(backup_retention, "storage.backup_retention")?,
    })
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
