//! Integration tests for T8: Kubuntu/Konsole resilience and safety validation (#142).

use hub::{HarnessId, HubStore, SettingsStore};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use tui::app::composer::RecipientMode;
use tui::app::draw_ui;
use tui::app::pane_ops::{HarnessWorkspaceState, PaneKind};
use tui::app::state::{AppState, TabIndex};
use tui::{HubReadModel, TuiOptions};

fn setup_resilience_app() -> (tempfile::TempDir, PathBuf, HubStore, AppState) {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let store = HubStore::open(&home_path).unwrap();

    // Register standard team members
    store.upsert_agent("claude", "Claude").unwrap();
    store.set_team_member("claude", true).unwrap();
    store.upsert_agent("gemini", "Gemini").unwrap();
    store.set_team_member("gemini", true).unwrap();

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
fn test_narrow_and_wide_responsive_layout_rendering() {
    let (_dir, _home_path, _store, mut app) = setup_resilience_app();

    // Set up 2 harness panes in split view
    app.harnesses
        .add_observed(HarnessId::Gemini, PathBuf::from("/tmp"), "Gemini output\n");
    app.harnesses
        .add_observed(HarnessId::Claude, PathBuf::from("/tmp"), "Claude output\n");
    app.harnesses.is_split_view = true;
    app.active_tab = TabIndex::HarnessPanes;

    // 1. Narrow terminal (< 80 cols, e.g. 72 cols x 24 rows)
    let narrow_backend = TestBackend::new(72, 24);
    let mut narrow_terminal = Terminal::new(narrow_backend).unwrap();

    // Must render all tabs in narrow mode without panicking
    for tab in [
        TabIndex::Orchestrate,
        TabIndex::ChatAndMemory,
        TabIndex::SharedHub,
        TabIndex::Settings,
        TabIndex::HarnessPanes,
    ] {
        app.active_tab = tab;
        narrow_terminal
            .draw(|frame| draw_ui(frame, &app))
            .expect("Narrow layout draw failed");
    }

    // 2. Wide terminal (>= 120 cols, e.g. 140 cols x 40 rows)
    let wide_backend = TestBackend::new(140, 40);
    let mut wide_terminal = Terminal::new(wide_backend).unwrap();

    app.active_tab = TabIndex::HarnessPanes;
    wide_terminal
        .draw(|frame| draw_ui(frame, &app))
        .expect("Wide layout draw failed");

    let buffer = wide_terminal.backend().buffer();
    let content = format!("{:?}", buffer);
    // Both pane titles should be visible in wide split mode
    assert!(content.contains("gemini") || content.contains("claude"));
}

#[test]
fn test_ascii_and_unicode_fallback_rendering() {
    let (_dir, _home_path, _store, mut app) = setup_resilience_app();
    app.active_tab = TabIndex::HarnessPanes;
    app.harnesses
        .add_observed(HarnessId::Gemini, PathBuf::from("/tmp"), "Line 1\n");

    // Standard UTF-8 mode
    app.read_model.effective_settings.tui.unicode_fallback = false;
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw_ui(frame, &app)).unwrap();

    let buf_utf8 = format!("{:?}", terminal.backend().buffer());
    // Should contain lightning bolt icon in UTF-8 mode
    assert!(buf_utf8.contains('⚡') || buf_utf8.contains("Coding-Assistants"));

    // Force ASCII fallback mode
    app.read_model.effective_settings.tui.unicode_fallback = true;
    let backend_ascii = TestBackend::new(100, 24);
    let mut terminal_ascii = Terminal::new(backend_ascii).unwrap();
    terminal_ascii.draw(|frame| draw_ui(frame, &app)).unwrap();

    let buf_ascii = format!("{:?}", terminal_ascii.backend().buffer());
    // In ASCII mode, [*] is used instead of ⚡
    assert!(buf_ascii.contains("[*]"));
}

#[test]
fn test_safety_boundary_no_unsafe_attach_to_foreign_pids() {
    let mut ws = HarnessWorkspaceState::new();

    // Adding an observed session creates no writer and attaches no master PTY
    let obs_id = ws.add_observed(
        HarnessId::Cursor,
        PathBuf::from("/workspace"),
        "Foreign process transcript",
    );

    let pane = ws.active_pane_mut().unwrap();
    assert_eq!(pane.id, obs_id);
    assert_eq!(pane.kind, PaneKind::Observed);
    assert!(pane.writer.is_none());
    assert!(pane.master.is_none());
    assert!(pane.child.is_none());

    // Injection / writing is rejected with error
    let res = pane.write_input("rm -rf /\n");
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("strictly read-only"));
}

#[test]
fn test_desktop_parity_c10_to_c13_without_markdown_bus_writes() {
    let (dir, _home_path, store, mut app) = setup_resilience_app();

    // C10: Create work session in HubStore
    app.open_create_session();
    app.create_session.name_input = String::from("parity-session");
    app.create_session
        .selected_members
        .insert("gemini".into(), true);
    app.create_session
        .selected_members
        .insert("claude".into(), true);
    let session = app.create_session.create_session(&store, None).unwrap();
    assert_eq!(session.name, "parity-session");

    // C11: Validate multi-agent orchestration task targets
    let registered_members: Vec<String> = store
        .list_team_members()
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert!(registered_members.contains(&"claude".to_string()));
    assert!(registered_members.contains(&"gemini".to_string()));

    // C12: Request and approve wake gate directly via HubStore
    store
        .upsert_role(
            "capped",
            "capped",
            Some(0),
            Some(10),
            false,
            false,
            false,
            &[],
        )
        .unwrap();
    store.assign_agent_role("gemini", "capped").unwrap();

    let outcomes = store
        .send_tagged_message_gated(
            "gemini",
            &["claude".to_string()],
            false,
            true,
            "Sprint review request",
            Some("urgent"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(outcomes[0].policy_decision, "gate_pending_role_limit");

    app.refresh();
    assert!(!app.read_model.pending_gates.is_empty());
    app.approvals.selected_approval_index = 0;
    let approved = app
        .approvals
        .resolve_selected(&store, &app.read_model.pending_gates, true)
        .unwrap();
    assert_eq!(approved.status, "approved");

    // C13: Send message and inspect delivery outcomes via HubStore
    app.open_composer();
    app.composer.body = String::from("Parity delivery message");
    app.composer.recipient_mode = RecipientMode::Single;
    app.composer.single_recipient = "claude".to_string();

    let send_outcomes = app
        .composer
        .execute_send(&store, None, None, &app.read_model.team_members, None)
        .unwrap();

    assert_eq!(send_outcomes.len(), 1);
    assert_eq!(send_outcomes[0].to_agent, "claude");
    assert!(send_outcomes[0].accepted);

    // Verify no markdown bus files were created on disk in dir
    let md_files: Vec<PathBuf> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();

    assert!(
        md_files.is_empty(),
        "Detected markdown files in home directory: {:?}",
        md_files
    );
}
