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
    assert_eq!(opts.workspace.unwrap(), PathBuf::from("/tmp/test_workspace"));
    assert_eq!(opts.session.unwrap(), "test_session");
    assert!(opts.set_as_default_workspace_settings);
    assert!(opts.set_as_default_session_settings);
}
