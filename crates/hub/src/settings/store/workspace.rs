use super::*;

use super::document::*;
impl SettingsStore {
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

    pub fn set_workspace_memory_recall_enabled(
        &mut self,
        workspace: &str,
        value: bool,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.orchestration.memory_recall_enabled = Some(value);
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn set_workspace_memory_recall_limit(
        &mut self,
        workspace: &str,
        limit: u8,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.orchestration.memory_recall_limit = Some(limit);
        over.validate()?;
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

    pub fn set_harness_default_model(
        &mut self,
        harness: &str,
        model: Option<&str>,
    ) -> Result<(), SettingsError> {
        let harness_id = validate_provider(harness)?;
        let mut settings = self.harness_settings(&harness_id)?;
        settings.default_model = model
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        self.set_harness_settings(settings)
    }

    pub fn set_harness_default_effort(
        &mut self,
        harness: &str,
        effort: Option<&str>,
    ) -> Result<(), SettingsError> {
        let harness_id = validate_provider(harness)?;
        let mut settings = self.harness_settings(&harness_id)?;
        settings.default_effort = effort
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        self.set_harness_settings(settings)
    }

    pub fn set_workspace_default_model(
        &mut self,
        workspace: &str,
        harness: &str,
        model: &str,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let harness = validate_provider(harness)?;
        let model = model.trim();
        if model.is_empty() {
            return Err(SettingsError::Invalid("model must not be empty".into()));
        }
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.default_models.insert(harness, model.to_string());
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn reset_workspace_default_model(
        &mut self,
        workspace: &str,
        harness: &str,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let harness = validate_provider(harness)?;
        if let Some(over) = self.workspaces.get_mut(&workspace) {
            over.default_models.remove(&harness);
            if over.is_empty() {
                self.workspaces.remove(&workspace);
            }
            write_workspace_fields(&mut self.document, &self.workspaces);
        }
        Ok(())
    }

    pub fn set_workspace_default_effort(
        &mut self,
        workspace: &str,
        harness: &str,
        effort: &str,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let harness = validate_provider(harness)?;
        let effort = effort.trim();
        if effort.is_empty() {
            return Err(SettingsError::Invalid("effort must not be empty".into()));
        }
        let mut over = self.workspaces.get(&workspace).cloned().unwrap_or_default();
        over.default_efforts.insert(harness, effort.to_string());
        self.workspaces.insert(workspace, over);
        write_workspace_fields(&mut self.document, &self.workspaces);
        Ok(())
    }

    pub fn reset_workspace_default_effort(
        &mut self,
        workspace: &str,
        harness: &str,
    ) -> Result<(), SettingsError> {
        let workspace = normalize_workspace(workspace)?;
        let harness = validate_provider(harness)?;
        if let Some(over) = self.workspaces.get_mut(&workspace) {
            over.default_efforts.remove(&harness);
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
            workspace.and_then(|path| self.workspaces.get(path)),
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
                selected_model: settings.default_model,
                selected_model_status: FieldStatus::Inherited,
                selected_effort: settings.default_effort,
                selected_effort_status: FieldStatus::Inherited,
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
                SettingsField::LinkSuggestionMode => over.orchestration.link_suggestion_mode = None,
                SettingsField::MemoryRecallEnabled => {
                    over.orchestration.memory_recall_enabled = None
                }
                SettingsField::MemoryRecallLimit => over.orchestration.memory_recall_limit = None,
            }
            if over.is_empty() {
                self.workspaces.remove(&workspace);
            }
            write_workspace_fields(&mut self.document, &self.workspaces);
        }
        Ok(())
    }
}
