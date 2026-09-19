//! Modal key handlers for TUI (T4 / #138).

use crate::app::composer::RecipientMode;
use crate::app::state::AppState;
use crossterm::event::{self, KeyCode, KeyModifiers};
use hub::HubStore;

pub fn handle_delivery_outcomes_key(app: &mut AppState, key: event::KeyEvent) -> bool {
    if app.composer.delivery_outcomes.is_none() {
        return false;
    }
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
            app.composer.delivery_outcomes = None;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.composer.outcome_scroll = app.composer.outcome_scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.composer.outcome_scroll = app.composer.outcome_scroll.saturating_sub(1);
        }
        _ => {}
    }
    true
}

pub fn handle_confirmation_key(app: &mut AppState, store: &HubStore, key: event::KeyEvent) -> bool {
    if app.composer.confirmation_prompt.is_none() {
        return false;
    }
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            let active_sess = app
                .read_model
                .work_sessions
                .iter()
                .find(|s| app.session_id.as_deref() == Some(&s.id))
                .cloned();
            match app.composer.execute_send(
                store,
                app.workspace_path.as_deref(),
                app.session_id.as_deref(),
                &app.read_model.team_members,
                active_sess.as_ref(),
            ) {
                Ok(outcomes) => {
                    app.refresh();
                    app.status_message =
                        format!("Sent message ({} delivery outcomes).", outcomes.len());
                }
                Err(e) => {
                    app.composer.error_message = Some(e.to_string());
                }
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.composer.confirmation_prompt = None;
            app.status_message = String::from("Send cancelled.");
        }
        _ => {}
    }
    true
}

pub fn handle_session_switcher_key(app: &mut AppState, key: event::KeyEvent) -> bool {
    if !app.session_switcher.is_open {
        return false;
    }
    let total = app.read_model.work_sessions.len();
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.session_switcher.close();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.session_switcher.select_prev(total);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.session_switcher.select_next(total);
        }
        KeyCode::Enter => {
            if let Some(session) = app
                .read_model
                .work_sessions
                .get(app.session_switcher.selected_index)
            {
                let sid = session.id.clone();
                app.load_session(sid);
            }
        }
        KeyCode::Char('n') => {
            app.session_switcher.close();
            app.open_create_session();
        }
        _ => {}
    }
    true
}

pub fn handle_create_session_key(
    app: &mut AppState,
    store: &HubStore,
    key: event::KeyEvent,
) -> bool {
    if !app.create_session.is_open {
        return false;
    }
    match key.code {
        KeyCode::Esc => {
            app.create_session.close();
        }
        KeyCode::Tab => {
            app.create_session.focused_field = (app.create_session.focused_field + 1) % 3;
        }
        KeyCode::BackTab => {
            app.create_session.focused_field = (app.create_session.focused_field + 2) % 3;
        }
        _ => {
            let keys: Vec<String> = app
                .create_session
                .selected_members
                .keys()
                .cloned()
                .collect();
            match app.create_session.focused_field {
                0 => match key.code {
                    KeyCode::Backspace => {
                        app.create_session.name_input.pop();
                    }
                    KeyCode::Char(c) => {
                        app.create_session.name_input.push(c);
                    }
                    KeyCode::Enter => {
                        if let Ok(sess) = app.create_session.create_session(store, None) {
                            app.load_session(sess.id);
                        }
                    }
                    _ => {}
                },
                1 => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        app.create_session.select_prev_member(keys.len());
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        app.create_session.select_next_member(keys.len());
                    }
                    KeyCode::Char(' ') => {
                        app.create_session.toggle_current_member(&keys);
                    }
                    KeyCode::Enter => {
                        if let Ok(sess) = app.create_session.create_session(store, None) {
                            app.load_session(sess.id);
                        }
                    }
                    _ => {}
                },
                _ => {
                    if key.code == KeyCode::Enter {
                        if let Ok(sess) = app.create_session.create_session(store, None) {
                            app.load_session(sess.id);
                        }
                    }
                }
            }
        }
    }
    true
}

pub fn handle_composer_key(app: &mut AppState, store: &HubStore, key: event::KeyEvent) -> bool {
    if !app.composer.is_open {
        return false;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('t') => {
                app.composer.toggle_task_tag();
                return true;
            }
            KeyCode::Char('w') => {
                app.composer.toggle_wake_tag();
                return true;
            }
            KeyCode::Char('r') => {
                app.composer.cycle_recipient_mode();
                return true;
            }
            KeyCode::Char('s') => {
                trigger_send(app, store);
                return true;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Esc => {
            app.composer.close();
        }
        KeyCode::Tab => {
            app.composer.cycle_recipient_mode();
        }
        KeyCode::Left => match app.composer.recipient_mode {
            RecipientMode::Subset => {
                let count = app.composer.selected_subset.len();
                if count > 0 {
                    if app.composer.subset_cursor == 0 {
                        app.composer.subset_cursor = count - 1;
                    } else {
                        app.composer.subset_cursor -= 1;
                    }
                }
            }
            RecipientMode::Single => {
                cycle_single_recipient(app, false);
            }
            _ => {}
        },
        KeyCode::Right => match app.composer.recipient_mode {
            RecipientMode::Subset => {
                let count = app.composer.selected_subset.len();
                if count > 0 {
                    app.composer.subset_cursor = (app.composer.subset_cursor + 1) % count;
                }
            }
            RecipientMode::Single => {
                cycle_single_recipient(app, true);
            }
            _ => {}
        },
        KeyCode::Enter => {
            trigger_send(app, store);
        }
        KeyCode::Backspace => {
            app.composer.body.pop();
        }
        KeyCode::Char(c) => {
            app.composer.body.push(c);
        }
        _ => {}
    }
    true
}

pub fn trigger_send(app: &mut AppState, store: &HubStore) {
    let active_sess = app
        .read_model
        .work_sessions
        .iter()
        .find(|s| app.session_id.as_deref() == Some(&s.id))
        .cloned();

    match app
        .composer
        .check_confirmation_and_validation(&app.read_model.team_members, active_sess.as_ref())
    {
        Ok(true) => {
            match app.composer.execute_send(
                store,
                app.workspace_path.as_deref(),
                app.session_id.as_deref(),
                &app.read_model.team_members,
                active_sess.as_ref(),
            ) {
                Ok(outcomes) => {
                    app.refresh();
                    app.status_message =
                        format!("Sent message ({} delivery outcomes).", outcomes.len());
                }
                Err(e) => {
                    app.composer.error_message = Some(e.to_string());
                }
            }
        }
        Ok(false) => {
            // Confirmation modal is active
        }
        Err(err) => {
            app.composer.error_message = Some(err);
        }
    }
}

fn cycle_single_recipient(app: &mut AppState, forward: bool) {
    let candidates: Vec<String> = app
        .read_model
        .team_members
        .iter()
        .filter(|a| a.id != "human" && a.id != "system")
        .map(|a| a.id.clone())
        .collect();
    if candidates.is_empty() {
        return;
    }
    let current_idx = candidates
        .iter()
        .position(|id| id == &app.composer.single_recipient)
        .unwrap_or(0);
    let next_idx = if forward {
        (current_idx + 1) % candidates.len()
    } else if current_idx == 0 {
        candidates.len() - 1
    } else {
        current_idx - 1
    };
    app.composer.single_recipient = candidates[next_idx].clone();
}
