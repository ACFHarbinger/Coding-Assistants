//! Integration tests for TUI T5: Danger zone, Memory search, Audit journal, and Budgets (#139).

use hub::{HubStore, MemoryScope, MessageKind, SettingsStore};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use tui::app::danger_ops::{DangerAction, DangerButton};
use tui::app::state::{AppState, TabIndex};
use tui::app::views::HubViewMode;
use tui::{HubReadModel, TuiOptions};

fn setup_test_env() -> (tempfile::TempDir, PathBuf, HubStore, AppState) {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_path_buf();
    let store = HubStore::open(&home_path).unwrap();

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
fn test_danger_zone_cancel_first_and_exact_target_match() {
    let (dir, home_path, store, mut app) = setup_test_env();
    let ws_path = dir.path().join("my-workspace");
    fs::create_dir_all(&ws_path).unwrap();
    app.workspace_path = Some(ws_path.clone());
    let ws_str = ws_path.display().to_string();

    // 1. Cancel-first focus contract
    app.danger
        .open(DangerAction::ResetWorkspaceOverrides, "my-workspace".into());
    assert!(app.danger.is_open);
    assert_eq!(app.danger.focused_button, DangerButton::Cancel);
    assert!(!app.danger.is_matched());

    // Toggle button to Confirm
    app.danger.toggle_button();
    assert_eq!(app.danger.focused_button, DangerButton::Confirm);

    // Mismatch rejects
    app.danger.input_text = "wrong-workspace".into();
    assert!(!app.danger.is_matched());
    let err = app.danger.execute(&store, &home_path, Some(&ws_path));
    assert!(err.is_err());
    assert!(err
        .unwrap_err()
        .to_string()
        .contains("Type \"my-workspace\" exactly to confirm"));

    // Close
    app.danger.close();
    assert!(!app.danger.is_open);

    // 2. ResetWorkspaceOverrides (recoverable)
    let mut settings_store = SettingsStore::open(&home_path);
    settings_store
        .set_workspace_default_session(&ws_str, Some("custom-session"))
        .unwrap();
    settings_store.save().unwrap();
    assert_eq!(
        settings_store.effective(Some(&ws_str)).default_session,
        Some("custom-session".into())
    );

    app.danger
        .open(DangerAction::ResetWorkspaceOverrides, "my-workspace".into());
    app.danger.input_text = "my-workspace".into();
    app.danger.focused_button = DangerButton::Confirm;
    let msg = app
        .danger
        .execute(&store, &home_path, Some(&ws_path))
        .unwrap();
    assert!(msg.contains("reset to global defaults"));
    let refreshed_eff = SettingsStore::open(&home_path).effective(Some(&ws_str));
    assert_eq!(refreshed_eff.default_session, None);

    // 3. PurgeTranscript (irreversible)
    store
        .send_message(
            "claude",
            "gemini",
            MessageKind::Message,
            "secret transcript message",
            None,
            Some(&ws_str),
            None,
        )
        .unwrap();
    assert_eq!(store.list_messages(None, None).unwrap().len(), 1);

    app.danger
        .open(DangerAction::PurgeTranscript, "my-workspace".into());
    app.danger.input_text = "my-workspace".into();
    app.danger.focused_button = DangerButton::Confirm;
    let msg = app
        .danger
        .execute(&store, &home_path, Some(&ws_path))
        .unwrap();
    assert!(msg.contains("Purged 1 transcript message(s)"));
    assert_eq!(store.list_messages(None, None).unwrap().len(), 0);

    // 4. PurgeMemories (irreversible)
    store
        .write_memory(
            hub::MemoryTier::ShortTerm,
            MemoryScope::Workspace,
            Some("gemini"),
            Some(&ws_str),
            Some("test memory"),
            "memory body content",
            &["tag1".into()],
        )
        .unwrap();
    assert_eq!(
        store
            .list_memories(Some(MemoryScope::Workspace), None, Some(&ws_str), true)
            .unwrap()
            .len(),
        1
    );

    app.danger
        .open(DangerAction::PurgeMemories, "my-workspace".into());
    app.danger.input_text = "my-workspace".into();
    app.danger.focused_button = DangerButton::Confirm;
    let msg = app
        .danger
        .execute(&store, &home_path, Some(&ws_path))
        .unwrap();
    assert!(msg.contains("Purged 1 memory item(s)"));
    assert_eq!(
        store
            .list_memories(Some(MemoryScope::Workspace), None, Some(&ws_str), true)
            .unwrap()
            .len(),
        0
    );

    // 5. PurgeAllData (irreversible)
    store
        .send_message(
            "grok",
            "gemini",
            MessageKind::Message,
            "another message",
            None,
            Some(&ws_str),
            None,
        )
        .unwrap();
    store
        .write_memory(
            hub::MemoryTier::ShortTerm,
            MemoryScope::Workspace,
            Some("grok"),
            Some(&ws_str),
            Some("working memory"),
            "body",
            &[],
        )
        .unwrap();

    app.danger
        .open(DangerAction::PurgeAllData, "my-workspace".into());
    app.danger.input_text = "my-workspace".into();
    app.danger.focused_button = DangerButton::Confirm;
    let msg = app
        .danger
        .execute(&store, &home_path, Some(&ws_path))
        .unwrap();
    assert!(msg.contains("Purged 1 transcript message(s) and 1 memory item(s)"));

    // 6. DeleteProfile (irreversible)
    let mut settings_store = SettingsStore::open(&home_path);
    settings_store
        .upsert_profile(hub::ProviderProfile {
            name: "doomed-profile".into(),
            provider: "openai".into(),
            model: Some("gpt-4o".into()),
            base_url: None,
            secret: hub::SecretReference::EnvVar {
                name: "OPENAI_API_KEY".into(),
            },
        })
        .unwrap();
    settings_store.save().unwrap();
    assert!(settings_store.profile("doomed-profile").is_some());

    app.danger.open(
        DangerAction::DeleteProfile("doomed-profile".into()),
        "doomed-profile".into(),
    );
    app.danger.input_text = "doomed-profile".into();
    app.danger.focused_button = DangerButton::Confirm;
    let msg = app
        .danger
        .execute(&store, &home_path, Some(&ws_path))
        .unwrap();
    assert!(msg.contains("permanently deleted"));
    let refreshed_settings = SettingsStore::open(&home_path);
    assert!(refreshed_settings.profile("doomed-profile").is_none());
}

#[test]
fn test_memory_search_and_scope_filtering() {
    let (_dir, _home_path, store, mut app) = setup_test_env();

    store
        .write_memory(
            hub::MemoryTier::Semantic,
            MemoryScope::Global,
            Some("claude"),
            None,
            Some("Global Convention"),
            "All Rust code must be <= 500 LoC per file.",
            &["governance".into()],
        )
        .unwrap();

    store
        .write_memory(
            hub::MemoryTier::ShortTerm,
            MemoryScope::Workspace,
            Some("gemini"),
            Some("/repo/assistants"),
            Some("Workspace Build Instructions"),
            "Run cargo test -p tui to verify.",
            &["build".into()],
        )
        .unwrap();

    // 1. Empty query returns all memories
    app.memory.query.clear();
    let count = app.memory.execute_search(&store, None).unwrap();
    assert_eq!(count, 2);

    // 2. Query search
    app.memory.query = "500 LoC".into();
    let count = app.memory.execute_search(&store, None).unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        app.memory.results[0].title.as_deref(),
        Some("Global Convention")
    );

    // 3. Scope filtering
    app.memory.query.clear();
    app.memory.scope_filter = Some("workspace".into());
    let count = app
        .memory
        .execute_search(&store, Some(&PathBuf::from("/repo/assistants")))
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        app.memory.results[0].title.as_deref(),
        Some("Workspace Build Instructions")
    );

    // Cycle scope
    app.memory.cycle_scope();
    assert_eq!(app.memory.scope_filter, None);
}

#[test]
fn test_audit_journal_review_and_hash_chain() {
    let (_dir, _home_path, store, mut app) = setup_test_env();

    // Record verified audit events
    store
        .record_settings_audit_event("tui.theme", "global", "dracula")
        .unwrap();
    store
        .record_settings_audit_event("tui.high_contrast", "global", "true")
        .unwrap();

    app.refresh();
    assert_eq!(app.read_model.all_audit_events.len(), 2);

    let events = &app.read_model.all_audit_events;
    let mut chain_valid = true;
    for i in 1..events.len() {
        if events[i].previous_hash.as_deref() != Some(events[i - 1].event_hash.as_str()) {
            chain_valid = false;
            break;
        }
    }
    assert!(chain_valid, "Hash chain should be continuous and valid");

    // Test command palette navigation to audit and budgets
    app.command_input = "audit".into();
    app.execute_command();
    assert_eq!(app.active_tab, TabIndex::SharedHub);
    assert_eq!(app.hub_view_mode, HubViewMode::AuditJournal);

    app.command_input = "budgets".into();
    app.execute_command();
    assert_eq!(app.active_tab, TabIndex::SharedHub);
    assert_eq!(app.hub_view_mode, HubViewMode::Budgets);
}

#[test]
fn test_truthful_budgets_and_metrics() {
    let (_dir, _home_path, store, mut app) = setup_test_env();

    store.set_agent_budget("gemini", 1000.0).unwrap();

    app.refresh();
    assert_eq!(app.read_model.agent_budgets.len(), 1);
    let b = &app.read_model.agent_budgets[0];
    assert_eq!(b.agent_id, "gemini");
    assert_eq!(b.limit_units, 1000.0);
    assert!(!b.paused);
}
