//! Keyboard handling for harness panes, launcher modal, and tmux-style prefix chords (T6).

use super::super::pane_ops::PaneKind;
use super::super::state::AppState;
use super::super::views::harness_launcher_modal::AVAILABLE_HARNESSES;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use hub::HubStore;

/// Handle keys inside the Harness Panes workspace view.
pub fn handle_harness_workspace_key(app: &mut AppState, _store: &HubStore, key: KeyEvent) {
    if app.harnesses.is_launcher_open {
        handle_launcher_modal_key(app, key);
        return;
    }

    // Check prefix chord triggering (default Ctrl+b)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        app.is_prefix_mode_active = true;
        return;
    }

    if app.is_prefix_mode_active {
        handle_prefix_chord_key(app, key);
        return;
    }

    if app.is_pane_focused {
        handle_focused_pane_input(app, key);
    } else {
        handle_pane_navigation_key(app, key);
    }
}

fn handle_launcher_modal_key(app: &mut AppState, key: KeyEvent) {
    let ws_state = &mut app.harnesses;
    match key.code {
        KeyCode::Esc => {
            ws_state.is_launcher_open = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if ws_state.launcher_selected_idx == 0 {
                ws_state.launcher_selected_idx = AVAILABLE_HARNESSES.len().saturating_sub(1);
            } else {
                ws_state.launcher_selected_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            ws_state.launcher_selected_idx =
                (ws_state.launcher_selected_idx + 1) % AVAILABLE_HARNESSES.len();
        }
        KeyCode::Char('m') => {
            ws_state.launcher_mode_is_owned = !ws_state.launcher_mode_is_owned;
        }
        KeyCode::Enter => {
            let harness = AVAILABLE_HARNESSES[ws_state.launcher_selected_idx];
            let ws = app
                .workspace_path
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            if ws_state.launcher_mode_is_owned {
                match ws_state.launch_owned(harness, ws, 24, 80) {
                    Ok(id) => {
                        app.status_message = format!("Launched owned harness pane: {id}");
                        app.is_pane_focused = true;
                    }
                    Err(e) => {
                        app.status_message = format!("Launch failed: {e}");
                    }
                }
            } else {
                let id = ws_state.add_observed(
                    harness,
                    ws,
                    &format!("--- Captured Session for {} [Read-Only] ---\nListening for incoming events...\n", harness.as_str()),
                );
                app.status_message = format!("Attached observed harness pane: {id}");
            }
            ws_state.is_launcher_open = false;
        }
        _ => {}
    }
}

fn handle_prefix_chord_key(app: &mut AppState, key: KeyEvent) {
    app.is_prefix_mode_active = false;
    match key.code {
        KeyCode::Char('d') => {
            // Detach focus from pane to TUI top navigation
            app.is_pane_focused = false;
            app.status_message = "Detached from harness pane focus.".to_string();
        }
        KeyCode::Char('c') => {
            // Open launcher
            app.harnesses.is_launcher_open = true;
            app.harnesses.launcher_selected_idx = 0;
        }
        KeyCode::Char('x') => {
            // Close active pane
            app.harnesses.close_active_pane();
            app.status_message = "Closed active harness pane.".to_string();
        }
        KeyCode::Char('n') => {
            app.harnesses.next_pane();
        }
        KeyCode::Char('p') => {
            app.harnesses.prev_pane();
        }
        KeyCode::Char('s') => {
            app.harnesses.toggle_split();
            let mode = if app.harnesses.is_split_view {
                "split tiles"
            } else {
                "single tab"
            };
            app.status_message = format!("Switched harness layout to {mode}.");
        }
        KeyCode::Char(d @ '1'..='9') => {
            if let Some(digit) = d.to_digit(10) {
                let idx = (digit as usize).saturating_sub(1);
                if idx < app.harnesses.panes.len() {
                    app.harnesses.active_pane_idx = idx;
                }
            }
        }
        KeyCode::Char('o') => {
            app.active_tab = super::super::state::TabIndex::Orchestrate;
            app.scroll_offset = 0;
        }
        KeyCode::Char('h') => {
            app.active_tab = super::super::state::TabIndex::SharedHub;
            app.scroll_offset = 0;
        }
        KeyCode::Char('?') => {
            app.is_help_open = true;
        }
        _ => {}
    }
}

fn handle_focused_pane_input(app: &mut AppState, key: KeyEvent) {
    let ws_state = &mut app.harnesses;
    if let Some(pane) = ws_state.active_pane_mut() {
        if pane.kind == PaneKind::Observed {
            app.status_message =
                "Observed sessions are read-only. Press [Ctrl+B d] to detach.".into();
            return;
        }

        let input_bytes: Option<Vec<u8>> = match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    let ascii = (c as u8).to_ascii_lowercase();
                    if ascii.is_ascii_lowercase() {
                        Some(vec![ascii - b'a' + 1])
                    } else {
                        None
                    }
                } else {
                    let mut b = [0u8; 4];
                    let s = c.encode_utf8(&mut b);
                    Some(s.as_bytes().to_vec())
                }
            }
            KeyCode::Enter => Some(vec![b'\r']),
            KeyCode::Backspace => Some(vec![0x7f]),
            KeyCode::Tab => Some(vec![b'\t']),
            KeyCode::Esc => Some(vec![0x1b]),
            KeyCode::Up => Some(vec![0x1b, b'[', b'A']),
            KeyCode::Down => Some(vec![0x1b, b'[', b'B']),
            KeyCode::Right => Some(vec![0x1b, b'[', b'C']),
            KeyCode::Left => Some(vec![0x1b, b'[', b'D']),
            KeyCode::Home => Some(vec![0x1b, b'[', b'H']),
            KeyCode::End => Some(vec![0x1b, b'[', b'F']),
            KeyCode::PageUp => {
                pane.buffer.scroll_up(10);
                None
            }
            KeyCode::PageDown => {
                pane.buffer.scroll_down(10);
                None
            }
            _ => None,
        };

        if let Some(bytes) = input_bytes {
            let _ = pane.write_input(&String::from_utf8_lossy(&bytes));
        }
    }
}

fn handle_pane_navigation_key(app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('c') => {
            app.harnesses.is_launcher_open = true;
            app.harnesses.launcher_selected_idx = 0;
        }
        KeyCode::Char('x') => {
            app.harnesses.close_active_pane();
            app.status_message = "Closed harness pane.".to_string();
        }
        KeyCode::Char('s') => {
            app.harnesses.toggle_split();
            let mode = if app.harnesses.is_split_view {
                "split tiles"
            } else {
                "single tab"
            };
            app.status_message = format!("Harness layout: {mode}.");
        }
        KeyCode::Tab | KeyCode::Char('l') => {
            app.harnesses.next_pane();
        }
        KeyCode::BackTab | KeyCode::Char('h') => {
            app.harnesses.prev_pane();
        }
        KeyCode::Enter | KeyCode::Char('i') => {
            if !app.harnesses.panes.is_empty() {
                app.is_pane_focused = true;
                app.status_message = "Focused harness terminal. Press [Ctrl+B d] to detach.".into();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(pane) = app.harnesses.active_pane_mut() {
                pane.buffer.scroll_up(2);
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(pane) = app.harnesses.active_pane_mut() {
                pane.buffer.scroll_down(2);
            }
        }
        KeyCode::PageUp => {
            if let Some(pane) = app.harnesses.active_pane_mut() {
                pane.buffer.scroll_up(10);
            }
        }
        KeyCode::PageDown => {
            if let Some(pane) = app.harnesses.active_pane_mut() {
                pane.buffer.scroll_down(10);
            }
        }
        _ => {}
    }
}
