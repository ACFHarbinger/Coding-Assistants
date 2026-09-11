//! S6 danger-zone workspace purge tests: transcript + memory purges delete
//! exactly one workspace's rows, return the count, and treat zero rows as a
//! successful no-op.

use super::super::*;
use tempfile::tempdir;

fn send_in(store: &HubStore, workspace: Option<&str>, body: &str) {
    store
        .send_message(
            "grok",
            "claude",
            MessageKind::Message,
            body,
            None,
            workspace,
            None,
        )
        .unwrap();
}

fn remember_in(store: &HubStore, workspace: Option<&str>, body: &str) {
    store
        .write_memory(
            MemoryTier::ShortTerm,
            MemoryScope::Global,
            Some("grok"),
            workspace,
            None,
            body,
            &[],
        )
        .unwrap();
}

#[test]
fn purge_messages_deletes_only_the_matching_workspace() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    send_in(&store, Some("/ws/a"), "a-1");
    send_in(&store, Some("/ws/a"), "a-2");
    send_in(&store, Some("/ws/b"), "b-1");
    send_in(&store, None, "unscoped");

    let deleted = store
        .purge_messages_in_workspace(std::path::Path::new("/ws/a"))
        .unwrap();
    assert_eq!(deleted, 2);

    let remaining = store.list_messages(None, None).unwrap();
    let bodies: Vec<&str> = remaining
        .iter()
        .map(|record| record.body.as_str())
        .collect();
    assert_eq!(bodies.len(), 2);
    assert!(bodies.contains(&"b-1"));
    assert!(bodies.contains(&"unscoped"));

    // Zero rows is a successful no-op, not a NotFound.
    let again = store
        .purge_messages_in_workspace(std::path::Path::new("/ws/a"))
        .unwrap();
    assert_eq!(again, 0);
}

#[test]
fn purge_memories_deletes_all_tiers_in_only_the_matching_workspace() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    remember_in(&store, Some("/ws/a"), "a-short");
    let stale = store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("grok"),
            Some("/ws/a"),
            None,
            "a-episodic",
            &[],
        )
        .unwrap();
    store.mark_memory_stale(&stale.id, true).unwrap();
    remember_in(&store, Some("/ws/b"), "b-short");
    remember_in(&store, None, "global");

    let deleted = store
        .purge_memories_in_workspace(std::path::Path::new("/ws/a"))
        .unwrap();
    assert_eq!(deleted, 2, "stale and fresh rows both go");

    let remaining = store.list_memories(None, None, None, true).unwrap();
    let bodies: Vec<&str> = remaining
        .iter()
        .map(|record| record.body.as_str())
        .collect();
    assert_eq!(bodies.len(), 2);
    assert!(bodies.contains(&"b-short"));
    assert!(bodies.contains(&"global"));
}

fn audit_events_for(store: &HubStore, field: &str, scope: &str) -> Vec<AuditEvent> {
    let expected = serde_json::json!({ "field": field, "scope": scope }).to_string();
    store
        .list_settings_audit_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.process_json == expected)
        .collect()
}

#[test]
fn audited_purges_commit_counts_with_an_approved_audit_row() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    send_in(&store, Some("/ws/a"), "a-1");
    remember_in(&store, Some("/ws/a"), "a-mem");

    assert_eq!(
        store
            .purge_messages_in_workspace_with_audit(std::path::Path::new("/ws/a"))
            .unwrap(),
        1
    );
    let transcript_audits = audit_events_for(&store, "transcript", "/ws/a");
    assert_eq!(transcript_audits.len(), 1);
    assert_eq!(transcript_audits[0].status, "approved");
    assert!(transcript_audits[0].operation.contains("purge:1"));

    assert_eq!(
        store
            .purge_memories_in_workspace_with_audit(std::path::Path::new("/ws/a"))
            .unwrap(),
        1
    );
    let memory_audits = audit_events_for(&store, "memory", "/ws/a");
    assert_eq!(memory_audits.len(), 1);
    assert_eq!(memory_audits[0].status, "approved");

    // The in-transaction audit inserts keep the hash chain valid.
    store.verify_audit_chain().unwrap();
}

#[test]
fn combined_purge_commits_both_halves_with_one_audit_row() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    send_in(&store, Some("/ws/a"), "a-1");
    send_in(&store, Some("/ws/a"), "a-2");
    remember_in(&store, Some("/ws/a"), "a-mem");

    let (messages, memories) = store
        .purge_workspace_data_with_audit(std::path::Path::new("/ws/a"))
        .unwrap();
    assert_eq!((messages, memories), (2, 1));

    assert!(store.list_messages(None, None).unwrap().is_empty());
    assert!(store
        .list_memories(None, None, None, true)
        .unwrap()
        .is_empty());
    let audits = audit_events_for(&store, "workspace-data", "/ws/a");
    assert_eq!(audits.len(), 1, "exactly one audit row for both halves");
    assert_eq!(audits[0].status, "approved");
    store.verify_audit_chain().unwrap();
}

fn abort_trigger(store: &HubStore, name: &str, sql: &str) {
    store
        .conn
        .execute_batch(&format!("CREATE TEMP TRIGGER {name} {sql}"))
        .unwrap();
}

fn drop_trigger(store: &HubStore, name: &str) {
    store
        .conn
        .execute_batch(&format!("DROP TRIGGER IF EXISTS {name}"))
        .unwrap();
}

#[test]
fn audited_transcript_purge_rolls_back_when_the_audit_insert_fails() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    send_in(&store, Some("/ws/a"), "a-1");

    abort_trigger(
        &store,
        "abort_audit",
        "BEFORE INSERT ON audit_events BEGIN SELECT RAISE(ABORT, 'injected audit failure'); END;",
    );
    let err = store
        .purge_messages_in_workspace_with_audit(std::path::Path::new("/ws/a"))
        .unwrap_err();
    assert!(err.to_string().contains("injected audit failure"));
    drop_trigger(&store, "abort_audit");

    // Deletion rolled back with the failed audit: row intact, nothing recorded.
    assert_eq!(store.list_messages(None, None).unwrap().len(), 1);
    assert!(audit_events_for(&store, "transcript", "/ws/a").is_empty());
    store.verify_audit_chain().unwrap();
}

#[test]
fn audited_memory_purge_rolls_back_when_the_audit_insert_fails() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    remember_in(&store, Some("/ws/a"), "a-mem");

    abort_trigger(
        &store,
        "abort_audit",
        "BEFORE INSERT ON audit_events BEGIN SELECT RAISE(ABORT, 'injected audit failure'); END;",
    );
    let err = store
        .purge_memories_in_workspace_with_audit(std::path::Path::new("/ws/a"))
        .unwrap_err();
    assert!(err.to_string().contains("injected audit failure"));
    drop_trigger(&store, "abort_audit");

    assert_eq!(
        store.list_memories(None, None, None, true).unwrap().len(),
        1
    );
    assert!(audit_events_for(&store, "memory", "/ws/a").is_empty());
    store.verify_audit_chain().unwrap();
}

#[test]
fn combined_purge_rolls_back_messages_when_memories_fail() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    send_in(&store, Some("/ws/a"), "a-1");
    remember_in(&store, Some("/ws/a"), "a-mem");
    let audits_before = store.list_settings_audit_events().unwrap().len();

    abort_trigger(
        &store,
        "abort_memories",
        "BEFORE DELETE ON memories BEGIN SELECT RAISE(ABORT, 'injected memories failure'); END;",
    );
    let err = store
        .purge_workspace_data_with_audit(std::path::Path::new("/ws/a"))
        .unwrap_err();
    assert!(err.to_string().contains("injected memories failure"));
    drop_trigger(&store, "abort_memories");

    // First half rolled back too: no partial completion, no audit row.
    assert_eq!(store.list_messages(None, None).unwrap().len(), 1);
    assert_eq!(
        store.list_memories(None, None, None, true).unwrap().len(),
        1
    );
    assert_eq!(
        store.list_settings_audit_events().unwrap().len(),
        audits_before
    );
    store.verify_audit_chain().unwrap();
}

#[cfg(unix)]
#[test]
fn purge_matches_the_canonicalized_path_for_symlinked_workspaces() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let real = dir.path().join("real-ws");
    std::fs::create_dir(&real).unwrap();
    let link = dir.path().join("link-ws");
    symlink(&real, &link).unwrap();

    let dir2 = tempdir().unwrap();
    let store = HubStore::open(dir2.path()).unwrap();
    // Rows recorded under the resolved path still purge via the symlink.
    send_in(&store, Some(&real.to_string_lossy()), "via-real");
    remember_in(&store, Some(&real.to_string_lossy()), "mem-via-real");

    assert_eq!(
        store.purge_messages_in_workspace(&link).unwrap(),
        1,
        "{link:?} must resolve to {real:?}"
    );
    assert_eq!(store.purge_memories_in_workspace(&link).unwrap(), 1);
}
