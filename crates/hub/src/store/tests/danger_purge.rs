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
