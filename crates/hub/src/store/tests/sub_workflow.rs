//! P19 (#366): a step with `sub_task_id` runs that child task.
use super::super::*;
use rusqlite::params;
use tempfile::tempdir;

fn open() -> (HubStore, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: true,
        })
        .unwrap();
    (store, dir)
}

fn step(agent: &str, instruction: &str, sub_task_id: Option<&str>) -> WorkflowStep {
    WorkflowStep {
        agent: agent.into(),
        role: None,
        instruction: instruction.into(),
        max_retries: 0,
        parallel_group: None,
        peer: None,
        branch: None,
        sub_task_id: sub_task_id.map(str::to_string),
        handoff_metadata: None,
        context_filter: None,
    }
}

fn set_steps(store: &HubStore, id: &str, steps: &[WorkflowStep]) {
    let json = serde_json::to_string(steps).unwrap();
    store
        .conn
        .execute(
            "UPDATE tasks SET steps_json = ?1 WHERE id = ?2",
            params![json, id],
        )
        .unwrap();
}

fn count(store: &HubStore, sql: &str, arg: &str) -> i64 {
    store.conn.query_row(sql, [arg], |row| row.get(0)).unwrap()
}

fn handoffs(store: &HubStore, task_id: &str) -> i64 {
    count(
        store,
        "SELECT COUNT(*) FROM messages WHERE task_id = ?1 AND kind = 'handoff'",
        task_id,
    )
}

fn task_wakes(store: &HubStore, task_id: &str) -> i64 {
    let like = format!("%task {task_id} %");
    count(
        store,
        "SELECT COUNT(*) FROM wake_requests WHERE reason LIKE ?1",
        &like,
    )
}

fn pair() -> (HubStore, TaskRecord, TaskRecord, tempfile::TempDir) {
    let (store, dir) = open();
    let child = store
        .create_task_with_parallel(
            "child",
            None,
            &[step("child-agent", "do the child work", None)],
            4,
            false,
        )
        .unwrap();
    let parent = store
        .create_task_with_parallel(
            "parent",
            None,
            &[step("parent-agent", "wait for the child", Some(&child.id))],
            4,
            false,
        )
        .unwrap();
    (store, parent, child, dir)
}

#[test]
fn parent_step_starts_child_without_handoff_or_wake() {
    let (store, parent, child, _dir) = pair();
    let started = store.advance_task(&parent.id, None, None).unwrap();
    assert_eq!(started.status, "running");
    assert!(started.last_message_id.is_none());
    assert_eq!(handoffs(&store, &parent.id), 0);
    assert_eq!(task_wakes(&store, &parent.id), 0);
    assert!(store
        .list_wakes(Some("parent-agent"), false)
        .unwrap()
        .is_empty());

    let child = store.get_task(&child.id).unwrap().unwrap();
    assert_eq!(child.status, "running");
    assert_ne!(child.status, "pending");
    assert_eq!(handoffs(&store, &child.id), 1);
    assert_eq!(
        store.list_wakes(Some("child-agent"), false).unwrap().len(),
        1
    );
}

#[test]
fn parent_advance_fails_while_child_is_running() {
    let (store, parent, child, _dir) = pair();
    store.advance_task(&parent.id, None, None).unwrap();
    let err = store
        .advance_task(&parent.id, Some("human"), None)
        .unwrap_err();
    assert!(err
        .to_string()
        .contains(&format!("waiting on sub-task {}", child.id)));
    let parent = store.get_task(&parent.id).unwrap().unwrap();
    assert_eq!(parent.status, "running");
    assert_eq!(
        store.get_task(&child.id).unwrap().unwrap().status,
        "running"
    );
}

#[test]
fn child_done_lets_parent_finish() {
    let (store, parent, child, _dir) = pair();
    store.advance_task(&parent.id, None, None).unwrap();
    let child = store
        .advance_task(&child.id, Some("child-agent"), None)
        .unwrap();
    assert_eq!(child.status, "done");
    let parent = store.get_task(&parent.id).unwrap().unwrap();
    assert_eq!(parent.status, "done");
    assert_eq!(handoffs(&store, &parent.id), 0);
}

#[test]
fn child_failed_fails_parent() {
    let (store, parent, child, _dir) = pair();
    store.advance_task(&parent.id, None, None).unwrap();
    let err = store.retry_task(&child.id, None, None).unwrap_err();
    assert!(err.to_string().contains("max_retries"));
    assert_eq!(store.get_task(&child.id).unwrap().unwrap().status, "failed");
    assert_eq!(
        store.get_task(&parent.id).unwrap().unwrap().status,
        "failed"
    );
    assert_eq!(handoffs(&store, &parent.id), 0);
}

#[test]
fn cycle_fails_parent_without_handoff() {
    let (store, _dir) = open();
    let first = store
        .create_task_with_parallel("a", None, &[step("parent-agent", "wait", None)], 4, false)
        .unwrap();
    let second = store
        .create_task_with_parallel(
            "b",
            None,
            &[step("child-agent", "wait", Some(&first.id))],
            4,
            false,
        )
        .unwrap();
    set_steps(
        &store,
        &first.id,
        &[step("parent-agent", "wait", Some(&second.id))],
    );

    let err = store.advance_task(&first.id, None, None).unwrap_err();
    assert!(err.to_string().contains("sub-workflow cycle"));
    assert_eq!(store.get_task(&first.id).unwrap().unwrap().status, "failed");
    assert_eq!(handoffs(&store, &first.id), 0);
    assert_eq!(handoffs(&store, &second.id), 0);
    assert_eq!(task_wakes(&store, &first.id), 0);
    assert_eq!(
        store.get_task(&second.id).unwrap().unwrap().status,
        "pending"
    );

    let solo = store
        .create_task_with_parallel(
            "self",
            None,
            &[step("parent-agent", "wait", None)],
            4,
            false,
        )
        .unwrap();
    set_steps(
        &store,
        &solo.id,
        &[step("parent-agent", "wait", Some(&solo.id))],
    );
    let err = store.advance_task(&solo.id, None, None).unwrap_err();
    assert!(err.to_string().contains("sub-workflow cycle"));
    assert_eq!(store.get_task(&solo.id).unwrap().unwrap().status, "failed");
    assert_eq!(handoffs(&store, &solo.id), 0);
    assert_eq!(task_wakes(&store, &solo.id), 0);
}
