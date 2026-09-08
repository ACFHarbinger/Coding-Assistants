//! Muse bridge tests (split out of bridge/muse.rs for the 500-LoC
//! cap, #273). A child module, so it globs muse's private items.

use super::*;
use crate::{HarnessInjectRequest, HubStore};
use std::io::Write;

const MANAGED_WS: &str = "/tmp/c14-muse-deliver";
const MANAGED_UUID: &str = "123e4567-e89b-42d3-a456-426614174000";

fn managed_uuid() -> String {
    format!("managed-{MANAGED_UUID}")
}

fn register_queued_managed(store: &HubStore) {
    store
        .register_managed_harness_session_with_state(
            "muse",
            MANAGED_WS,
            &managed_uuid(),
            None,
            crate::HarnessSessionState::Queued,
        )
        .unwrap();
}

fn task_request() -> HarnessInjectRequest {
    HarnessInjectRequest {
        harness: "muse".into(),
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
    let outcome = deliver_muse_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "unavailable");
    assert_eq!(outcome.pid, None);
    assert!(!outcome.detail.contains("spawned"));
}

#[test]
fn deliver_refuses_observed_sessions_without_spawning() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .register_harness_session("muse", MANAGED_WS, MANAGED_UUID, None)
        .unwrap();
    let outcome = deliver_muse_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "unavailable");
    assert_eq!(outcome.pid, None);
}

#[test]
fn deliver_runs_the_worker_and_arms_capture() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);
    let outcome = deliver_muse_task_with(&store, &task_request(), |_, body, uuid, _, _| {
        // The Hub work-session id must never reach the CLI; the runner
        // gets the stripped disk UUID.
        assert_eq!(body, "do the thing");
        assert_eq!(uuid, MANAGED_UUID);
        Ok(Some(4321))
    })
    .unwrap();
    assert_eq!(outcome.status, "delivered");
    assert_eq!(outcome.pid, Some(4321));
    let row = store
        .get_harness_session("muse", MANAGED_WS)
        .unwrap()
        .unwrap();
    assert_eq!(row.state, crate::HarnessSessionState::Ready);
    assert_eq!(row.disk_session_id, managed_uuid());
    // The lease was released: a new owner can acquire it.
    assert!(store
        .acquire_harness_writer("muse", MANAGED_WS, "next-turn")
        .is_ok());
}

#[test]
fn deliver_failure_returns_the_session_to_queued() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);
    let outcome =
        deliver_muse_task_with(&store, &task_request(), |_, _, _, _, _| Err("boom".into()))
            .unwrap();
    assert_eq!(outcome.status, "errored");
    assert_eq!(outcome.pid, None);
    let row = store
        .get_harness_session("muse", MANAGED_WS)
        .unwrap()
        .unwrap();
    assert_eq!(row.state, crate::HarnessSessionState::Queued);
    assert!(store
        .acquire_harness_writer("muse", MANAGED_WS, "retry-turn")
        .is_ok());
}

#[test]
fn deliver_while_busy_stays_queued_without_spawning() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    register_queued_managed(&store);
    store
        .acquire_harness_writer("muse", MANAGED_WS, "other-turn")
        .unwrap();
    let outcome = deliver_muse_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "queued");
    assert_eq!(outcome.pid, None);
}

#[test]
fn deliver_rejects_an_unusable_registered_id_without_spawning() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .register_managed_harness_session_with_state(
            "muse",
            MANAGED_WS,
            "chat-1",
            None,
            crate::HarnessSessionState::Queued,
        )
        .unwrap();
    let outcome = deliver_muse_task_with(&store, &task_request(), no_spawn_runner).unwrap();
    assert_eq!(outcome.status, "unavailable");
    assert!(outcome.detail.contains("UUID"));
}

fn write_log(dir: &Path, workspace_root: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let mut file = std::fs::File::create(dir.join("session.jsonl")).unwrap();
    writeln!(
        file,
        r#"{{"payload_type":"runtime.session.metadata","payload":{{"kind":"metadata","record":{{"workspace_root":"{workspace_root}"}}}}}}"#
    )
    .unwrap();
    writeln!(
        file,
        r#"{{"payload_type":"runtime.session","payload":{{"kind":"run","event":{{"kind":"assistant_message_committed","text":"hi"}}}}}}"#
    )
    .unwrap();
}

#[test]
fn finds_a_sharded_session_log_by_id() {
    let root = tempfile::tempdir().unwrap();
    let dir = root
        .path()
        .join("2026")
        .join("09")
        .join("08")
        .join("sess-1");
    write_log(&dir, "/tmp/ws");
    let found = muse_session_log_path(root.path(), "sess-1").unwrap();
    assert_eq!(found, dir.join("session.jsonl"));
    assert!(muse_session_log_path(root.path(), "nope").is_none());
    assert!(muse_session_log_path(root.path(), "  ").is_none());
}

#[test]
fn attributes_a_log_to_its_workspace_and_picks_newest() {
    use std::time::{Duration, SystemTime};
    let root = tempfile::tempdir().unwrap();
    // Fixture creation is faster than mtime granularity, so pin explicit
    // mtimes — otherwise "newest" is filesystem luck.
    let stamp = |dir: &Path, age_secs: u64| {
        let time = SystemTime::now() - Duration::from_secs(age_secs);
        std::fs::File::options()
            .write(true)
            .open(dir.join("session.jsonl"))
            .unwrap()
            .set_modified(time)
            .unwrap();
    };
    // Older session for another workspace must not win.
    let old = root
        .path()
        .join("2026")
        .join("09")
        .join("07")
        .join("sess-old");
    write_log(&old, "/tmp/other-ws");
    stamp(&old, 200);
    // Newer session for the wanted workspace wins.
    let new = root
        .path()
        .join("2026")
        .join("09")
        .join("08")
        .join("sess-new");
    write_log(&new, "/tmp/ws");
    stamp(&new, 100);
    // Same-workspace older log loses to the newer one.
    let older = root
        .path()
        .join("2026")
        .join("09")
        .join("06")
        .join("sess-older");
    write_log(&older, "/tmp/ws");
    stamp(&older, 300);

    let latest = latest_muse_session_id_from(root.path(), Path::new("/tmp/ws"), 200).unwrap();
    assert_eq!(latest, "sess-new");
    assert!(latest_muse_session_id_from(root.path(), Path::new("/tmp/unknown"), 200).is_none());
}

#[test]
fn skips_hidden_and_tool_output_trees() {
    let root = tempfile::tempdir().unwrap();
    let hidden = root.path().join(".msp-view-v1").join("sess-hidden");
    write_log(&hidden, "/tmp/ws");
    assert!(muse_session_log_path(root.path(), "sess-hidden").is_none());
}
