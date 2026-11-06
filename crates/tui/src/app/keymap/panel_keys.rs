//! Panel-specific key event handlers for ca tui (T5 / #139).

use crate::app::danger_ops::DangerAction;
use crate::app::settings_ops::{SettingsScope, SettingsSection};
use crate::app::state::AppState;
use crossterm::event::{self, KeyCode, KeyModifiers};
use hub::HubStore;

pub fn handle_settings_key(app: &mut AppState, store: &HubStore, key: event::KeyEvent) -> bool {
    match (key.code, key.modifiers) {
        (KeyCode::Char('w'), KeyModifiers::NONE) => {
            app.settings.toggle_scope();
            app.status_message = format!("Settings scope: {}", app.settings.active_scope.badge());
            true
        }
        (KeyCode::Char('['), _) => {
            app.settings.prev_section();
            true
        }
        (KeyCode::Char(']'), _) => {
            app.settings.next_section();
            true
        }
        _ => match app.settings.active_section {
            SettingsSection::General => match (key.code, key.modifiers) {
                (KeyCode::Char('b'), KeyModifiers::NONE) => {
                    let cur = app.read_model.effective_settings.backup_retention;
                    let next = if cur >= 20 { 1 } else { cur + 2 };
                    let result = (|| -> anyhow::Result<()> {
                        match (app.settings.active_scope, app.workspace_path.as_deref()) {
                            (SettingsScope::Workspace, Some(workspace)) => {
                                let workspace = workspace.display().to_string();
                                let mut settings = hub::SettingsStore::open(&app.home_dir);
                                settings.set_workspace_backup_retention(&workspace, next)?;
                                settings.save()?;
                                store.record_settings_audit_event(
                                    "workspace.backup_retention",
                                    &workspace,
                                    &next.to_string(),
                                )?;
                                Ok(())
                            }
                            _ => app
                                .settings
                                .set_backup_retention(&app.home_dir, store, next),
                        }
                    })();
                    if result.is_ok() {
                        app.refresh();
                    }
                    true
                }
                _ => false,
            },
            SettingsSection::TuiPreferences => match (key.code, key.modifiers) {
                (KeyCode::Char('p'), KeyModifiers::NONE) => {
                    if let Ok(ch) = app.settings.cycle_prefix(&app.home_dir, store) {
                        app.refresh();
                        app.status_message = format!("TUI prefix chord set to {ch}.");
                    }
                    true
                }
                (KeyCode::Char('u'), KeyModifiers::NONE) => {
                    if let Ok(val) = app.settings.toggle_unicode(&app.home_dir, store) {
                        app.refresh();
                        app.status_message = format!("Unicode fallback set to {val}.");
                    }
                    true
                }
                (KeyCode::Char('b'), KeyModifiers::NONE) => {
                    if let Ok(val) = app.settings.toggle_bell(&app.home_dir, store) {
                        app.refresh();
                        app.status_message = format!("Bell notification set to {val}.");
                    }
                    true
                }
                (KeyCode::Char('c'), KeyModifiers::NONE) => {
                    if let Ok(val) = app.settings.toggle_high_contrast(&app.home_dir, store) {
                        app.refresh();
                        app.status_message = format!("High contrast mode set to {val}.");
                    }
                    true
                }
                _ => false,
            },
            SettingsSection::Profiles => match (key.code, key.modifiers) {
                (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
                    let total = app.read_model.profiles.len();
                    if total > 0 {
                        app.settings.selected_profile_cursor =
                            (app.settings.selected_profile_cursor + 1) % total;
                    }
                    true
                }
                (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
                    let total = app.read_model.profiles.len();
                    if total > 0 {
                        if app.settings.selected_profile_cursor == 0 {
                            app.settings.selected_profile_cursor = total - 1;
                        } else {
                            app.settings.selected_profile_cursor -= 1;
                        }
                    }
                    true
                }
                (KeyCode::Enter, _) | (KeyCode::Char(' '), KeyModifiers::NONE) => {
                    if let Some(prof) = app
                        .read_model
                        .profiles
                        .get(app.settings.selected_profile_cursor)
                        .cloned()
                    {
                        let ws_str = app
                            .workspace_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "global".to_string());
                        if let Ok(()) = app.settings.select_workspace_profile(
                            &app.home_dir,
                            store,
                            &ws_str,
                            &prof.provider,
                            &prof.name,
                        ) {
                            app.refresh();
                        }
                    }
                    true
                }
                _ => false,
            },
            SettingsSection::Advanced => match (key.code, key.modifiers) {
                (KeyCode::Char(' '), KeyModifiers::NONE)
                | (KeyCode::Char('e'), KeyModifiers::NONE) => {
                    app.settings.toggle_advanced();
                    true
                }
                (KeyCode::Char('+'), _) | (KeyCode::Char('='), _) => {
                    let cur = app.read_model.effective_settings.backup_retention;
                    let next = match cur {
                        1 => 3,
                        3 => 5,
                        5 => 10,
                        10 => 20,
                        _ => 5,
                    };
                    let result = (|| -> anyhow::Result<()> {
                        match (app.settings.active_scope, app.workspace_path.as_deref()) {
                            (SettingsScope::Workspace, Some(workspace)) => {
                                let workspace = workspace.display().to_string();
                                let mut settings = hub::SettingsStore::open(&app.home_dir);
                                settings.set_workspace_backup_retention(&workspace, next)?;
                                settings.save()?;
                                store.record_settings_audit_event(
                                    "workspace.backup_retention",
                                    &workspace,
                                    &next.to_string(),
                                )?;
                                Ok(())
                            }
                            _ => app
                                .settings
                                .set_backup_retention(&app.home_dir, store, next),
                        }
                    })();
                    if result.is_ok() {
                        app.refresh();
                    }
                    true
                }
                _ => false,
            },
            SettingsSection::DangerZone => {
                let ws_target_name = app
                    .workspace_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("workspace")
                    .to_string();

                match (key.code, key.modifiers) {
                    (KeyCode::Char('1'), _) => {
                        app.danger
                            .open(DangerAction::ResetWorkspaceOverrides, ws_target_name);
                        true
                    }
                    (KeyCode::Char('2'), _) => {
                        app.danger
                            .open(DangerAction::PurgeTranscript, ws_target_name);
                        true
                    }
                    (KeyCode::Char('3'), _) => {
                        app.danger.open(DangerAction::PurgeMemories, ws_target_name);
                        true
                    }
                    (KeyCode::Char('4'), _) => {
                        app.danger.open(DangerAction::PurgeAllData, ws_target_name);
                        true
                    }
                    (KeyCode::Char('5'), _) => {
                        let profile_name = app
                            .read_model
                            .profiles
                            .get(app.settings.selected_profile_cursor)
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| "default".to_string());
                        app.danger.open(
                            DangerAction::DeleteProfile(profile_name.clone()),
                            profile_name,
                        );
                        true
                    }
                    _ => false,
                }
            }
        },
    }
}

pub fn handle_shared_hub_key(app: &mut AppState, key: event::KeyEvent) -> bool {
    match (key.code, key.modifiers) {
        (KeyCode::Char('u'), KeyModifiers::NONE) => {
            app.hub_view_mode = app.hub_view_mode.next();
            app.status_message = format!("Switched Hub view to: {}", app.hub_view_mode.label());
            true
        }
        _ => false,
    }
}

pub fn handle_memory_key(app: &mut AppState, store: &HubStore, key: event::KeyEvent) -> bool {
    match (key.code, key.modifiers) {
        (KeyCode::Char('/'), KeyModifiers::NONE) | (KeyCode::Char('i'), KeyModifiers::NONE) => {
            app.memory.is_active = true;
            app.status_message =
                String::from("Memory search: typing query (press Enter/Esc when done).");
            true
        }
        (KeyCode::Char('w'), KeyModifiers::NONE) => {
            app.memory.cycle_scope();
            let _ = app
                .memory
                .execute_search(store, app.workspace_path.as_deref());
            app.status_message = format!(
                "Memory scope filter: [{}]",
                app.memory.scope_filter.as_deref().unwrap_or("All Scopes")
            );
            true
        }
        (KeyCode::Enter, _) => {
            let _ = app
                .memory
                .execute_search(store, app.workspace_path.as_deref());
            true
        }
        (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            app.memory.select_next();
            true
        }
        (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            app.memory.select_prev();
            true
        }
        _ => false,
    }
}
