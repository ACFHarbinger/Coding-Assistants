use tempfile::tempdir;
use tui::app::{AppState, TabIndex};
use tui::{HubReadModel, TuiOptions};

#[test]
fn test_tui_app_state_navigation_and_command_palette() {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let settings_store = hub::SettingsStore::open(&home_path);
    let effective = settings_store.effective(None);

    let opts = TuiOptions::default();
    let read_model = HubReadModel {
        work_sessions: vec![],
        team_members: vec![],
        channel_messages: vec![],
        tasks: vec![],
        audit_events: vec![],
        effective_settings: effective.clone(),
    };

    let mut app = AppState::new(&opts, home_path, &effective, read_model);

    // Default tab is Orchestrate
    assert_eq!(app.active_tab, TabIndex::Orchestrate);

    // Tab cycle navigation
    app.active_tab = app.active_tab.next();
    assert_eq!(app.active_tab, TabIndex::ChatAndMemory);

    app.active_tab = app.active_tab.next();
    assert_eq!(app.active_tab, TabIndex::SharedHub);

    app.active_tab = app.active_tab.next();
    assert_eq!(app.active_tab, TabIndex::Settings);

    app.active_tab = app.active_tab.next();
    assert_eq!(app.active_tab, TabIndex::Orchestrate);

    // Command palette execution
    app.command_input = String::from("chat");
    app.execute_command();
    assert_eq!(app.active_tab, TabIndex::ChatAndMemory);

    app.command_input = String::from("settings");
    app.execute_command();
    assert_eq!(app.active_tab, TabIndex::Settings);

    app.command_input = String::from("help");
    app.execute_command();
    assert!(app.is_help_open);

    app.command_input = String::from("quit");
    app.execute_command();
    assert!(app.should_quit);
}
