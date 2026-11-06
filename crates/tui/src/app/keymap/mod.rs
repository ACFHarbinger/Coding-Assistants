//! Key event mapping and dispatch for ca tui (T4 / #138).
//!
//! Handles modal interactions, prefix chords, session switching, composer shortcuts,
//! wake approvals, inbox toggling, and standard navigation.

pub mod harness_keys;
pub mod modals;
pub mod panel_keys;

use super::approvals::ChatViewMode;
use super::state::{AppState, TabIndex};
use crossterm::event::{self, KeyCode, KeyModifiers};
use hub::HubStore;

pub fn handle_key(app: &mut AppState, store: &HubStore, key: event::KeyEvent) {
    // 0. Active conflict banner dismissal or refresh (T7)
    if app.conflict.is_conflict_active {
        if key.code == KeyCode::Char('r') {
            app.refresh();
            return;
        } else if key.code == KeyCode::Esc {
            app.conflict.dismiss();
            return;
        }
    }

    // 1. Danger modal (Cancel-first, typed target name confirmation)
    if modals::handle_danger_modal_key(app, store, key) {
        return;
    }

    // 2. Recovery modal (malformed settings restore/quarantine)
    if modals::handle_recovery_modal_key(app, key) {
        return;
    }

    // 3. Delivery outcomes modal
    if modals::handle_delivery_outcomes_key(app, key) {
        return;
    }

    // 4. Confirmation modal (wakes, broadcasts, auto-enrollments)
    if modals::handle_confirmation_key(app, store, key) {
        return;
    }

    // 5. Work session switcher modal
    if modals::handle_session_switcher_key(app, key) {
        return;
    }

    // 6. Create work session modal
    if modals::handle_create_session_key(app, store, key) {
        return;
    }

    // 7. Message composer modal
    if modals::handle_composer_key(app, store, key) {
        return;
    }

    // 8. Memory search query input mode
    if app.active_tab == TabIndex::ChatAndMemory
        && app.approvals.chat_view_mode == ChatViewMode::MemorySearch
        && app.memory.is_active
    {
        match key.code {
            KeyCode::Esc => {
                app.memory.is_active = false;
            }
            KeyCode::Enter => {
                let _ = app
                    .memory
                    .execute_search(store, app.workspace_path.as_deref());
                app.memory.is_active = false;
            }
            KeyCode::Backspace => {
                app.memory.query.pop();
                let _ = app
                    .memory
                    .execute_search(store, app.workspace_path.as_deref());
            }
            KeyCode::Char(c) => {
                app.memory.query.push(c);
                let _ = app
                    .memory
                    .execute_search(store, app.workspace_path.as_deref());
            }
            _ => {}
        }
        return;
    }

    // 6. Prefix chord mode
    if app.is_prefix_mode_active {
        if app.active_tab == TabIndex::HarnessPanes {
            harness_keys::handle_harness_workspace_key(app, store, key);
            return;
        }
        app.is_prefix_mode_active = false;
        match key.code {
            KeyCode::Char('b') | KeyCode::Char('a') => {
                app.status_message = String::from("Prefix chord action executed.");
            }
            KeyCode::Char('c') => {
                app.active_tab = TabIndex::ChatAndMemory;
                app.scroll_offset = 0;
            }
            KeyCode::Char('o') => {
                app.active_tab = TabIndex::Orchestrate;
                app.scroll_offset = 0;
            }
            KeyCode::Char('h') => {
                app.active_tab = TabIndex::SharedHub;
                app.scroll_offset = 0;
            }
            KeyCode::Char('s') => {
                app.active_tab = TabIndex::Settings;
                app.scroll_offset = 0;
            }
            KeyCode::Char('?') => {
                app.is_help_open = true;
            }
            _ => {}
        }
        return;
    }

    // 7. Command palette modal
    if app.is_command_palette_open {
        match key.code {
            KeyCode::Esc => {
                app.is_command_palette_open = false;
                app.command_input.clear();
            }
            KeyCode::Enter => {
                app.execute_command();
            }
            KeyCode::Backspace => {
                app.command_input.pop();
            }
            KeyCode::Char(c) => {
                app.command_input.push(c);
            }
            _ => {}
        }
        return;
    }

    // 8. Help modal
    if app.is_help_open {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                app.is_help_open = false;
            }
            _ => {}
        }
        return;
    }

    // 9. Prefix chord check
    if is_prefix_chord_key(key, &app.read_model.effective_settings.tui.prefix_chord) {
        app.is_prefix_mode_active = true;
        app.status_message = format!(
            "Prefix chord active ({}). Press [c] chat, [o] orch, [h] hub, [s] settings, [?] help.",
            app.read_model.effective_settings.tui.prefix_chord
        );
        return;
    }

    // 10. Normal navigation
    handle_navigation_key(app, store, key);
}

fn handle_navigation_key(app: &mut AppState, store: &HubStore, key: event::KeyEvent) {
    if app.active_tab == TabIndex::HarnessPanes {
        harness_keys::handle_harness_workspace_key(app, store, key);
        return;
    }
    if app.active_tab == TabIndex::Settings && panel_keys::handle_settings_key(app, store, key) {
        return;
    }
    if app.active_tab == TabIndex::SharedHub && panel_keys::handle_shared_hub_key(app, key) {
        return;
    }
    if app.active_tab == TabIndex::ChatAndMemory
        && app.approvals.chat_view_mode == ChatViewMode::MemorySearch
        && panel_keys::handle_memory_key(app, store, key)
    {
        return;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        (KeyCode::Char('/'), _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            app.is_command_palette_open = true;
            app.command_input.clear();
        }
        (KeyCode::Char('?'), _) | (KeyCode::F(1), _) => {
            app.is_help_open = !app.is_help_open;
        }
        (KeyCode::Char('r'), _) => {
            app.refresh();
        }
        (KeyCode::Char('c'), KeyModifiers::NONE) => {
            app.open_composer();
        }
        (KeyCode::Char('s'), KeyModifiers::NONE) => {
            app.open_session_switcher();
        }
        (KeyCode::Char('n'), KeyModifiers::NONE) => {
            app.open_create_session();
        }
        (KeyCode::Char('I'), _) => {
            app.approvals.chat_view_mode = app.approvals.chat_view_mode.next();
            app.status_message =
                format!("Switched view to: {}", app.approvals.chat_view_mode.title());
        }
        (KeyCode::Char('a'), KeyModifiers::NONE) => {
            if !app.read_model.pending_gates.is_empty() {
                match app
                    .approvals
                    .resolve_selected(store, &app.read_model.pending_gates, true)
                {
                    Ok(res) => {
                        app.refresh();
                        app.status_message = format!(
                            "Approved wake gate request for {}.",
                            res.to_agents.join(", ")
                        );
                    }
                    Err(e) => {
                        app.status_message = format!("Approval error: {e}");
                    }
                }
            } else if !app.read_model.inbox_messages.is_empty() {
                match app
                    .approvals
                    .mark_selected_inbox_acked(store, &app.read_model.inbox_messages)
                {
                    Ok(msg) => {
                        app.refresh();
                        app.status_message =
                            format!("Acknowledged message from {}.", msg.from_agent);
                    }
                    Err(e) => {
                        app.status_message = format!("Acknowledge error: {e}");
                    }
                }
            }
        }
        (KeyCode::Char('d'), KeyModifiers::NONE) | (KeyCode::Char('x'), KeyModifiers::NONE) => {
            if !app.read_model.pending_gates.is_empty() {
                match app
                    .approvals
                    .resolve_selected(store, &app.read_model.pending_gates, false)
                {
                    Ok(res) => {
                        app.refresh();
                        app.status_message =
                            format!("Denied wake gate request for {}.", res.to_agents.join(", "));
                    }
                    Err(e) => {
                        app.status_message = format!("Denial error: {e}");
                    }
                }
            }
        }
        (KeyCode::Char('T'), _) => {
            app.cycle_theme();
        }
        (KeyCode::Tab, KeyModifiers::NONE)
        | (KeyCode::Char('l'), KeyModifiers::NONE)
        | (KeyCode::Right, KeyModifiers::NONE) => {
            app.active_tab = app.active_tab.next();
            app.scroll_offset = 0;
        }
        (KeyCode::BackTab, _)
        | (KeyCode::Tab, KeyModifiers::SHIFT)
        | (KeyCode::Char('h'), KeyModifiers::NONE)
        | (KeyCode::Left, KeyModifiers::NONE) => {
            app.active_tab = app.active_tab.prev();
            app.scroll_offset = 0;
        }
        (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            if app.active_tab == TabIndex::Orchestrate && !app.read_model.pending_gates.is_empty() {
                app.approvals
                    .select_next_approval(app.read_model.pending_gates.len());
            } else if app.active_tab == TabIndex::ChatAndMemory {
                match app.approvals.chat_view_mode {
                    super::approvals::ChatViewMode::PendingApprovals => {
                        app.approvals
                            .select_next_approval(app.read_model.pending_gates.len());
                    }
                    super::approvals::ChatViewMode::HumanInbox => {
                        app.approvals
                            .select_next_inbox(app.read_model.inbox_messages.len());
                    }
                    super::approvals::ChatViewMode::SessionMessages => {
                        app.scroll_offset = app.scroll_offset.saturating_add(1);
                    }
                    super::approvals::ChatViewMode::MemorySearch => {
                        app.memory.select_next();
                    }
                }
            } else {
                app.scroll_offset = app.scroll_offset.saturating_add(1);
            }
            app.selected_index = app.selected_index.saturating_add(1);
        }
        (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            if app.active_tab == TabIndex::Orchestrate && !app.read_model.pending_gates.is_empty() {
                app.approvals
                    .select_prev_approval(app.read_model.pending_gates.len());
            } else if app.active_tab == TabIndex::ChatAndMemory {
                match app.approvals.chat_view_mode {
                    super::approvals::ChatViewMode::PendingApprovals => {
                        app.approvals
                            .select_prev_approval(app.read_model.pending_gates.len());
                    }
                    super::approvals::ChatViewMode::HumanInbox => {
                        app.approvals
                            .select_prev_inbox(app.read_model.inbox_messages.len());
                    }
                    super::approvals::ChatViewMode::SessionMessages => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(1);
                    }
                    super::approvals::ChatViewMode::MemorySearch => {
                        app.memory.select_prev();
                    }
                }
            } else {
                app.scroll_offset = app.scroll_offset.saturating_sub(1);
            }
            app.selected_index = app.selected_index.saturating_sub(1);
        }
        (KeyCode::Char('g'), KeyModifiers::NONE) | (KeyCode::Home, KeyModifiers::NONE) => {
            app.scroll_offset = 0;
            app.selected_index = 0;
        }
        (KeyCode::Char('G'), KeyModifiers::NONE) | (KeyCode::End, KeyModifiers::NONE) => {
            app.scroll_offset = 100;
        }
        (KeyCode::Char('1'), _) => {
            app.active_tab = TabIndex::Orchestrate;
            app.scroll_offset = 0;
        }
        (KeyCode::Char('2'), _) => {
            app.active_tab = TabIndex::ChatAndMemory;
            app.scroll_offset = 0;
        }
        (KeyCode::Char('3'), _) => {
            app.active_tab = TabIndex::SharedHub;
            app.scroll_offset = 0;
        }
        (KeyCode::Char('4'), _) => {
            app.active_tab = TabIndex::Settings;
            app.scroll_offset = 0;
        }
        (KeyCode::Char('5'), _) => {
            app.active_tab = TabIndex::HarnessPanes;
            app.scroll_offset = 0;
        }
        (KeyCode::Esc, _) => {
            app.should_quit = true;
        }
        _ => {}
    }
}

pub fn is_prefix_chord_key(key: event::KeyEvent, configured: &str) -> bool {
    let clean = configured.trim().to_lowercase();
    let (target_code, target_mods) = match clean.as_str() {
        "ctrl+a" => (KeyCode::Char('a'), KeyModifiers::CONTROL),
        "ctrl+x" => (KeyCode::Char('x'), KeyModifiers::CONTROL),
        "ctrl+g" => (KeyCode::Char('g'), KeyModifiers::CONTROL),
        _ => (KeyCode::Char('b'), KeyModifiers::CONTROL),
    };
    key.code == target_code && key.modifiers.contains(target_mods)
}

pub fn is_ascii_terminal() -> bool {
    if let Ok(lang) = std::env::var("LANG") {
        let lower = lang.to_lowercase();
        if lower.contains("ascii") || (lower.contains("c") && !lower.contains("utf")) {
            return true;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        if term == "linux" || term == "dumb" {
            return true;
        }
    }
    false
}
