use super::*;

use super::document::*;
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
        let harnesses = effective_harnesses(&self.harnesses, &self.profiles, over);
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

    pub fn set_embedding_provider(&mut self, provider: EmbeddingProvider) {
        self.snapshot.embedding_provider = provider;
        write_snapshot_fields(&mut self.document, &self.snapshot);
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

    pub fn set_memory_recall_enabled(&mut self, value: bool) -> Result<(), SettingsError> {
        self.snapshot.orchestration.memory_recall_enabled = value;
        write_snapshot_fields(&mut self.document, &self.snapshot);
        Ok(())
    }

    pub fn set_memory_recall_limit(&mut self, limit: u8) -> Result<(), SettingsError> {
        let mut next = self.snapshot.orchestration.clone();
        next.memory_recall_limit = limit;
        next.validate()?;
        self.snapshot.orchestration = next;
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
}
