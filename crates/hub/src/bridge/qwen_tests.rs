//! Qwen bridge tests (split out of bridge/qwen.rs for the 500-LoC cap).

use super::*;
use crate::{HarnessInjectRequest, HubStore};
use std::fs;
use std::path::PathBuf;

const MANAGED_WS: &str = "/tmp/c14-qwen-deliver";
const MANAGED_UUID: &str = "123e4567-e89b-42d3-a456-426614174000";

fn managed_uuid() -> String {
    format!("managed-{MANAGED_UUID}")
}

fn register_queued_managed(store: &HubStore) {
    store
        .register_managed_harness_session_with_state(
            "qwen",
            MANAGED_WS,
            &managed_uuid(),
            None,
            crate::HarnessSessionState::Queued,
        )
        .unwrap();
}

fn task_request() -> HarnessInjectRequest {
    HarnessInjectRequest {
        harness: "qwen".into(),
        workspace: PathBuf::from(MANAGED_WS),
        session_id: Some("hub-session-1".into()),
        message_id: None,
        body: "do the thing".into(),
        is_task: true,
        is_wake: false,
        ..Default::default()
    }
}

fn no_spawn_runner(
    _: &Path,
    _: &str,
    _: &str,
    _: Option<&str>,
    _: Option<&str>,
) -> Result<Option<u32>, String> {
    panic!("deliver must not spawn without an armed managed session")
}

#[test]
fn encode_workspace_matches_the_live_cli_sanitiser() {
    assert_eq!(
        encode_workspace_dir_name(Path::new("/tmp/c14-qwen")),
        "-tmp-c14-qwen"
    );
}

#[test]
fn latest_session_id_picks_the_newest_jsonl_in_the_workspace_chats_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let chats = chats_dir(tmp.path(), Path::new(MANAGED_WS));
    fs::create_dir_all(&chats).unwrap();
    let older = chats.join("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa.jsonl");
    let newer = chats.join(format!("{MANAGED_UUID}.jsonl"));
    fs::write(&older, "{}\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(&newer, "{}\n").unwrap();
    assert_eq!(
        latest_qwen_session_id_from(tmp.path(), Path::new(MANAGED_WS)).as_deref(),
        Some(MANAGED_UUID)
    );
    assert_eq!(
        qwen_session_log_path(tmp.path(), Path::new(MANAGED_WS), &managed_uuid()),
        Some(newer)
    );
}

#[test]
fn deliver_requires_a_managed_registration() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let outcome = deliver_qwen_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "unavailable");
    assert_eq!(outcome.pid, None);
    assert!(!outcome.detail.contains("spawned"));
}

#[test]
fn deliver_refuses_observed_sessions_without_spawning() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .register_harness_session("qwen", MANAGED_WS, MANAGED_UUID, None)
        .unwrap();
    let outcome = deliver_qwen_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "unavailable");
    assert_eq!(outcome.pid, None);
}

#[test]
fn deliver_runs_the_worker_and_arms_capture() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);
    let outcome = deliver_qwen_task_with(&store, &task_request(), |_, body, uuid, _, _| {
        assert_eq!(body, "do the thing");
        assert_eq!(uuid, MANAGED_UUID);
        Ok(Some(4321))
    })
    .unwrap();
    assert_eq!(outcome.status, "delivered");
    assert_eq!(outcome.pid, Some(4321));
    let row = store
        .get_harness_session("qwen", MANAGED_WS)
        .unwrap()
        .unwrap();
    assert_eq!(row.state, crate::HarnessSessionState::Ready);
    assert_eq!(row.disk_session_id, managed_uuid());
    assert!(store
        .acquire_harness_writer("qwen", MANAGED_WS, "next-turn")
        .is_ok());
}

#[test]
fn deliver_failure_returns_the_session_to_queued() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);
    let outcome =
        deliver_qwen_task_with(&store, &task_request(), |_, _, _, _, _| Err("boom".into()))
            .unwrap();
    assert_eq!(outcome.status, "errored");
    assert_eq!(outcome.pid, None);
    let row = store
        .get_harness_session("qwen", MANAGED_WS)
        .unwrap()
        .unwrap();
    assert_eq!(row.state, crate::HarnessSessionState::Queued);
}

#[test]
fn deliver_while_busy_stays_queued_without_spawning() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);
    store
        .acquire_harness_writer("qwen", MANAGED_WS, "other-owner")
        .unwrap();
    let outcome = deliver_qwen_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "queued");
    assert_eq!(outcome.pid, None);
}
