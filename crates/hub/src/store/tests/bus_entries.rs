//! Shared bus-entries table tests (C15 / #330).

use super::super::*;
use tempfile::tempdir;

fn open_store() -> (tempfile::TempDir, HubStore) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    (dir, store)
}

#[test]
fn append_defaults_blank_topic_to_log_and_round_trips() {
    let (_dir, store) = open_store();
    let record = store
        .append_bus_entry("muse", None, "claimed C15", Some("#330"), None)
        .unwrap();
    assert_eq!(record.topic, "log");
    assert_eq!(record.issue_ref.as_deref(), Some("#330"));
    assert!(record.task_id.is_none());
    let rows = store.list_bus_entries(&BusEntryFilter::default()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, record.id);
    assert_eq!(rows[0].body, "claimed C15");
}

#[test]
fn empty_agent_or_body_is_rejected() {
    let (_dir, store) = open_store();
    assert!(store
        .append_bus_entry("", None, "body", None, None)
        .is_err());
    assert!(store
        .append_bus_entry("muse", None, "   ", None, None)
        .is_err());
    let rows = store.list_bus_entries(&BusEntryFilter::default()).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn filters_combine_and_newest_comes_first() {
    let (_dir, store) = open_store();
    store
        .append_bus_entry("muse", Some("rfr"), "first", Some("#330"), None)
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    store
        .append_bus_entry("grok", Some("task-board"), "second", None, Some("task-1"))
        .unwrap();

    let all = store.list_bus_entries(&BusEntryFilter::default()).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].body, "second", "newest first");

    let by_agent = store
        .list_bus_entries(&BusEntryFilter {
            agent: Some("muse".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(by_agent.len(), 1);
    assert_eq!(by_agent[0].topic, "rfr");

    let by_task = store
        .list_bus_entries(&BusEntryFilter {
            task_id: Some("task-1".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(by_task.len(), 1);
    assert_eq!(by_task[0].agent, "grok");

    let limited = store
        .list_bus_entries(&BusEntryFilter {
            limit: Some(1),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].body, "second");

    let future = store
        .list_bus_entries(&BusEntryFilter {
            since: Some("2999-01-01T00:00:00Z".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(future.is_empty());
}
