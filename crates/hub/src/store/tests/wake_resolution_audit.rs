//! `set_wake_status_via` writer regressions (Team Scorecard T4 / #344):
//! one idempotent, atomically audited human decision per wake, human-gated
//! only, refused while a sync lock is held.

use super::super::*;
use tempfile::tempdir;

#[test]
fn set_wake_status_via_rejects_unknown_channels() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let wake = store
        .request_wake("grok", Some("slice"), None, true)
        .unwrap();

    let err = store
        .set_wake_status_via(&wake.id, WakeStatus::Delivered, "carrier-pigeon")
        .unwrap_err();
    assert!(matches!(err, HubError::Invalid(_)));

    // The wake is untouched and still a pending decision.
    let digest = store
        .get_owner_digest(chrono::Utc::now(), DIGEST_WINDOW_DAYS)
        .unwrap();
    assert_eq!(digest.summary.pending_count, 1);
    assert_eq!(digest.summary.resolved_window.total, 0);
}

/// Direct fixture: a pending wake row with an explicit gate flag and stamp,
/// bypassing `request_wake`'s policy path.
fn pending_wake_row(
    store: &HubStore,
    id: &str,
    target: &str,
    reason: Option<&str>,
    human_gate: bool,
    created_at: &str,
) {
    store
        .conn
        .execute(
            "INSERT INTO wake_requests(id, target_agent, message_id, reason, status, requires_human_gate, created_at) \
             VALUES (?1, ?2, NULL, ?3, 'pending', ?4, ?5)",
            params![id, target, reason, i64::from(human_gate), created_at],
        )
        .unwrap();
}

fn wake_resolved_rows(store: &HubStore) -> Vec<String> {
    store
        .list_audit_events(false)
        .unwrap()
        .iter()
        .filter(|event| event.operation == "wake.resolved")
        .map(|event| event.process_json.clone())
        .collect()
}

#[test]
fn wake_resolution_writes_one_row_across_repeated_and_competing_channels() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let wake = store
        .request_wake("grok", Some("slice"), None, true)
        .unwrap();

    // First decision arrives via Telegram.
    store
        .set_wake_status_via(&wake.id, WakeStatus::Delivered, "telegram")
        .unwrap();
    // Same decision retried: idempotent no-op.
    store
        .set_wake_status_via(&wake.id, WakeStatus::Delivered, "telegram")
        .unwrap();
    // A competing channel re-resolves the same wake: state changes,
    // but it is not a second human decision.
    store
        .set_wake_status_via(&wake.id, WakeStatus::Cancelled, "tcp")
        .unwrap();
    // Reset to pending and resolve again: still no second row.
    store
        .set_wake_status_via(&wake.id, WakeStatus::Pending, "desktop")
        .unwrap();
    store
        .set_wake_status_via(&wake.id, WakeStatus::Delivered, "tcp")
        .unwrap();

    let rows = wake_resolved_rows(&store);
    assert_eq!(rows.len(), 1, "one human decision per wake, ever");
    assert!(rows[0].contains("\"via\":\"telegram\""));

    let digest = store
        .get_owner_digest(chrono::Utc::now(), DIGEST_WINDOW_DAYS)
        .unwrap();
    assert_eq!(digest.summary.resolved_window.telegram, 1);
    assert_eq!(digest.summary.resolved_window.tcp, 0);
    assert_eq!(digest.summary.resolved_window.total, 1);
    assert_eq!(digest.summary.pending_count, 0);
}

#[test]
fn wake_resolution_is_atomic_when_the_audit_write_fails() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let wake = store
        .request_wake("grok", Some("slice"), None, true)
        .unwrap();

    // Force the audit insert to fail inside the transaction.
    store.conn.execute("DROP TABLE audit_events", []).unwrap();

    let result = store.set_wake_status_via(&wake.id, WakeStatus::Delivered, "desktop");
    assert!(result.is_err(), "the audit write must surface its failure");

    let status: String = store
        .conn
        .query_row(
            "SELECT status FROM wake_requests WHERE id = ?1",
            params![wake.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        status, "pending",
        "the wake row must roll back with the audit row"
    );
}

#[test]
fn wake_resolution_refuses_to_mutate_while_a_sync_lock_is_held() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let wake = store
        .request_wake("grok", Some("slice"), None, true)
        .unwrap();

    crate::sync::acquire_persisted(store.data_dir(), "up").unwrap();
    let err = store
        .set_wake_status_via(&wake.id, WakeStatus::Delivered, "telegram")
        .unwrap_err();
    assert!(err.to_string().contains(crate::sync::LOCKED_MESSAGE));
    crate::sync::release(store.data_dir()).unwrap();

    let status: String = store
        .conn
        .query_row(
            "SELECT status FROM wake_requests WHERE id = ?1",
            params![wake.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        status, "pending",
        "a locked store must not touch the wake row"
    );

    // After release, the same resolution goes through and audits once.
    store
        .set_wake_status_via(&wake.id, WakeStatus::Delivered, "telegram")
        .unwrap();
    assert_eq!(wake_resolved_rows(&store).len(), 1);
}

#[test]
fn non_gated_wake_lifecycle_is_not_a_human_review() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    // One non-gated wake resolves; another stays pending. Neither is a
    // human decision, so neither may reach the digest.
    pending_wake_row(
        &store,
        "auto-1",
        "grok",
        Some("routine delivery"),
        false,
        "2026-10-19T09:00:00Z",
    );
    pending_wake_row(
        &store,
        "auto-2",
        "muse",
        Some("routine delivery"),
        false,
        "2026-10-19T09:30:00Z",
    );

    store
        .set_wake_status_via("auto-1", WakeStatus::Delivered, "desktop")
        .unwrap();

    assert!(
        wake_resolved_rows(&store).is_empty(),
        "non-gated delivery writes no wake.resolved row"
    );

    let digest = store
        .get_owner_digest(chrono::Utc::now(), DIGEST_WINDOW_DAYS)
        .unwrap();
    assert_eq!(
        digest.summary.pending_count, 0,
        "pending non-gated wakes are not decisions"
    );
    assert_eq!(digest.summary.resolved_window.total, 0);
}
