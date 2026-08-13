use super::*;

use super::document::*;
impl SettingsStore {
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
