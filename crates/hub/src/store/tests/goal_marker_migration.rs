use super::super::*;
use tempfile::tempdir;

#[test]
fn goal_marker_migration_preserves_the_audit_chain_and_is_idempotent() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let message = store
        .send_message(
            "lead",
            "worker",
            MessageKind::Message,
            "[TASK] ship it",
            Some("channel:dev:kind:task"),
            None,
            None,
        )
        .unwrap();
    for metadata in [
        r#"{"body":"[TASK] ship it","subject":"channel:dev:kind:task"}"#,
        r#"{"body":"later event"}"#,
    ] {
        store
            .record_audit_event(
                dir.path(),
                Path::new("chat"),
                "message.sent",
                metadata,
                None,
            )
            .unwrap();
    }
    assert_eq!(store.verify_audit_chain().unwrap(), 2);
    let original_audit = serde_json::to_value(store.list_audit_events(false).unwrap()).unwrap();
    fs::create_dir_all(dir.path().join("journals")).unwrap();
    let journal = dir.path().join("journals/chat.md");
    fs::write(&journal, "[TASK] ship it").unwrap();
    drop(store);

    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../migrations/rename-task-marker-to-goal.sh");
    for _ in 0..2 {
        // Exercise the real one-shot migration only against a temporary Hub.
        let output = std::process::Command::new(&script)
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let store = HubStore::open(dir.path()).unwrap();
        assert_eq!(store.verify_audit_chain().unwrap(), 2);
        assert_eq!(
            serde_json::to_value(store.list_audit_events(false).unwrap()).unwrap(),
            original_audit
        );
        let migrated = store.get_message(&message.id).unwrap().unwrap();
        assert_eq!(migrated.body, "[GOAL] ship it");
        assert_eq!(migrated.subject.as_deref(), Some("channel:dev:kind:goal"));
        assert_eq!(fs::read_to_string(&journal).unwrap(), "[GOAL] ship it");
    }
}
