use super::*;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn pairing_redeems_once_and_rejects_a_wrong_or_expired_code() {
    let dir = tempdir().unwrap();
    let mut store = BindingStore::open(dir.path()).unwrap();
    let code = store.issue_pairing(100).unwrap();
    store.redeem(7, 100, Some("pk".into()), &code).unwrap();
    assert!(store.is_bound(7));
    assert_eq!(store.bound_users()[0].channel_id, 100);
    assert!(store.issue_pairing(100).is_ok());
    assert!(store.redeem(8, 100, None, "NOPE").is_err());
    store.data.pairing = Some(Pairing {
        code: "DEAD00".into(),
        expires_at: (Utc::now() - chrono::Duration::minutes(1)).to_rfc3339(),
        channel_id: 100,
    });
    assert!(store.redeem(8, 100, None, "DEAD00").is_err());
    assert!(!store.is_bound(8));
}

#[test]
fn cursors_are_per_channel_monotonic_and_durable() {
    let dir = tempdir().unwrap();
    let mut store = BindingStore::open(dir.path()).unwrap();
    store.advance_cursor(100, 42).unwrap();
    store.advance_cursor(100, 12).unwrap();
    store.advance_cursor(200, 7).unwrap();
    assert_eq!(store.cursor(100), Some(42));
    assert_eq!(store.cursor(200), Some(7));

    let reopened = BindingStore::open(dir.path()).unwrap();
    assert_eq!(reopened.cursor(100), Some(42));
    assert_eq!(reopened.cursor(200), Some(7));
}

#[test]
fn poll_channels_unions_bound_and_pairing_channels() {
    let dir = tempdir().unwrap();
    let mut store = BindingStore::open(dir.path()).unwrap();
    assert!(store.poll_channels().is_empty());
    store.issue_pairing(300).unwrap();
    assert_eq!(store.poll_channels(), vec![300]);
    let code = store.issue_pairing(300).unwrap();
    store.redeem(7, 100, None, &code).unwrap();
    assert_eq!(store.poll_channels(), vec![100]);
}

#[test]
fn reload_picks_up_external_unbind_without_stale_restore() {
    let dir = tempdir().unwrap();
    let mut daemon_store = BindingStore::open(dir.path()).unwrap();
    let code = daemon_store.issue_pairing(100).unwrap();
    daemon_store.redeem(7, 100, None, &code).unwrap();
    assert!(daemon_store.is_bound(7));

    let mut cli_store = BindingStore::open(dir.path()).unwrap();
    assert_eq!(cli_store.unbind(Some(7)).unwrap(), 1);
    assert!(!cli_store.is_bound(7));

    daemon_store.reload().unwrap();
    assert!(!daemon_store.is_bound(7));

    daemon_store.advance_cursor(100, 50).unwrap();
    let reopened = BindingStore::open(dir.path()).unwrap();
    assert!(!reopened.is_bound(7));
    assert_eq!(reopened.cursor(100), Some(50));
}

#[test]
fn reload_corrupted_file_fails_closed_and_clears_stale_bound_users() {
    let dir = tempdir().unwrap();
    let mut store = BindingStore::open(dir.path()).unwrap();
    let code = store.issue_pairing(100).unwrap();
    store.redeem(7, 100, None, &code).unwrap();
    assert!(store.is_bound(7));

    let path = dir.path().join(BINDING_FILE);
    fs::write(&path, b"not valid json").unwrap();

    assert!(store.reload().is_err());
    assert!(!store.is_bound(7));
    assert!(store.bound_users().is_empty());
}

#[test]
fn save_writes_atomically_without_leaving_temporary_files() {
    let dir = tempdir().unwrap();
    let mut store = BindingStore::open(dir.path()).unwrap();
    store.advance_cursor(100, 1).unwrap();

    let path = dir.path().join(BINDING_FILE);
    assert!(path.exists());
    let entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
        .collect();
    assert!(entries.is_empty());
}

#[test]
fn cross_process_lock_prevents_lost_update_when_daemon_marks_wake_notified_after_cli_unbind() {
    let dir = tempdir().unwrap();
    let mut daemon_store = BindingStore::open(dir.path()).unwrap();
    let code = daemon_store.issue_pairing(100).unwrap();
    daemon_store.redeem(7, 100, None, &code).unwrap();
    assert!(daemon_store.is_bound(7));

    // CLI unbinds user 7 while daemon still holds user 7 in its in-memory struct
    let mut cli_store = BindingStore::open(dir.path()).unwrap();
    assert_eq!(cli_store.unbind(Some(7)).unwrap(), 1);
    assert!(!cli_store.is_bound(7));

    // Daemon finishes wake-notification without prior reload.
    // with_mutation must re-read disk state inside the cross-process lock
    // so user 7 is not restored when saving notified wake IDs.
    assert!(daemon_store.mark_wake_notified("wake-1").unwrap());
    assert!(!daemon_store.is_bound(7));

    let reopened = BindingStore::open(dir.path()).unwrap();
    assert!(
        !reopened.is_bound(7),
        "daemon must not restore revoked user"
    );
    assert!(reopened.was_wake_notified("wake-1"));
}

#[test]
fn cross_process_lock_blocks_concurrent_writers() {
    let dir = tempdir().unwrap();
    let lock_path = dir.path().join(".discord_binding.lock");
    let guard = BindingLockGuard::acquire_with_timeout(&lock_path, Duration::from_secs(1)).unwrap();

    let err = BindingLockGuard::acquire_with_timeout(&lock_path, Duration::from_millis(50));
    assert!(err.is_err(), "second acquire must fail while lock is held");

    drop(guard);

    let guard2 =
        BindingLockGuard::acquire_with_timeout(&lock_path, Duration::from_millis(50)).unwrap();
    drop(guard2);
}

#[test]
fn stale_lock_with_dead_pid_is_recovered() {
    let dir = tempdir().unwrap();
    let lock_path = dir.path().join(".discord_binding.lock");

    let dead_record = LockRecord {
        pid: 9_999_999,
        nonce: "dead".into(),
        created_at_ms: Utc::now().timestamp_millis(),
    };
    fs::write(&lock_path, serde_json::to_vec(&dead_record).unwrap()).unwrap();

    let guard =
        BindingLockGuard::acquire_with_timeout(&lock_path, Duration::from_millis(100)).unwrap();
    drop(guard);
    assert!(!lock_path.exists());
}
