//! Integration tests for T7: Local multi-instance coherence and version-stamped reject-and-refresh (#141).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use hub::{HubStore, SettingsStore};
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use tui::app::conflict_ops::{ConflictState, StaleWriteError};
use tui::app::keymap::handle_key;
use tui::app::state::AppState;
use tui::{HubReadModel, TuiOptions};

fn setup_coherence_app() -> (tempfile::TempDir, HubStore, AppState) {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let store = HubStore::open(&home_path).unwrap();

    // Create initial settings.toml
    let settings_file = home_path.join("settings.toml");
    fs::write(&settings_file, "active_profile = \"default\"\n").unwrap();

    let settings_store = SettingsStore::open(&home_path);
    let effective = settings_store.effective(None);
    let read_model = HubReadModel::load(&home_path, None, None).unwrap();
    let app = AppState::new(&TuiOptions::default(), home_path, &effective, read_model);
    (dir, store, app)
}

#[test]
fn test_conflict_state_detects_external_file_mutations() {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let settings_file = home_path.join("settings.toml");

    fs::write(&settings_file, "active_profile = \"profile1\"\n").unwrap();

    let mut conflict = ConflictState::new(&home_path);
    assert!(!conflict.is_stale(&home_path));
    assert!(conflict.guard_write(&home_path).is_ok());

    // External modification after a short pause so mtime increments
    thread::sleep(Duration::from_millis(50));
    fs::write(
        &settings_file,
        "active_profile = \"profile2_external_desktop\"\n",
    )
    .unwrap();

    // Conflict state must now detect staleness
    assert!(conflict.is_stale(&home_path));

    // Guard write must reject stale write and set banner
    let guard_res = conflict.guard_write(&home_path);
    assert!(guard_res.is_err());
    match guard_res {
        Err(StaleWriteError::StaleSettings) => {}
        Err(e) => panic!("Expected StaleSettings error, got {e:?}"),
        Ok(_) => panic!("Expected guard_write to fail on stale file"),
    }

    assert!(conflict.is_conflict_active);
    assert!(conflict
        .conflict_message
        .contains("modified by another instance"));

    // Dismiss banner
    conflict.dismiss();
    assert!(!conflict.is_conflict_active);
    assert!(conflict.conflict_message.is_empty());

    // Update stamp (equivalent to refresh)
    conflict.update_stamp(&home_path);
    assert!(!conflict.is_stale(&home_path));
    assert!(conflict.guard_write(&home_path).is_ok());
}

#[test]
fn test_conflict_state_handles_file_creation_and_deletion() {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let settings_file = home_path.join("settings.toml");

    // Initially no file exists
    let mut conflict = ConflictState::new(&home_path);
    assert_eq!(conflict.expected_stamp, None);
    assert!(!conflict.is_stale(&home_path));

    // File created externally
    fs::write(&settings_file, "active_profile = \"created\"\n").unwrap();
    assert!(conflict.is_stale(&home_path));

    // Refresh stamp
    conflict.update_stamp(&home_path);
    assert!(!conflict.is_stale(&home_path));

    // File deleted externally
    fs::remove_file(&settings_file).unwrap();
    assert!(conflict.is_stale(&home_path));
}

#[test]
fn test_app_refresh_and_retry_workflow() {
    let (_dir, store, mut app) = setup_coherence_app();
    let settings_file = app.home_dir.join("settings.toml");

    // Initial state has clean stamp
    assert!(!app.conflict.is_conflict_active);

    // Simulate external edit by Desktop or second TUI instance
    thread::sleep(Duration::from_millis(50));
    fs::write(&settings_file, "active_profile = \"modified_by_desktop\"\n").unwrap();

    // Guard write fails and displays conflict banner
    let res = app.conflict.guard_write(&app.home_dir);
    assert!(res.is_err());
    assert!(app.conflict.is_conflict_active);

    // When banner is active, pressing 'r' triggers Refresh & retry
    let key_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
    handle_key(&mut app, &store, key_r);

    // After refresh, stamp is updated and conflict banner is cleared
    assert!(!app.conflict.is_conflict_active);
    assert!(!app.conflict.is_stale(&app.home_dir));

    // Subsequent guard_write succeeds
    assert!(app.conflict.guard_write(&app.home_dir).is_ok());
}

#[test]
fn test_app_dismiss_conflict_banner_with_escape() {
    let (_dir, store, mut app) = setup_coherence_app();
    let settings_file = app.home_dir.join("settings.toml");

    thread::sleep(Duration::from_millis(50));
    fs::write(&settings_file, "active_profile = \"modified\"\n").unwrap();

    let _ = app.conflict.guard_write(&app.home_dir);
    assert!(app.conflict.is_conflict_active);

    // Pressing 'Esc' dismisses banner without updating stamp
    let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    handle_key(&mut app, &store, key_esc);

    assert!(!app.conflict.is_conflict_active);
    // Note: still stale because refresh was not called
    assert!(app.conflict.is_stale(&app.home_dir));
}
