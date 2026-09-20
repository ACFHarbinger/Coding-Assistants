//! Integration tests for TUI T5: Settings mutations, profiles, and recovery (#139).

use hub::{HubStore, LoadStatus, SettingsStore};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use tui::app::settings_ops::{format_field_status, SettingsScope, SettingsSection};
use tui::app::state::{AppState, TabIndex};
use tui::{HubReadModel, TuiOptions};

fn setup_test_env() -> (tempfile::TempDir, PathBuf, HubStore, AppState) {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let store = HubStore::open(&home_path).unwrap();

    let settings_store = SettingsStore::open(&home_path);
    let effective = settings_store.effective(None);

    let read_model = HubReadModel::load(&home_path, None, None).unwrap();
    let app = AppState::new(
        &TuiOptions::default(),
        home_path.clone(),
        &effective,
        read_model,
    );

    (dir, home_path, store, app)
}

#[test]
fn test_settings_navigation_and_typed_mutations() {
    let (_dir, home_path, store, mut app) = setup_test_env();

    // 1. Navigate to Settings
    app.active_tab = TabIndex::Settings;
    assert_eq!(app.settings.active_section, SettingsSection::General);
    assert_eq!(app.settings.active_scope, SettingsScope::Global);

    // 2. Cycle sections
    app.settings.next_section();
    assert_eq!(app.settings.active_section, SettingsSection::TuiPreferences);
    app.settings.next_section();
    assert_eq!(app.settings.active_section, SettingsSection::Profiles);
    app.settings.next_section();
    assert_eq!(app.settings.active_section, SettingsSection::Advanced);
    app.settings.next_section();
    assert_eq!(app.settings.active_section, SettingsSection::DangerZone);
    app.settings.prev_section();
    assert_eq!(app.settings.active_section, SettingsSection::Advanced);

    // 3. Scope toggle
    app.settings.toggle_scope();
    assert_eq!(app.settings.active_scope, SettingsScope::Workspace);
    assert_eq!(app.settings.active_scope.badge(), "[Workspace]");
    app.settings.toggle_scope();
    assert_eq!(app.settings.active_scope, SettingsScope::Global);
    assert_eq!(app.settings.active_scope.badge(), "[Global]");

    // 4. Inheritance formatting
    assert_eq!(
        format_field_status(hub::FieldStatus::Inherited),
        "[Inherited: Global]"
    );
    assert_eq!(
        format_field_status(hub::FieldStatus::Override),
        "[Overridden: Workspace]"
    );

    // 5. Typed mutations with audit trail
    let new_bell = app.settings.toggle_bell(&home_path, &store).unwrap();
    let eff = SettingsStore::open(&home_path).effective(None);
    assert_eq!(eff.tui.bell_notification, new_bell);

    let new_unicode = app.settings.toggle_unicode(&home_path, &store).unwrap();
    let eff = SettingsStore::open(&home_path).effective(None);
    assert_eq!(eff.tui.unicode_fallback, new_unicode);

    let new_chord = app.settings.cycle_prefix(&home_path, &store).unwrap();
    let eff = SettingsStore::open(&home_path).effective(None);
    assert_eq!(eff.tui.prefix_chord, new_chord);

    app.settings
        .set_backup_retention(&home_path, &store, 7)
        .unwrap();
    let eff = SettingsStore::open(&home_path).effective(None);
    assert_eq!(eff.backup_retention, 7);

    // Verify audit events
    let audits = store.list_audit_events(false).unwrap();
    assert!(audits.iter().any(|e| e.path == "tui.bell_notification"));
    assert!(audits.iter().any(|e| e.path == "tui.unicode_fallback"));
    assert!(audits.iter().any(|e| e.path == "tui.prefix_chord"));
    assert!(audits.iter().any(|e| e.path == "storage.backup_retention"));
}

#[test]
fn test_provider_profiles_select_only_and_source_badges() {
    let (_dir, home_path, store, mut app) = setup_test_env();

    // Configure a global profile in settings
    let mut settings_store = SettingsStore::open(&home_path);
    settings_store
        .upsert_profile(hub::ProviderProfile {
            name: "gemini-pro".into(),
            provider: "gemini".into(),
            model: Some("gemini-2.5-pro".into()),
            base_url: None,
            secret: hub::SecretReference::EnvVar {
                name: "GEMINI_API_KEY".into(),
            },
        })
        .unwrap();
    settings_store.save().unwrap();

    app.refresh();
    assert_eq!(app.read_model.profiles.len(), 1);
    let prof = &app.read_model.profiles[0];
    assert_eq!(prof.name, "gemini-pro");
    assert_eq!(prof.provider, "gemini");
    assert_eq!(prof.secret_badge, "Env Var $GEMINI_API_KEY");

    // Select profile for workspace (select-only)
    app.settings
        .select_workspace_profile(&home_path, &store, "/repo/project", "gemini", "gemini-pro")
        .unwrap();

    let updated_eff = SettingsStore::open(&home_path).effective(Some("/repo/project"));
    let harness_override = updated_eff
        .harnesses
        .iter()
        .find(|h| h.harness == "gemini")
        .unwrap();
    assert_eq!(harness_override.default_profile, Some("gemini-pro".into()));
    assert_eq!(
        harness_override.default_profile_status,
        hub::FieldStatus::Override
    );

    // Verify audit event
    let audits = store.list_audit_events(false).unwrap();
    assert!(audits
        .iter()
        .any(|e| e.path == "workspace.profile.gemini" && e.status == "approved"));
}

#[test]
fn test_malformed_settings_resilience_and_backup_recovery() {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();

    // 1. Create a valid settings file and save twice so a backup is generated
    let mut initial_store = SettingsStore::open(&home_path);
    initial_store.set_tui_bell_notification(true).unwrap();
    initial_store.save().unwrap();
    initial_store.set_backup_retention(9).unwrap();
    initial_store.save().unwrap();
    assert!(!initial_store.list_backups().unwrap().is_empty());

    // 2. Corrupt settings.toml
    let settings_file = home_path.join("settings.toml");
    fs::write(&settings_file, "this is not valid toml = [[[!!").unwrap();

    // 3. Load HubReadModel and AppState: must not panic, starts safely on defaults
    let read_model = HubReadModel::load(&home_path, None, None).unwrap();
    match &read_model.settings_load.status {
        LoadStatus::Invalid { reason } => {
            assert!(!reason.is_empty());
        }
        LoadStatus::Unreadable { reason } => {
            assert!(!reason.is_empty());
        }
        _ => panic!("Expected load status to be Invalid or Unreadable"),
    }

    let effective = SettingsStore::open(&home_path).effective(None);
    let mut app = AppState::new(
        &TuiOptions::default(),
        home_path.clone(),
        &effective,
        read_model,
    );

    // Recovery modal automatically opens to assist user
    assert!(app.recovery.is_open);
    assert!(!app.read_model.backups.is_empty());

    // 4. Restore selected backup
    let restored = app
        .recovery
        .restore_selected(&home_path, &app.read_model.backups)
        .unwrap();
    assert!(restored.exists());
    assert!(!app.recovery.is_open);

    // Settings are restored and valid again
    let refreshed = SettingsStore::open(&home_path);
    assert!(matches!(refreshed.load().status, LoadStatus::Loaded));
    assert!(refreshed.effective(None).tui.bell_notification);

    // 5. Test quarantine and reset
    fs::write(&settings_file, "garbage toml %%%").unwrap();
    let corrupt_model = HubReadModel::load(&home_path, None, None).unwrap();
    app.recovery
        .check_load_status(&corrupt_model.settings_load.status);
    assert!(app.recovery.is_open);

    let quarantined = app.recovery.quarantine_and_reset(&home_path).unwrap();
    assert!(quarantined.exists());
    let reloaded = SettingsStore::open(&home_path);
    assert!(matches!(reloaded.load().status, LoadStatus::Loaded));
}
