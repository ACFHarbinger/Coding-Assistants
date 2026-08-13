use std::path::PathBuf;
use tui::options::TuiOptions;

#[test]
fn test_default_options() {
    let opts = TuiOptions::default();
    assert!(opts.home.is_none());
    assert!(opts.workspace.is_none());
    assert!(opts.session.is_none());
    assert!(!opts.set_as_default_workspace_settings);
    assert!(!opts.set_as_default_session_settings);
}

#[test]
fn test_options_with_overrides() {
    let opts = TuiOptions {
        home: Some(PathBuf::from("/tmp/test_home")),
        workspace: Some(PathBuf::from("/tmp/test_workspace")),
        session: Some("test_session".to_string()),
        set_as_default_workspace_settings: true,
        set_as_default_session_settings: true,
    };

    assert_eq!(opts.home.unwrap(), PathBuf::from("/tmp/test_home"));
    assert_eq!(
        opts.workspace.unwrap(),
        PathBuf::from("/tmp/test_workspace")
    );
    assert_eq!(opts.session.unwrap(), "test_session");
    assert!(opts.set_as_default_workspace_settings);
    assert!(opts.set_as_default_session_settings);
}

#[test]
fn test_set_as_default_workspace_and_session_settings_persistence_and_audit() {
    use hub::{HubStore, SettingsStore};
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let workspace_path = dir.path().join("my_workspace");

    let opts = TuiOptions {
        home: Some(home_path.clone()),
        workspace: Some(workspace_path.clone()),
        session: Some("feature_session".to_string()),
        set_as_default_workspace_settings: true,
        set_as_default_session_settings: true,
    };

    let store = HubStore::open(&home_path).unwrap();
    tui::app::persist_requested_defaults(&opts, &home_path, &store).unwrap();

    // Reload settings store from disk
    let reloaded_settings = SettingsStore::open(&home_path);
    let ws_str = workspace_path.display().to_string();
    let effective = reloaded_settings.effective(Some(&ws_str));

    assert_eq!(effective.default_workspace.as_deref().unwrap(), ws_str);
    assert_eq!(
        effective.default_session.as_deref().unwrap(),
        "feature_session"
    );

    let read_model =
        tui::HubReadModel::load(&home_path, Some(&workspace_path), Some("feature_session"))
            .unwrap();
    let app = tui::app::AppState::new(&opts, home_path, &effective, read_model);
    assert!(app.is_default_workspace_persisted);
    assert!(app.is_default_session_persisted);

    // Audit event check
    let audit_events = store.list_settings_audit_events().unwrap();
    assert_eq!(audit_events.len(), 2);
    assert_eq!(audit_events[0].path, "general.default_workspace");
    assert_eq!(audit_events[1].path, "workspace.default_session");
}
