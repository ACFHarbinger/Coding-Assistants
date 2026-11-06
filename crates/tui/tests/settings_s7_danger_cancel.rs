//! S7 (#133): a wrong or empty typed confirmation must not purge transcripts,
//! memories, or settings. The gate is `DangerModalState::execute`. This is
//! not a UI harness.

use hub::{HubStore, MemoryScope, MemoryTier, MessageKind, SettingsStore};
use std::path::Path;
use tempfile::tempdir;
use tui::app::danger_ops::{DangerAction, DangerModalState};

fn reject(
    danger: &mut DangerModalState,
    store: &HubStore,
    home: &Path,
    workspace: &Path,
    typed: &str,
) {
    danger.input_text = typed.to_string();
    let err = danger
        .execute(store, home, Some(workspace))
        .expect_err("mismatched confirmation must not run the action");
    assert!(err.to_string().contains("exactly to confirm"), "{err}");
    assert!(
        danger.is_open,
        "a rejected confirmation stays on the dialog"
    );
}

#[test]
fn wrong_or_empty_confirmation_does_not_delete_data() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let workspace = dir.path().join("my-workspace");
    std::fs::create_dir(&workspace).unwrap();
    let ws = workspace.display().to_string();

    let hub = HubStore::open(&home).unwrap();
    hub.send_message(
        "grok",
        "claude",
        MessageKind::Message,
        "keep this transcript",
        None,
        Some(&ws),
        None,
    )
    .unwrap();
    hub.write_memory(
        MemoryTier::ShortTerm,
        MemoryScope::Workspace,
        Some("grok"),
        Some(&ws),
        Some("keep"),
        "keep this memory",
        &[],
    )
    .unwrap();

    let mut settings = SettingsStore::open(&home);
    settings
        .set_workspace_default_session(&ws, Some("kept-session"))
        .unwrap();
    settings
        .upsert_profile(hub::ProviderProfile {
            name: "kept-profile".into(),
            provider: "grok".into(),
            model: None,
            base_url: None,
            secret: hub::SecretReference::EnvVar {
                name: "XAI_API_KEY".into(),
            },
        })
        .unwrap();
    settings.save().unwrap();
    let settings_bytes = std::fs::read(settings.path()).unwrap();

    let mut danger = DangerModalState::default();
    let actions = [
        DangerAction::PurgeTranscript,
        DangerAction::PurgeMemories,
        DangerAction::PurgeAllData,
        DangerAction::ResetWorkspaceOverrides,
        DangerAction::DeleteProfile("kept-profile".into()),
    ];
    for action in actions {
        let target = match &action {
            DangerAction::DeleteProfile(name) => name.clone(),
            _ => "my-workspace".into(),
        };
        danger.open(action, target);
        reject(&mut danger, &hub, &home, &workspace, "");
        reject(&mut danger, &hub, &home, &workspace, "   ");
        reject(&mut danger, &hub, &home, &workspace, "nope");
    }

    assert_eq!(hub.list_messages(None, None).unwrap().len(), 1);
    assert_eq!(
        hub.list_memories(Some(MemoryScope::Workspace), None, Some(&ws), true)
            .unwrap()
            .len(),
        1
    );
    let reloaded = SettingsStore::open(&home);
    assert_eq!(
        reloaded.effective(Some(&ws)).default_session.as_deref(),
        Some("kept-session")
    );
    assert!(reloaded.profile("kept-profile").is_some());
    assert_eq!(std::fs::read(reloaded.path()).unwrap(), settings_bytes);
}
