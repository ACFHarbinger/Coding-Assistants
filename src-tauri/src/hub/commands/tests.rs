//! M6 acceptance gate (#82): a durable memory record written by one
//! caller must be retrievable through this Tauri command layer, not
//! just through the `ca` CLI that shares the same `HubStore`.
use super::quota_codex::now_unix;
use super::settings::*;
use super::{memory::*, messaging::*, store::open_store};
use hub::{HarnessSettings, ProviderProfile, SecretReference};
use hub::{MemoryScope, MemoryTier, MessageKind};
use std::sync::Mutex;

#[path = "tests/quota.rs"]
mod quota;

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

#[test]
fn hub_send_message_rejects_untagged_wake_kind() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-wake-reject-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);
    let error = hub_send_message(SendMessageArgs {
        from: "human".into(),
        to: "grok".into(),
        kind: Some("wake".into()),
        subject: None,
        workspace: None,
        task: None,
        body: "must not bypass tagged send".into(),
    })
    .expect_err("untagged wake must be rejected");
    assert!(error.contains("hub_send_tagged_message"), "{error}");
    let _ = std::fs::remove_dir_all(&dir);
    std::env::remove_var("CA_HOME");
}

/// `open_store()` reads the process-global `CA_HOME` env var, so any
/// test that sets it must not run concurrently with another one doing
/// the same (Rust's default test runner is multi-threaded within one
/// binary). Every test below acquires this before touching `CA_HOME`.
/// `pub(crate)` so other `#[cfg(test)]` modules touching `CA_HOME` (e.g.
/// `crate::harness::commands::tests`) coordinate on the same lock.
pub(crate) static CA_HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

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

#[test]
fn settings_profile_and_harness_commands_are_redacted_and_durable() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-settings-s4-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);

    let listed = settings_upsert_profile(ProviderProfile {
        name: "work".into(),
        provider: "grok".into(),
        model: Some("grok-4".into()),
        base_url: Some("https://api.x.ai".into()),
        secret: SecretReference::EnvVar {
            name: "XAI_API_KEY".into(),
        },
    })
    .expect("upsert profile");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].secret_badge, "Env Var $XAI_API_KEY");
    let encoded = serde_json::to_string(&listed[0]).unwrap();
    assert!(!encoded.to_ascii_lowercase().contains("sk-"));
    assert!(!encoded.contains("Bearer"));

    let renamed = settings_rename_profile("work".into(), "office".into()).expect("rename");
    assert_eq!(renamed[0].name, "office");

    let effective =
        settings_set_workspace_default_profile("/abs/repo".into(), "grok".into(), "office".into())
            .expect("select default profile");
    let grok = effective
        .harnesses
        .iter()
        .find(|entry| entry.harness == "grok")
        .expect("grok harness");
    assert_eq!(grok.default_profile.as_deref(), Some("office"));
    assert_eq!(
        grok.default_profile_badge.as_deref(),
        Some("Env Var $XAI_API_KEY")
    );

    let harness = settings_update_harness(HarnessSettings {
        harness: "grok".into(),
        executable: "/usr/bin/grok".into(),
        workdir: Some("/abs/ws".into()),
        capture_polling: false,
        inject_permission: true,
    })
    .expect("update harness");
    assert_eq!(harness.executable, "/usr/bin/grok");
    assert!(!harness.capture_polling);

    let listed_harnesses =
        settings_list_harnesses(Some("/abs/repo".into())).expect("list harnesses");
    assert!(listed_harnesses.iter().any(|entry| entry.harness == "grok"));

    settings_reset_workspace_default_profile("/abs/repo".into(), "grok".into())
        .expect("reset default profile");
    settings_remove_profile("office".into()).expect("remove profile");
    assert!(settings_list_profiles()
        .expect("list after remove")
        .is_empty());

    let rejected = settings_upsert_profile(ProviderProfile {
        name: "bad".into(),
        provider: "grok".into(),
        model: Some("sk-secret".into()),
        base_url: None,
        secret: SecretReference::ProviderLogin,
    });
    assert!(rejected.is_err(), "{rejected:?}");

    let rejected_shell = settings_update_harness(HarnessSettings {
        harness: "grok".into(),
        executable: "grok && rm -rf /".into(),
        workdir: None,
        capture_polling: true,
        inject_permission: true,
    });
    assert!(rejected_shell.is_err(), "{rejected_shell:?}");

    std::env::remove_var("CA_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}

/// S5 / #131: `hub_export_markdown`/`hub_export_markdown_git` must consume
/// the persisted `export_enabled` orchestration policy, not just store it.
#[test]
fn export_commands_honor_the_persisted_export_enabled_policy() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-export-policy-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);

    // Export is enabled by default: a fresh install must not be blocked.
    settings_update_orchestration(
        None,
        OrchestrationPatch {
            confirm_new_enrollment: None,
            confirm_broadcast: None,
            auto_enrollment_allowed: None,
            sandbox_strictness: None,
            export_enabled: Some(false),
        },
    )
    .expect("disable export via Settings");

    let blocked = hub_export_markdown();
    assert!(blocked.is_err(), "{blocked:?}");
    assert!(blocked
        .unwrap_err()
        .contains("disabled by orchestration policy"));

    let blocked_git = hub_export_markdown_git(None);
    assert!(blocked_git.is_err(), "{blocked_git:?}");

    settings_update_orchestration(
        None,
        OrchestrationPatch {
            confirm_new_enrollment: None,
            confirm_broadcast: None,
            auto_enrollment_allowed: None,
            sandbox_strictness: None,
            export_enabled: Some(true),
        },
    )
    .expect("re-enable export via Settings");

    hub_export_markdown().expect("export must succeed once re-enabled");

    std::env::remove_var("CA_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}

/// S5 / #131 return: the legacy Shared Hub → Policy tab exposed both
/// `WakePolicy` fields (`default_requires_human_gate`, `allow_auto_wake`).
/// The unified Settings Orchestration flow must expose both too, so
/// retiring that tab doesn't drop `allow_auto_wake` editing capability.
#[test]
fn standing_policy_exposes_and_updates_both_wake_policy_fields() {
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "hub-tauri-standing-policy-{}-{}",
        std::process::id(),
        now_unix()
    ));
    std::env::set_var("CA_HOME", &dir);

    let defaults = settings_get_standing_policy(None).expect("get standing policy");
    assert!(defaults.confirm_wakes);
    assert!(defaults.allow_auto_wake);

    let after_gate = settings_set_confirm_wakes(false).expect("set confirm_wakes");
    assert!(!after_gate.confirm_wakes);
    assert!(
        after_gate.allow_auto_wake,
        "unrelated field must be untouched"
    );

    let after_auto = settings_set_allow_auto_wake(false).expect("set allow_auto_wake");
    assert!(!after_auto.allow_auto_wake);
    assert!(
        !after_auto.confirm_wakes,
        "the previous confirm_wakes change must survive this call"
    );

    let reread = settings_get_standing_policy(None).expect("re-read standing policy");
    assert!(!reread.confirm_wakes);
    assert!(!reread.allow_auto_wake);

    std::env::remove_var("CA_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}
