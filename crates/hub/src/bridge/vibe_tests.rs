//! Vibe bridge tests (split out of bridge/vibe.rs for the 500-LoC
//! cap, S6). A child module, so it globs vibe's private items.

use super::*;
use crate::{HarnessInjectRequest, HubStore, MessageKind};
use std::env;
use std::path::PathBuf;

/// Helper to save and restore environment variables for test isolation.
struct EnvGuard {
    orig_vibe_home: Option<String>,
    orig_home: Option<String>,
}

impl EnvGuard {
    fn new() -> Self {
        Self {
            orig_vibe_home: env::var("VIBE_HOME").ok(),
            orig_home: env::var("HOME").ok(),
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.orig_vibe_home {
            Some(val) => env::set_var("VIBE_HOME", val),
            None => env::remove_var("VIBE_HOME"),
        }
        match &self.orig_home {
            Some(val) => env::set_var("HOME", val),
            None => env::remove_var("HOME"),
        }
    }
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

#[test]
fn vibe_logs_root_uses_vibe_home_env() {
    let _guard = EnvGuard::new();
    env::set_var("VIBE_HOME", "/custom/vibe");
    let root = vibe_logs_root();
    assert_eq!(root, PathBuf::from("/custom/vibe/logs/session"));
}

#[test]
fn vibe_logs_root_falls_back_to_home_vibe() {
    let _guard = EnvGuard::new();
    env::remove_var("VIBE_HOME");
    env::set_var("HOME", "/test/home");
    let root = vibe_logs_root();
    assert_eq!(root, PathBuf::from("/test/home/.vibe/logs/session"));
}

// ---------------------------------------------------------------------------
// Session log path
// ---------------------------------------------------------------------------

#[test]
fn vibe_session_log_path_finds_existing_session() {
    let temp = tempfile::tempdir().unwrap();
    let sessions_root = temp.path();
    let session_dir = sessions_root.join("session_20260909_120000_test");
    std::fs::create_dir(&session_dir).unwrap();
    let log_file = session_dir.join("messages.jsonl");
    std::fs::File::create(&log_file).unwrap();
    let path = vibe_session_log_path(sessions_root, "session_20260909_120000_test");
    assert_eq!(path, Some(log_file));
}

#[test]
fn vibe_session_log_path_returns_none_for_missing_session() {
    let temp = tempfile::tempdir().unwrap();
    let path = vibe_session_log_path(temp.path(), "nonexistent");
    assert_eq!(path, None);
}

#[test]
fn vibe_session_log_path_returns_none_for_empty_id() {
    let temp = tempfile::tempdir().unwrap();
    let path = vibe_session_log_path(temp.path(), "");
    assert_eq!(path, None);
}

// ---------------------------------------------------------------------------
// Workspace + session_id reading from meta.json
// ---------------------------------------------------------------------------

#[test]
fn vibe_session_workspace_reads_from_meta_json() {
    let temp = tempfile::tempdir().unwrap();
    let session_dir = temp.path().join("session_test");
    std::fs::create_dir(&session_dir).unwrap();
    let meta = serde_json::json!({
        "environment": {"working_directory": "/test/workspace"},
        "session_id": "123e4567-e89b-42d3-a456-426614174000"
    });
    std::fs::write(session_dir.join("meta.json"), meta.to_string()).unwrap();
    assert_eq!(
        vibe_session_workspace(&session_dir),
        Some("/test/workspace".to_string())
    );
}

#[test]
fn vibe_session_id_reads_from_meta_json() {
    let temp = tempfile::tempdir().unwrap();
    let session_dir = temp.path().join("session_test");
    std::fs::create_dir(&session_dir).unwrap();
    let meta = serde_json::json!({
        "environment": {"working_directory": "/test/workspace"},
        "session_id": "123e4567-e89b-42d3-a456-426614174000"
    });
    std::fs::write(session_dir.join("meta.json"), meta.to_string()).unwrap();
    assert_eq!(
        vibe_session_id(&session_dir),
        Some("123e4567-e89b-42d3-a456-426614174000".to_string())
    );
}

#[test]
fn vibe_session_workspace_returns_none_for_missing_meta() {
    let temp = tempfile::tempdir().unwrap();
    let session_dir = temp.path().join("session_test");
    std::fs::create_dir(&session_dir).unwrap();
    assert_eq!(vibe_session_workspace(&session_dir), None);
}

#[test]
fn vibe_session_id_returns_none_for_missing_meta() {
    let temp = tempfile::tempdir().unwrap();
    let session_dir = temp.path().join("session_test");
    std::fs::create_dir(&session_dir).unwrap();
    assert_eq!(vibe_session_id(&session_dir), None);
}

// ---------------------------------------------------------------------------
// latest_vibe_session_id_from returns meta.json.session_id (not dir name)
// ---------------------------------------------------------------------------

#[test]
fn latest_vibe_session_id_from_returns_meta_json_session_id_not_dir_name() {
    let temp = tempfile::tempdir().unwrap();
    let sessions_root = temp.path();

    let session_dir = sessions_root.join("session_20260909_120000_abc");
    std::fs::create_dir(&session_dir).unwrap();
    std::fs::File::create(session_dir.join("messages.jsonl")).unwrap();
    let meta = serde_json::json!({
        "environment": {"working_directory": "/workspace/match"},
        "session_id": "123e4567-e89b-42d3-a456-426614174000"
    });
    std::fs::write(session_dir.join("meta.json"), meta.to_string()).unwrap();

    let id = latest_vibe_session_id_from(sessions_root, Path::new("/workspace/match"), 10);
    assert_eq!(id, Some("123e4567-e89b-42d3-a456-426614174000".to_string()));
    // Must NOT be the directory name
    assert_ne!(id.as_deref(), Some("session_20260909_120000_abc"));
}

#[test]
fn latest_vibe_session_id_from_returns_none_for_no_matching_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let sessions_root = temp.path();
    let session_dir = sessions_root.join("session_20260909_120000_test");
    std::fs::create_dir(&session_dir).unwrap();
    std::fs::File::create(session_dir.join("messages.jsonl")).unwrap();
    let meta = serde_json::json!({
        "environment": {"working_directory": "/workspace/one"},
        "session_id": "123e4567-e89b-42d3-a456-426614174000"
    });
    std::fs::write(session_dir.join("meta.json"), meta.to_string()).unwrap();
    let id = latest_vibe_session_id_from(sessions_root, Path::new("/workspace/different"), 10);
    assert_eq!(id, None);
}

#[test]
fn latest_vibe_session_id_from_picks_newest_matching() {
    let temp = tempfile::tempdir().unwrap();
    let sessions_root = temp.path();

    let old_dir = sessions_root.join("session_20260909_100000_old");
    std::fs::create_dir(&old_dir).unwrap();
    std::fs::File::create(old_dir.join("messages.jsonl")).unwrap();
    let old_meta = serde_json::json!({
        "environment": {"working_directory": "/workspace/match"},
        "session_id": "11111111-1111-1111-1111-111111111111"
    });
    std::fs::write(old_dir.join("meta.json"), old_meta.to_string()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let new_dir = sessions_root.join("session_20260909_130000_new");
    std::fs::create_dir(&new_dir).unwrap();
    std::fs::File::create(new_dir.join("messages.jsonl")).unwrap();
    let new_meta = serde_json::json!({
        "environment": {"working_directory": "/workspace/match"},
        "session_id": "22222222-2222-2222-2222-222222222222"
    });
    std::fs::write(new_dir.join("meta.json"), new_meta.to_string()).unwrap();

    let id = latest_vibe_session_id_from(sessions_root, Path::new("/workspace/match"), 10);
    assert_eq!(id, Some("22222222-2222-2222-2222-222222222222".to_string()));
}

// ---------------------------------------------------------------------------
// deliver_vibe_task_with — writer-lease state machine
// ---------------------------------------------------------------------------

const MANAGED_WS: &str = "/tmp/c14-vibe-deliver";
const MANAGED_UUID: &str = "123e4567-e89b-42d3-a456-426614174000";

fn managed_uuid() -> String {
    format!("managed-{MANAGED_UUID}")
}

fn register_queued_managed(store: &HubStore) {
    store
        .register_managed_harness_session_with_state(
            "vibe",
            MANAGED_WS,
            &managed_uuid(),
            None,
            crate::HarnessSessionState::Queued,
        )
        .unwrap();
}

fn task_request() -> HarnessInjectRequest {
    HarnessInjectRequest {
        harness: "vibe".into(),
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
fn deliver_requires_a_managed_registration() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let outcome = deliver_vibe_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "unavailable");
    assert_eq!(outcome.pid, None);
}

#[test]
fn deliver_refuses_observed_sessions_without_spawning() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .register_harness_session("vibe", MANAGED_WS, MANAGED_UUID, None)
        .unwrap();
    let outcome = deliver_vibe_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "unavailable");
    assert_eq!(outcome.pid, None);
}

#[test]
fn deliver_runs_the_worker_and_arms_capture() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);
    let outcome = deliver_vibe_task_with(&store, &task_request(), |_, body, uuid, _, _| {
        assert_eq!(body, "do the thing");
        assert_eq!(uuid, MANAGED_UUID);
        Ok(Some(4321))
    })
    .unwrap();
    assert_eq!(outcome.status, "delivered");
    assert_eq!(outcome.pid, Some(4321));
    let row = store
        .get_harness_session("vibe", MANAGED_WS)
        .unwrap()
        .unwrap();
    assert_eq!(row.state, crate::HarnessSessionState::Ready);
    assert_eq!(row.disk_session_id, managed_uuid());
    assert!(store
        .acquire_harness_writer("vibe", MANAGED_WS, "next-turn")
        .is_ok());
}

#[test]
fn deliver_failure_returns_the_session_to_queued() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);
    let outcome = deliver_vibe_task_with(&store, &task_request(), |_, _, _, _, _| {
        Err("worker failed".to_string())
    })
    .unwrap();
    assert_eq!(outcome.status, "errored");
    assert_eq!(outcome.pid, None);
    let row = store
        .get_harness_session("vibe", MANAGED_WS)
        .unwrap()
        .unwrap();
    assert_eq!(row.state, crate::HarnessSessionState::Queued);
}

#[test]
fn deliver_acks_the_message_on_success() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);

    let message = store
        .send_message(
            "sender",
            "vibe",
            MessageKind::Message,
            "Do the thing",
            Some("task"),
            Some(MANAGED_WS),
            None,
        )
        .unwrap();

    let request = HarnessInjectRequest {
        message_id: Some(message.id.clone()),
        ..task_request()
    };
    let _outcome =
        deliver_vibe_task_with(&store, &request, |_, _, _, _, _| Ok(Some(9101))).unwrap();
    let updated = store.get_message(&message.id).unwrap().unwrap();
    assert_eq!(updated.status, "acked");
}

// ---------------------------------------------------------------------------
// End-to-end inject_harness_with_store routing for HarnessId::Vibe
// ---------------------------------------------------------------------------

#[test]
fn inject_harness_with_store_routes_vibe_task_to_deliver_vibe_task() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);

    let request = HarnessInjectRequest {
        harness: "vibe".into(),
        workspace: PathBuf::from(MANAGED_WS),
        session_id: Some("hub-session-1".into()),
        message_id: None,
        body: "do the thing via inject".into(),
        is_task: true,
        is_wake: false,
        ..Default::default()
    };

    // Without a store, task-only injection falls through to the generic
    // queued fallback (no store = no managed delivery).
    let result = crate::inject_harness(&request).unwrap();
    assert_eq!(result.status, "queued");

    // With a store and a managed registration, the Vibe bridge delivers.
    let result = crate::inject_harness_with_store(&store, &request).unwrap();
    // The real runner spawns `vibe` which won't exist in the test env,
    // so we expect "errored" (not "delivered") — but crucially NOT the
    // generic "task is recorded in the session inbox" queued fallback.
    assert_ne!(result.status, "queued");
    assert_ne!(
        result.detail,
        "task is recorded in the session inbox; it awaits the target's active harness adapter"
    );
}

#[test]
fn inject_harness_with_store_vibe_unmanaged_returns_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let request = HarnessInjectRequest {
        harness: "vibe".into(),
        workspace: PathBuf::from(MANAGED_WS),
        session_id: Some("hub-session-1".into()),
        message_id: None,
        body: "do the thing".into(),
        is_task: true,
        is_wake: false,
        ..Default::default()
    };

    let result = crate::inject_harness_with_store(&store, &request).unwrap();
    assert_eq!(result.status, "unavailable");
    assert_eq!(result.pid, None);
}
