use hub::{HubStore, SettingsStore};
use tempfile::tempdir;
use tui::HubReadModel;

#[test]
fn test_hub_read_model_loads_coherent_data() {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let workspace_path = dir.path().join("test_ws");

    let store = HubStore::open(&home_path).unwrap();
    let mut settings_store = SettingsStore::open(&home_path);

    // Register a work session
    let session = store.create_work_session("T2 Test Session").unwrap();

    // Persist default workspace setting
    let ws_str = workspace_path.display().to_string();
    settings_store.set_default_workspace(Some(&ws_str)).unwrap();
    settings_store.save().unwrap();

    store
        .record_settings_audit_event("general.default_workspace", "global", "set_default")
        .unwrap();

    // Load HubReadModel
    let read_model = HubReadModel::load(&home_path, Some(&workspace_path), Some(&session.id)).unwrap();

    assert!(!read_model.work_sessions.is_empty());
    assert_eq!(read_model.work_sessions[0].name, "T2 Test Session");
    assert!(!read_model.audit_events.is_empty());
    assert_eq!(read_model.audit_events[0].path, "general.default_workspace");
    assert_eq!(
        read_model.effective_settings.default_workspace.as_deref().unwrap(),
        ws_str
    );
}
