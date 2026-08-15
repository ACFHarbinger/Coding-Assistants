//! Audit-chain and settings-audit tests (split from tests/roster.rs
//! for the 500-LoC cap, #158).

use super::super::*;
use tempfile::tempdir;

#[test]
fn audit_events_are_reviewable_and_hash_chained() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path().join("watched");
    fs::create_dir_all(&root).unwrap();

    let first = store
        .record_audit_event(
            &root,
            Path::new("journals/chat.md"),
            "modified",
            r#"{"pid":123,"attribution":"test"}"#,
            Some("abc"),
        )
        .unwrap();
    let second = store
        .record_audit_event(
            &root,
            Path::new("journals/chat.md"),
            "modified",
            r#"{"pid":123,"attribution":"test"}"#,
            Some("def"),
        )
        .unwrap();
    assert_eq!(
        second.previous_hash.as_deref(),
        Some(first.event_hash.as_str())
    );
    assert_eq!(store.verify_audit_chain().unwrap(), 2);
    assert_eq!(store.list_audit_events(true).unwrap().len(), 2);
    store.set_audit_status(&first.id, "approved").unwrap();
    assert_eq!(store.list_audit_events(true).unwrap().len(), 1);
    assert!(store.set_audit_status("missing", "approved").is_err());
}

#[test]
fn settings_audit_events_are_approved_and_isolated_from_other_audit_rows() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path().join("watched");
    fs::create_dir_all(&root).unwrap();

    // A non-settings audit row (e.g. an observed filesystem change) should
    // not leak into the settings-scoped view.
    store
        .record_audit_event(&root, Path::new("journals/chat.md"), "modified", "{}", None)
        .unwrap();

    let event = store
        .record_settings_audit_event("storage.backup_retention", "global", "update")
        .unwrap();
    assert_eq!(event.root_path, "settings");
    assert_eq!(event.path, "storage.backup_retention");
    assert_eq!(event.operation, "update");
    assert_eq!(event.status, "approved");
    assert!(event.process_json.contains("\"scope\":\"global\""));
    assert!(!event.process_json.contains("backup_retention\":"));

    let settings_events = store.list_settings_audit_events().unwrap();
    assert_eq!(settings_events.len(), 1);
    assert_eq!(settings_events[0].id, event.id);

    // The event is still on the shared Hub audit chain alongside the
    // filesystem row, just filterable by root_path.
    assert_eq!(store.list_audit_events(false).unwrap().len(), 2);
    assert_eq!(store.verify_audit_chain().unwrap(), 2);
}
