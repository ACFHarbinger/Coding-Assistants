//! Malformed settings resilience and backup recovery for ca tui (T5 / #139).
//!
//! When `settings.toml` is malformed or unreadable, ca tui starts safely
//! on defaults and offers a keyboard-driven prompt to restore from a selected
//! last-known-good backup or quarantine and reset.

use hub::{LoadStatus, SettingsStore};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct RecoveryModalState {
    pub is_open: bool,
    pub reason: String,
    pub selected_backup_index: usize,
    pub error_message: Option<String>,
}

impl RecoveryModalState {
    pub fn check_load_status(&mut self, status: &LoadStatus) {
        match status {
            LoadStatus::Invalid { reason } => {
                self.is_open = true;
                self.reason = format!("Invalid settings format: {reason}");
            }
            LoadStatus::Unreadable { reason } => {
                self.is_open = true;
                self.reason = format!("Unreadable settings file: {reason}");
            }
            _ => {
                self.is_open = false;
            }
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.error_message = None;
    }

    pub fn select_next_backup(&mut self, total: usize) {
        if total > 0 {
            self.selected_backup_index = (self.selected_backup_index + 1) % total;
        }
    }

    pub fn select_prev_backup(&mut self, total: usize) {
        if total > 0 {
            if self.selected_backup_index == 0 {
                self.selected_backup_index = total - 1;
            } else {
                self.selected_backup_index -= 1;
            }
        }
    }

    pub fn restore_selected(
        &mut self,
        home: &Path,
        backups: &[PathBuf],
    ) -> Result<PathBuf, anyhow::Error> {
        let backup = backups
            .get(self.selected_backup_index)
            .ok_or_else(|| anyhow::anyhow!("No backup selected"))?;

        let mut settings_store = SettingsStore::open(home);
        settings_store.restore_backup(backup)?;
        let restored = backup.clone();
        self.close();
        Ok(restored)
    }

    pub fn quarantine_and_reset(&mut self, home: &Path) -> Result<PathBuf, anyhow::Error> {
        let mut settings_store = SettingsStore::open(home);
        let path = settings_store.quarantine_invalid_and_save()?;
        self.close();
        Ok(path)
    }
}
