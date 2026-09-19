//! Integration tests for TUI T4: session and orchestration workflows (#138).

use hub::{HubStore, MessageKind, MessageStatus, SettingsStore};
use std::path::PathBuf;
use tempfile::tempdir;
use tui::app::approvals::ChatViewMode;
use tui::app::composer::RecipientMode;
use tui::app::state::AppState;
use tui::{HubReadModel, TuiOptions};

fn setup_test_env() -> (tempfile::TempDir, PathBuf, HubStore, AppState) {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let store = HubStore::open(&home_path).unwrap();

    // Enroll standard agents
    store.upsert_agent("claude", "Claude").unwrap();
    store.set_team_member("claude", true).unwrap();
    store.upsert_agent("grok", "Grok").unwrap();
    store.set_team_member("grok", true).unwrap();
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
fn test_create_and_load_work_session() {
    let (_dir, _home_path, store, mut app) = setup_test_env();

    // 1. Open create session dialog
    app.open_create_session();
    assert!(app.create_session.is_open);
    assert_eq!(app.create_session.focused_field, 0);

    // Enter name
    app.create_session.name_input = String::from("feature-refactor");
    // Ensure grok and claude are selected
    app.create_session
        .selected_members
        .insert("grok".into(), true);
    app.create_session
        .selected_members
        .insert("claude".into(), true);

    // Create session
    let created = app.create_session.create_session(&store, None).unwrap();
    assert_eq!(created.name, "feature-refactor");
    assert!(!app.create_session.is_open);

    // 2. Load session
    app.load_session(created.id.clone());
    assert_eq!(app.session_id.as_deref(), Some(created.id.as_str()));
    assert!(app.status_message.contains("feature-refactor"));

    // Verify channel messages read model points to this session
    assert!(app
        .read_model
        .work_sessions
        .iter()
        .any(|s| s.id == created.id));
}

#[test]
fn test_composer_all_subset_single_modes() {
    let (_dir, _home_path, _store, mut app) = setup_test_env();

    app.open_composer();
    assert!(app.composer.is_open);

    let roster = &app.read_model.team_members;

    // 1. RecipientMode::All
    app.composer.recipient_mode = RecipientMode::All;
    let targets = app.composer.resolve_targets(roster, None);
    assert!(targets.contains(&"claude".to_string()));
    assert!(targets.contains(&"grok".to_string()));
    assert!(targets.contains(&"gemini".to_string()));
    assert!(!targets.contains(&"human".to_string()));

    // 2. RecipientMode::Subset
    app.composer.recipient_mode = RecipientMode::Subset;
    app.composer.selected_subset.clear();
    app.composer.selected_subset.insert("claude".into(), true);
    app.composer.selected_subset.insert("grok".into(), false);
    app.composer.selected_subset.insert("gemini".into(), true);
    let subset_targets = app.composer.resolve_targets(roster, None);
    assert_eq!(
        subset_targets,
        vec!["claude".to_string(), "gemini".to_string()]
    );

    // 3. RecipientMode::Single
    app.composer.recipient_mode = RecipientMode::Single;
    app.composer.single_recipient = "gemini".to_string();
    let single_targets = app.composer.resolve_targets(roster, None);
    assert_eq!(single_targets, vec!["gemini".to_string()]);
}

#[test]
fn test_c11_task_validation_rejects_non_present_targets() {
    let (_dir, _home_path, _store, mut app) = setup_test_env();

    app.open_composer();
    app.composer.body = String::from("Deploy feature v2");
    app.composer.recipient_mode = RecipientMode::Single;
    app.composer.single_recipient = "unknown_agent_99".to_string();
    app.composer.is_task_tag = true;

    // Task-tagged message must fail C11 validation if target is not on team
    let result = app
        .composer
        .check_confirmation_and_validation(&app.read_model.team_members, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("Task target not on team"));
    assert!(err.contains("Task stays non-spawning"));
}

#[test]
fn test_confirmation_defaults_for_wakes_and_broadcasts() {
    let (_dir, _home_path, _store, mut app) = setup_test_env();

    app.open_composer();
    app.composer.body = String::from("Wakeup and run");
    app.composer.recipient_mode = RecipientMode::Single;
    app.composer.single_recipient = "claude".to_string();
    app.composer.is_wake_tag = true;

    // Wakes confirm by default
    let result = app
        .composer
        .check_confirmation_and_validation(&app.read_model.team_members, None);
    assert_eq!(result, Ok(false));
    assert!(app.composer.confirmation_prompt.is_some());

    // Broadcasts confirm by default
    app.composer.is_wake_tag = false;
    app.composer.recipient_mode = RecipientMode::All;
    let result_broadcast = app
        .composer
        .check_confirmation_and_validation(&app.read_model.team_members, None);
    assert_eq!(result_broadcast, Ok(false));
    assert!(app.composer.confirmation_prompt.is_some());
}

#[test]
fn test_composer_execute_send_produces_delivery_outcomes() {
    let (_dir, _home_path, store, mut app) = setup_test_env();

    app.open_composer();
    app.composer.body = String::from("Hello team");
    app.composer.recipient_mode = RecipientMode::Single;
    app.composer.single_recipient = "claude".to_string();
    app.composer.is_task_tag = false;
    app.composer.is_wake_tag = false;

    // Execute Send
    let outcomes = app
        .composer
        .execute_send(&store, None, None, &app.read_model.team_members, None)
        .unwrap();

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].to_agent, "claude");
    assert!(outcomes[0].accepted);
    assert!(app.composer.body.is_empty());
    assert!(app.composer.delivery_outcomes.is_some());
}

#[test]
fn test_wake_gate_approval_and_denial() {
    let (_dir, _home_path, store, mut app) = setup_test_env();

    // Setup role with 0 wake quota to trigger gate
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

    // Trigger gated send
    let outcomes = store
        .send_tagged_message_gated(
            "gemini",
            &["claude".to_string()],
            false,
            true,
            "Urgent wake message body",
            Some("urgent subject"),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(outcomes[0].policy_decision, "gate_pending_role_limit");

    app.refresh();
    assert!(!app.read_model.pending_gates.is_empty());

    // Resolve gate approval (Approve)
    app.approvals.selected_approval_index = 0;
    let approved = app
        .approvals
        .resolve_selected(&store, &app.read_model.pending_gates, true)
        .unwrap();
    assert_eq!(approved.status, "approved");

    app.refresh();
    assert!(app.read_model.pending_gates.is_empty());
}

#[test]
fn test_inbox_toggle_and_acknowledge() {
    let (_dir, _home_path, store, mut app) = setup_test_env();

    // Send a message to human
    let _msg = store
        .send_message(
            "grok",
            "human",
            MessageKind::Message,
            "Need input on PR #138",
            None,
            None,
            None,
        )
        .unwrap();

    app.refresh();
    assert!(!app.read_model.inbox_messages.is_empty());

    // Cycle chat view mode
    assert_eq!(app.approvals.chat_view_mode, ChatViewMode::SessionMessages);
    app.approvals.chat_view_mode = app.approvals.chat_view_mode.next();
    assert_eq!(app.approvals.chat_view_mode, ChatViewMode::HumanInbox);

    // Acknowledge inbox message
    app.approvals.selected_inbox_index = 0;
    let acked = app
        .approvals
        .mark_selected_inbox_acked(&store, &app.read_model.inbox_messages)
        .unwrap();
    assert_eq!(acked.status, MessageStatus::Acked.as_str());
}
