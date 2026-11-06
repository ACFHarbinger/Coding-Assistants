//! Integration tests for T6: Owned and observed harness panes (#140).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use hub::{HarnessId, HubStore, SettingsStore};
use std::path::PathBuf;
use tempfile::tempdir;
use tui::app::keymap::handle_key;
use tui::app::pane_ops::{HarnessWorkspaceState, PaneKind, PaneStatus};
use tui::app::state::{AppState, TabIndex};
use tui::app::vt_parser::VtScreenBuffer;
use tui::{HubReadModel, TuiOptions};

fn setup_app() -> (tempfile::TempDir, HubStore, AppState) {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let store = HubStore::open(&home_path).unwrap();
    let settings_store = SettingsStore::open(&home_path);
    let effective = settings_store.effective(None);
    let read_model = HubReadModel::load(&home_path, None, None).unwrap();
    let app = AppState::new(&TuiOptions::default(), home_path, &effective, read_model);
    (dir, store, app)
}

#[test]
fn test_owned_pane_lifecycle_and_input_output() {
    let dir = tempdir().unwrap();
    let mut ws = HarnessWorkspaceState::new();

    let pane_id = ws
        .launch_owned(HarnessId::Gemini, dir.path().to_path_buf(), 24, 80)
        .expect("Failed to launch owned pane");

    assert_eq!(ws.panes.len(), 1);
    let pane = ws.active_pane().expect("active pane exists");
    assert_eq!(pane.id, pane_id);
    assert_eq!(pane.kind, PaneKind::Owned);
    assert_eq!(pane.status, PaneStatus::Running);
    assert_eq!(pane.harness, HarnessId::Gemini);

    // Drain initial output from shell
    std::thread::sleep(std::time::Duration::from_millis(100));
    ws.drain_all();

    let pane = ws.active_pane_mut().expect("active pane mut exists");
    // Writing input to owned pane must succeed
    let write_res = pane.write_input("echo 'hello from pty'\n");
    assert!(write_res.is_ok(), "Expected write to owned pane to succeed");

    // Propagate resize
    pane.resize(30, 100);

    // Terminate pane
    pane.kill();
    assert_eq!(pane.status, PaneStatus::Exited(130));
}

#[test]
fn test_observed_pane_safety_and_read_only() {
    let (_dir, store, mut app) = setup_app();

    app.active_tab = TabIndex::HarnessPanes;
    let initial_transcript =
        "Captured output from external claude instance.\nTask #42 in progress.\n";
    let pane_id =
        app.harnesses
            .add_observed(HarnessId::Claude, PathBuf::from("/tmp"), initial_transcript);

    assert_eq!(app.harnesses.panes.len(), 1);
    let pane = app.harnesses.active_pane_mut().expect("active pane exists");
    assert_eq!(pane.id, pane_id);
    assert_eq!(pane.kind, PaneKind::Observed);
    assert_eq!(pane.status, PaneStatus::Captured);

    // Direct write to observed pane must be strictly rejected
    let write_res = pane.write_input("malicious keystroke\n");
    assert!(
        write_res.is_err(),
        "Observed session must reject write_input"
    );
    assert!(write_res.unwrap_err().contains("read-only"));

    // Verify buffer contains transcript
    assert_eq!(pane.buffer.line_count(), 2);

    // Focus the pane and simulate keypress
    app.is_pane_focused = true;
    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    handle_key(&mut app, &store, key);

    // Verify key was blocked and status banner updated
    assert!(app
        .status_message
        .contains("Observed sessions are read-only"));
}

#[test]
fn test_harness_workspace_tabs_and_splits() {
    let mut ws = HarnessWorkspaceState::new();
    let ws_path = PathBuf::from("/tmp");

    ws.add_observed(HarnessId::Grok, ws_path.clone(), "Grok session 1");
    ws.add_observed(HarnessId::Claude, ws_path.clone(), "Claude session 2");
    ws.add_observed(HarnessId::Gemini, ws_path, "Gemini session 3");

    assert_eq!(ws.panes.len(), 3);
    assert_eq!(ws.active_pane_idx, 2);

    // Next pane wraps around
    ws.next_pane();
    assert_eq!(ws.active_pane_idx, 0);
    ws.next_pane();
    assert_eq!(ws.active_pane_idx, 1);

    // Prev pane
    ws.prev_pane();
    assert_eq!(ws.active_pane_idx, 0);
    ws.prev_pane();
    assert_eq!(ws.active_pane_idx, 2);

    // Toggle split
    assert!(!ws.is_split_view);
    ws.toggle_split();
    assert!(ws.is_split_view);
    ws.toggle_split();
    assert!(!ws.is_split_view);

    // Close active pane
    ws.close_active_pane();
    assert_eq!(ws.panes.len(), 2);
    assert_eq!(ws.active_pane_idx, 1);
}

#[test]
fn test_prefix_chords_in_harness_view() {
    let (_dir, store, mut app) = setup_app();
    app.active_tab = TabIndex::HarnessPanes;
    app.harnesses
        .add_observed(HarnessId::Grok, PathBuf::from("/tmp"), "Grok buffer");

    // 1. Trigger Ctrl+B
    let ctrl_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
    handle_key(&mut app, &store, ctrl_b);
    assert!(app.is_prefix_mode_active);

    // 2. Chord 'c' opens launcher modal
    let key_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
    handle_key(&mut app, &store, key_c);
    assert!(!app.is_prefix_mode_active);
    assert!(app.harnesses.is_launcher_open);

    // Close launcher modal
    let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    handle_key(&mut app, &store, key_esc);
    assert!(!app.harnesses.is_launcher_open);

    // 3. Trigger Ctrl+B then 's' toggles split
    handle_key(&mut app, &store, ctrl_b);
    assert!(app.is_prefix_mode_active);
    let key_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
    handle_key(&mut app, &store, key_s);
    assert!(app.harnesses.is_split_view);

    // 4. Trigger Ctrl+B then 'd' detaches focus
    app.is_pane_focused = true;
    handle_key(&mut app, &store, ctrl_b);
    assert!(app.is_prefix_mode_active);
    let key_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
    handle_key(&mut app, &store, key_d);
    assert!(!app.is_pane_focused);
}

#[test]
fn test_vt_buffer_ansi_colors_and_scrollback() {
    let mut buffer = VtScreenBuffer::default();

    // Feed text with ANSI SGR color codes: Red, Green, Bold, Reset
    buffer.feed_str("\x1b[31mRed Alert\x1b[0m Normal \x1b[1;32mBold Green\x1b[0m\n");
    buffer.feed_str("Second line of text\rSecond replaced text\n");

    assert_eq!(buffer.line_count(), 2);

    let visible = buffer.visible_lines(10);
    assert_eq!(visible.len(), 2);

    // Check first line spans
    let line0 = &visible[0];
    assert!(!line0.spans.is_empty());

    // Check second line contains carriage-return overwritten content
    let line1 = &visible[1];
    let line1_text: String = line1.spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(line1_text, "Second replaced text");

    // Test scrollback limits
    for i in 0..50 {
        buffer.feed_str(&format!("Line {i}\n"));
    }
    assert_eq!(buffer.line_count(), 52);

    buffer.scroll_up(5);
    assert_eq!(buffer.scroll_offset, 5);
    buffer.scroll_down(2);
    assert_eq!(buffer.scroll_offset, 3);
    buffer.scroll_down(10);
    assert_eq!(buffer.scroll_offset, 0);
}
