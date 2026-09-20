use hub::{FieldStatus, HubStore, SettingsStore};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    General,
    TuiPreferences,
    Profiles,
    Advanced,
    DangerZone,
}

impl SettingsSection {
    pub fn next(self) -> Self {
        match self {
            Self::General => Self::TuiPreferences,
            Self::TuiPreferences => Self::Profiles,
            Self::Profiles => Self::Advanced,
            Self::Advanced => Self::DangerZone,
            Self::DangerZone => Self::General,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::General => Self::DangerZone,
            Self::TuiPreferences => Self::General,
            Self::Profiles => Self::TuiPreferences,
            Self::Advanced => Self::Profiles,
            Self::DangerZone => Self::Advanced,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::TuiPreferences => "TUI Preferences",
            Self::Profiles => "Provider Profiles",
            Self::Advanced => "Advanced",
            Self::DangerZone => "Danger Zone",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsScope {
    #[default]
    Global,
    Workspace,
}

impl SettingsScope {
    pub fn toggle(self) -> Self {
        match self {
            Self::Global => Self::Workspace,
            Self::Workspace => Self::Global,
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::Global => "[Global]",
            Self::Workspace => "[Workspace]",
        }
    }
}

#[derive(Debug, Default)]
pub struct SettingsState {
    pub active_section: SettingsSection,
    pub active_scope: SettingsScope,
    pub field_cursor: usize,
    pub is_advanced_expanded: bool,
    pub selected_profile_cursor: usize,
    pub status_message: Option<String>,
}

impl SettingsState {
    pub fn next_section(&mut self) {
        self.active_section = self.active_section.next();
        self.field_cursor = 0;
    }

    pub fn prev_section(&mut self) {
        self.active_section = self.active_section.prev();
        self.field_cursor = 0;
    }

    pub fn toggle_scope(&mut self) {
        self.active_scope = self.active_scope.toggle();
    }

    pub fn toggle_advanced(&mut self) {
        self.is_advanced_expanded = !self.is_advanced_expanded;
    }

    pub fn next_field(&mut self, max: usize) {
        if max > 0 {
            self.field_cursor = (self.field_cursor + 1) % max;
        }
    }

    pub fn prev_field(&mut self, max: usize) {
        if max > 0 {
            if self.field_cursor == 0 {
                self.field_cursor = max - 1;
            } else {
                self.field_cursor -= 1;
            }
        }
    }

    /// Mutate TUI bell notification setting with audit event.
    pub fn toggle_bell(&mut self, home: &Path, store: &HubStore) -> Result<bool, anyhow::Error> {
        let mut settings_store = SettingsStore::open(home);
        let current = settings_store.effective(None).tui.bell_notification;
        let new_val = !current;
        settings_store.set_tui_bell_notification(new_val)?;
        settings_store.save()?;
        store.record_settings_audit_event(
            "tui.bell_notification",
            "global",
            &new_val.to_string(),
        )?;
        self.status_message = Some(format!("TUI bell notification set to {new_val}."));
        Ok(new_val)
    }

    /// Mutate TUI unicode fallback setting with audit event.
    pub fn toggle_unicode(&mut self, home: &Path, store: &HubStore) -> Result<bool, anyhow::Error> {
        let mut settings_store = SettingsStore::open(home);
        let current = settings_store.effective(None).tui.unicode_fallback;
        let new_val = !current;
        settings_store.set_tui_unicode_fallback(new_val)?;
        settings_store.save()?;
        store.record_settings_audit_event(
            "tui.unicode_fallback",
            "global",
            &new_val.to_string(),
        )?;
        self.status_message = Some(format!("TUI unicode fallback set to {new_val}."));
        Ok(new_val)
    }

    /// Mutate TUI high contrast setting with audit event.
    pub fn toggle_high_contrast(
        &mut self,
        home: &Path,
        store: &HubStore,
    ) -> Result<bool, anyhow::Error> {
        let mut settings_store = SettingsStore::open(home);
        let current = settings_store.effective(None).tui.high_contrast;
        let new_val = !current;
        settings_store.set_tui_high_contrast(new_val)?;
        settings_store.save()?;
        store.record_settings_audit_event("tui.high_contrast", "global", &new_val.to_string())?;
        self.status_message = Some(format!("TUI high contrast set to {new_val}."));
        Ok(new_val)
    }

    /// Cycle TUI prefix chord setting with audit event.
    pub fn cycle_prefix(&mut self, home: &Path, store: &HubStore) -> Result<String, anyhow::Error> {
        let mut settings_store = SettingsStore::open(home);
        let current = settings_store.effective(None).tui.prefix_chord;
        let next_chord = match current.to_lowercase().as_str() {
            "ctrl+b" => "ctrl+a",
            "ctrl+a" => "ctrl+x",
            "ctrl+x" => "ctrl+g",
            _ => "ctrl+b",
        };
        settings_store.set_tui_prefix_chord(next_chord)?;
        settings_store.save()?;
        store.record_settings_audit_event("tui.prefix_chord", "global", next_chord)?;
        self.status_message = Some(format!("TUI prefix chord set to {next_chord}."));
        Ok(next_chord.to_string())
    }

    /// Mutate backup retention with audit event.
    pub fn set_backup_retention(
        &mut self,
        home: &Path,
        store: &HubStore,
        count: u32,
    ) -> Result<(), anyhow::Error> {
        let mut settings_store = SettingsStore::open(home);
        settings_store.set_backup_retention(count)?;
        settings_store.save()?;
        store.record_settings_audit_event(
            "storage.backup_retention",
            "global",
            &count.to_string(),
        )?;
        self.status_message = Some(format!("Backup retention set to {count}."));
        Ok(())
    }

    /// Select an existing profile as default for workspace and harness (select-only).
    pub fn select_workspace_profile(
        &mut self,
        home: &Path,
        store: &HubStore,
        workspace: &str,
        harness: &str,
        profile: &str,
    ) -> Result<(), anyhow::Error> {
        let mut settings_store = SettingsStore::open(home);
        settings_store.set_workspace_default_profile(workspace, harness, profile)?;
        settings_store.save()?;
        store.record_settings_audit_event(
            &format!("workspace.profile.{harness}"),
            workspace,
            profile,
        )?;
        self.status_message = Some(format!(
            "Set {harness} default profile to '{profile}' for {workspace}."
        ));
        Ok(())
    }
}

pub fn format_field_status(status: FieldStatus) -> &'static str {
    match status {
        FieldStatus::Inherited => "[Inherited: Global]",
        FieldStatus::Override => "[Overridden: Workspace]",
    }
}
