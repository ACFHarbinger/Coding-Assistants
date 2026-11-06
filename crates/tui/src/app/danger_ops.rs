//! S6 Danger-zone confirmation and hard-delete operations for ca tui (T5 / #139).
//!
//! Enforces the desktop confirmation contract:
//! - Cancel-first focus (cancellation never touches data).
//! - Typed target name verification before destructive actions become confirmable.
//! - Amber treatment for recoverable operations (reset overrides).
//! - Red treatment for irreversible hard-purges (transcript, memories, all data, delete profile).

use hub::{HubStore, SettingsStore};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DangerAction {
    ResetWorkspaceOverrides,
    PurgeTranscript,
    PurgeMemories,
    PurgeAllData,
    DeleteProfile(String),
}

impl DangerAction {
    pub fn is_red(&self) -> bool {
        !matches!(self, Self::ResetWorkspaceOverrides)
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::ResetWorkspaceOverrides => "Reset Workspace Overrides",
            Self::PurgeTranscript => "Purge Workspace Transcript",
            Self::PurgeMemories => "Purge Workspace Memories",
            Self::PurgeAllData => "Purge All Workspace Data",
            Self::DeleteProfile(_) => "Delete Provider Profile",
        }
    }

    pub fn description(&self, target_label: &str) -> String {
        match self {
            Self::ResetWorkspaceOverrides => format!(
                "Clears configuration-policy, retention, export/linking, default-session, and harness-profile overrides for {target_label}, reverting them to global defaults. Recoverable."
            ),
            Self::PurgeTranscript => format!(
                "Permanently deletes every message recorded against {target_label} — all kinds and statuses. Irreversible: rows are hard-deleted, not cancelled."
            ),
            Self::PurgeMemories => format!(
                "Permanently deletes every memory recorded against {target_label} — all tiers, including non-stale rows. Irreversible."
            ),
            Self::PurgeAllData => format!(
                "Permanently deletes transcript messages AND memories for {target_label} as an atomic unit with a single audit event. Irreversible."
            ),
            Self::DeleteProfile(name) => format!(
                "Permanently deletes global named provider profile '{name}'. Workspaces using it fall back to global default. Irreversible."
            ),
        }
    }

    pub fn confirm_label(&self) -> &'static str {
        match self {
            Self::ResetWorkspaceOverrides => "Confirm Reset",
            Self::PurgeTranscript => "Confirm Purge Transcript",
            Self::PurgeMemories => "Confirm Purge Memories",
            Self::PurgeAllData => "Confirm Purge All Data",
            Self::DeleteProfile(_) => "Confirm Delete Profile",
        }
    }

    pub fn cancel_label(&self) -> &'static str {
        match self {
            Self::ResetWorkspaceOverrides => "Cancel (Keep Overrides)",
            Self::PurgeTranscript => "Cancel (Keep Transcript)",
            Self::PurgeMemories => "Cancel (Keep Memories)",
            Self::PurgeAllData => "Cancel (Keep Data)",
            Self::DeleteProfile(_) => "Cancel (Keep Profile)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DangerButton {
    #[default]
    Cancel,
    Confirm,
}

#[derive(Debug, Default)]
pub struct DangerModalState {
    pub is_open: bool,
    pub action: Option<DangerAction>,
    pub target_name: String,
    pub input_text: String,
    pub focused_button: DangerButton,
    pub error_message: Option<String>,
}

impl DangerModalState {
    pub fn open(&mut self, action: DangerAction, target_name: String) {
        self.is_open = true;
        self.action = Some(action);
        self.target_name = target_name;
        self.input_text.clear();
        self.focused_button = DangerButton::Cancel; // Cancel-first by contract!
        self.error_message = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.action = None;
        self.target_name.clear();
        self.input_text.clear();
        self.error_message = None;
    }

    pub fn is_matched(&self) -> bool {
        self.input_text.trim() == self.target_name.trim()
    }

    pub fn toggle_button(&mut self) {
        self.focused_button = match self.focused_button {
            DangerButton::Cancel => DangerButton::Confirm,
            DangerButton::Confirm => DangerButton::Cancel,
        };
    }

    pub fn execute(
        &mut self,
        store: &HubStore,
        home: &Path,
        workspace: Option<&Path>,
    ) -> Result<String, anyhow::Error> {
        if !self.is_matched() {
            let msg = format!("Type \"{}\" exactly to confirm.", self.target_name);
            self.error_message = Some(msg.clone());
            anyhow::bail!(msg);
        }

        let action = self
            .action
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No active danger action"))?;

        let result_msg = match action {
            DangerAction::ResetWorkspaceOverrides => {
                let ws = workspace.ok_or_else(|| anyhow::anyhow!("No workspace selected"))?;
                let ws_str = ws.display().to_string();
                let mut settings_store = SettingsStore::open(home);
                let fields = [
                    hub::SettingsField::BackupRetention,
                    hub::SettingsField::DefaultSession,
                    hub::SettingsField::ConfirmNewEnrollment,
                    hub::SettingsField::ConfirmBroadcast,
                    hub::SettingsField::AutoEnrollmentAllowed,
                    hub::SettingsField::SandboxStrictness,
                    hub::SettingsField::RetentionDays,
                    hub::SettingsField::ExportEnabled,
                    hub::SettingsField::LinkSuggestionMode,
                    hub::SettingsField::MemoryRecallEnabled,
                    hub::SettingsField::MemoryRecallLimit,
                ];
                for f in fields {
                    let _ = settings_store.reset_workspace_field(&ws_str, f);
                }
                for h in [
                    "opencode",
                    "gemini",
                    "anthropic",
                    "openai",
                    "codex",
                    "claude",
                ] {
                    let _ = settings_store.reset_workspace_default_profile(&ws_str, h);
                    let _ = settings_store.reset_workspace_default_model(&ws_str, h);
                    let _ = settings_store.reset_workspace_default_effort(&ws_str, h);
                }
                settings_store.save()?;
                store.record_settings_audit_event("workspace-overrides", &ws_str, "reset")?;
                format!(
                    "All workspace overrides reset to global defaults for {}.",
                    self.target_name
                )
            }
            DangerAction::PurgeTranscript => {
                let ws = workspace.ok_or_else(|| anyhow::anyhow!("No workspace selected"))?;
                let deleted = store.purge_messages_in_workspace_with_audit(ws)?;
                format!(
                    "Purged {deleted} transcript message(s) for {}.",
                    self.target_name
                )
            }
            DangerAction::PurgeMemories => {
                let ws = workspace.ok_or_else(|| anyhow::anyhow!("No workspace selected"))?;
                let deleted = store.purge_memories_in_workspace_with_audit(ws)?;
                format!("Purged {deleted} memory item(s) for {}.", self.target_name)
            }
            DangerAction::PurgeAllData => {
                let ws = workspace.ok_or_else(|| anyhow::anyhow!("No workspace selected"))?;
                let (messages, memories) = store.purge_workspace_data_with_audit(ws)?;
                format!(
                    "Purged {messages} transcript message(s) and {memories} memory item(s) for {}.",
                    self.target_name
                )
            }
            DangerAction::DeleteProfile(profile_name) => {
                let mut settings_store = SettingsStore::open(home);
                settings_store.remove_profile(&profile_name)?;
                settings_store.save()?;
                store.record_settings_audit_event("profile", &profile_name, "delete")?;
                format!("Provider profile '{profile_name}' permanently deleted.")
            }
        };

        self.close();
        Ok(result_msg)
    }
}
