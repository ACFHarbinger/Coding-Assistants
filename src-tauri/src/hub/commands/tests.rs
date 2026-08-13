//! M6 acceptance gate (#82): a durable memory record written by one
//! caller must be retrievable through this Tauri command layer, not
//! just through the `ca` CLI that shares the same `HubStore`.
use super::quota_claude::{claude_home, claude_quota};
use super::quota_codex::now_unix;
use super::quota_grok::{grok_home, grok_quota, grok_token_from_auth, grok_windows_from_value};
use super::{memory::*, messaging::*, store::open_store};
use hub::{MemoryScope, MemoryTier, MessageKind};
use std::sync::Mutex;

#[test]
fn tagged_and_session_send_args_accept_tauri_camel_case_payloads() {
    let tagged: SendTaggedMessageArgs = serde_json::from_value(serde_json::json!({
        "from": "human",
        "to": ["grok"],
        "isTask": true,
        "isWake": false,
        "subject": "channel:session:example:task",
        "workspace": null,
        "task": "review",
        "sessionId": "example",
        "body": "Please review this."
    }))
    .unwrap();
    assert!(tagged.is_task);
    assert!(!tagged.is_wake);
    assert_eq!(tagged.session_id.as_deref(), Some("example"));

    let session: SendSessionMessageArgs = serde_json::from_value(serde_json::json!({
        "from": "human",
        "sessionId": "example",
        "to": ["grok"],
        "subject": "channel:session:example:message",
        "workspace": null,
        "task": null,
        "body": "Status update"
    }))
    .unwrap();
    assert_eq!(session.session_id, "example");
}

/// `open_store()` reads the process-global `CA_HOME` env var, so any
/// test that sets it must not run concurrently with another one doing
/// the same (Rust's default test runner is multi-threaded within one
/// binary). Every test below acquires this before touching `CA_HOME`.
static CA_HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn tauri_hub_commands_retrieve_what_the_store_wrote() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-test-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);

    let store = open_store().expect("open_store should create the hub dir");
    store
        .write_memory(
            MemoryTier::Semantic,
            MemoryScope::Workspace,
            Some("claude"),
            Some("Coding-Assistants"),
            Some("M6 desktop-layer check"),
            "written directly against HubStore, must surface via hub_list_memories",
            &["m6".to_string()],
        )
        .expect("write_memory should succeed");

    let listed = hub_list_memories(
        Some("workspace".into()),
        None,
        Some("Coding-Assistants".into()),
        None,
    )
    .expect("hub_list_memories should succeed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title.as_deref(), Some("M6 desktop-layer check"));

    let found = hub_search_memories("desktop-layer check".into())
        .expect("hub_search_memories should succeed");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, listed[0].id);

    std::env::remove_var("CA_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ca102_hub_commands_return_only_the_requested_channel_and_linked_memories() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-ca102-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);

    let store = open_store().expect("open_store should create the hub dir");
    let memory = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("human"),
            None,
            Some("Linked chat decision"),
            "The Messager chat should remain the central conversation surface.",
            &[],
        )
        .expect("write_memory should succeed");
    let general = store
        .send_message(
            "human",
            "team",
            MessageKind::Message,
            &format!("Decision recorded: [Memory #{}]", memory.id),
            Some("channel:general"),
            None,
            None,
        )
        .expect("send_message should succeed");
    store
        .send_message(
            "human",
            "team",
            MessageKind::Message,
            "This belongs in a separate channel.",
            Some("channel:engineering"),
            None,
            None,
        )
        .expect("send_message should succeed");

    let channel = hub_list_channel_messages("general".into(), Some(10))
        .expect("hub_list_channel_messages should succeed");
    assert_eq!(channel.len(), 1);
    assert_eq!(channel[0].id, general.id);

    let linked = hub_list_message_memories(general.id)
        .expect("hub_list_message_memories should resolve the message reference");
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].id, memory.id);

    std::env::remove_var("CA_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ca106_hub_commands_edit_delete_every_copy_and_reject_non_human_authors() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-ca106-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);

    let store = open_store().expect("open_store should create the hub dir");
    store.set_team_member("claude", true).unwrap();
    store.set_team_member("grok", true).unwrap();

    let posted = store
        .send_message_to_team(
            "human",
            hub::MessageKind::Message,
            "hi",
            Some("channel:general:22222222-2222-2222-2222-222222222222"),
            None,
            None,
        )
        .expect("send_message_to_team should succeed");
    assert!(posted.len() >= 2, "{posted:?}");

    let edited = hub_update_message(posted[0].id.clone(), "hi (edited)".into())
        .expect("hub_update_message should succeed for a human-authored post");
    assert_eq!(edited.len(), posted.len());
    assert!(edited.iter().all(|m| m.body == "hi (edited)"));

    let deleted = hub_delete_message(posted[0].id.clone())
        .expect("hub_delete_message should succeed for a human-authored post");
    assert_eq!(deleted, posted.len());
    for original in &posted {
        let refreshed = store.get_message(&original.id).unwrap().unwrap();
        assert_eq!(refreshed.status, "cancelled");
    }

    let agent_authored = store
        .send_message(
            "grok",
            "human",
            hub::MessageKind::Message,
            "not yours",
            None,
            None,
            None,
        )
        .expect("send_message should succeed");
    let rejected = hub_update_message(agent_authored.id.clone(), "rewritten".into());
    assert!(
        rejected.is_err(),
        "expected agent-authored edit to be rejected"
    );
    assert_eq!(
        store.get_message(&agent_authored.id).unwrap().unwrap().body,
        "not yours",
        "an agent's message must not be silently rewritten"
    );

    std::env::remove_var("CA_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ca111_audit_tab_lists_pending_and_can_approve_or_quarantine() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-ca111-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);

    let store = open_store().expect("open_store should create the hub dir");
    let watched = store
        .record_audit_event(
            std::path::Path::new("/workspace"),
            std::path::Path::new("/workspace/src/lib.rs"),
            "modified",
            r#"{"pid":1234,"name":"vim"}"#,
            None,
        )
        .expect("record_audit_event should succeed");
    let to_quarantine = store
        .record_audit_event(
            std::path::Path::new("/workspace"),
            std::path::Path::new("/workspace/suspicious.sh"),
            "created",
            r#"{"pid":5678,"name":"unknown"}"#,
            None,
        )
        .expect("record_audit_event should succeed");

    let pending = hub_list_audit_events(Some(true)).expect("hub_list_audit_events should succeed");
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|e| e.status == "pending"));

    hub_approve_audit(watched.id.clone()).expect("hub_approve_audit should succeed");
    hub_quarantine_audit(to_quarantine.id.clone()).expect("hub_quarantine_audit should succeed");

    let remaining_pending =
        hub_list_audit_events(Some(true)).expect("hub_list_audit_events should succeed");
    assert!(remaining_pending.is_empty(), "{remaining_pending:?}");

    let all = hub_list_audit_events(Some(false)).expect("hub_list_audit_events should succeed");
    assert_eq!(
        all.iter().find(|e| e.id == watched.id).unwrap().status,
        "approved"
    );
    assert_eq!(
        all.iter()
            .find(|e| e.id == to_quarantine.id)
            .unwrap()
            .status,
        "quarantined"
    );

    std::env::remove_var("CA_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Real (not mocked) smoke test against the live, undocumented Claude
/// usage endpoint — skips instead of failing when this machine has no
/// logged-in Claude Code CLI, since that's real environment-dependent
/// state, not something to fake. Where it *can* run, it must never
/// panic and must return a well-formed struct even if the private
/// endpoint's shape has drifted since this was written.
#[test]
fn claude_quota_is_well_formed_when_logged_in() {
    if !claude_home().join(".credentials.json").exists() {
        eprintln!("skipping: no ~/.claude/.credentials.json on this machine");
        return;
    }
    let quota = claude_quota();
    assert_eq!(quota.agent_id, "claude");
    assert_eq!(quota.provider, "anthropic");
    match quota.status.as_str() {
        "ok" => {
            assert!(!quota.windows.is_empty(), "status ok but no windows");
            for window in &quota.windows {
                assert!(
                    (0..=100).contains(&window.used_percent),
                    "{window:?} out of range"
                );
                assert_eq!(window.used_percent + window.remaining_percent, 100);
            }
        }
        "unavailable" => {
            assert!(quota.detail.is_some(), "unavailable status with no detail");
        }
        other => panic!("unexpected status: {other}"),
    }
}

#[test]
fn grok_token_prefers_accounts_sign_in_key() {
    let auth = serde_json::json!({
        "https://accounts.x.ai/sign-in": {
            "key": "session-token-from-grok-login-xyz"
        },
        "access_token": "should-not-win-over-sign-in-key"
    });
    assert_eq!(
        grok_token_from_auth(&auth).as_deref(),
        Some("session-token-from-grok-login-xyz")
    );
    let alt = serde_json::json!({
        "https://auth.x.ai/callback": { "access_token": "oidc-access-token-value-xx" }
    });
    assert_eq!(
        grok_token_from_auth(&alt).as_deref(),
        Some("oidc-access-token-value-xx")
    );
}

#[test]
fn grok_windows_parse_weekly_credit_snapshot() {
    let payload = serde_json::json!({
        "isUnifiedBillingUser": true,
        "creditUsagePercent": 37.4,
        "currentPeriod": "WEEKLY",
        "billingPeriodStart": "2026-08-10T00:00:00Z",
        "billingPeriodEnd": "2026-08-17T00:00:00Z",
        "onDemandUsed": 2.5,
        "onDemandCap": 10.0,
        "history": [
            { "creditUsagePercent": 99, "currentPeriod": "WEEKLY" }
        ]
    });
    let windows = grok_windows_from_value(&payload);
    assert_eq!(windows.len(), 2, "{windows:?}");
    assert_eq!(windows[0].label, "Weekly");
    assert_eq!(windows[0].used_percent, 37);
    assert_eq!(windows[0].remaining_percent, 63);
    assert_eq!(windows[0].window_minutes, Some(7 * 24 * 60));
    assert_eq!(windows[0].resets_at, Some(1_786_924_800));
    assert_eq!(windows[1].label, "Extra usage credits");
    assert_eq!(windows[1].used_percent, 25);
    assert_eq!(windows[1].remaining_percent, 75);
}

#[test]
fn grok_quota_is_well_formed_when_logged_in() {
    if !grok_home().join("auth.json").exists() {
        eprintln!("skipping: no ~/.grok/auth.json on this machine");
        return;
    }
    let quota = grok_quota();
    assert_eq!(quota.agent_id, "grok");
    assert_eq!(quota.provider, "xai");
    match quota.status.as_str() {
        "ok" => {
            assert!(!quota.windows.is_empty(), "status ok but no windows");
            for window in &quota.windows {
                assert!(
                    (0..=100).contains(&window.used_percent),
                    "{window:?} out of range"
                );
                assert_eq!(window.used_percent + window.remaining_percent, 100);
            }
        }
        "unavailable" => {
            assert!(quota.detail.is_some(), "unavailable status with no detail");
        }
        other => panic!("unexpected status: {other}"),
    }
}
