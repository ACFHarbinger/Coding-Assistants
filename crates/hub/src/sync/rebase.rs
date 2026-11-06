//! Fork-aware audit rebase (S6 / #96).
//!
//! Rebase only when the two heads observed **different** paths. Same-path
//! forks stay in review. The sync-resolution event names both prior heads.

use super::lock;
use super::SyncError;
use crate::{AuditEvent, HubStore};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditObservation {
    pub event_hash: String,
    pub previous_hash: Option<String>,
    pub path: String,
    pub content_hash: Option<String>,
    pub operation: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForkDecision {
    FastForward,
    Rebase {
        local_head: String,
        remote_head: String,
    },
    Review {
        reason: &'static str,
    },
}

pub fn decide_fork(
    local_after: &[AuditObservation],
    remote_after: &[AuditObservation],
) -> ForkDecision {
    if remote_after.is_empty() || local_after.is_empty() {
        return ForkDecision::FastForward;
    }
    let local_paths: BTreeSet<&str> = local_after.iter().map(|row| row.path.as_str()).collect();
    let remote_paths: BTreeSet<&str> = remote_after.iter().map(|row| row.path.as_str()).collect();
    for path in local_paths.intersection(&remote_paths) {
        let local_hash = local_after
            .iter()
            .find(|row| row.path == *path)
            .and_then(|row| row.content_hash.as_deref());
        let remote_hash = remote_after
            .iter()
            .find(|row| row.path == *path)
            .and_then(|row| row.content_hash.as_deref());
        if local_hash != remote_hash {
            return ForkDecision::Review {
                reason: "same-path-fork",
            };
        }
    }
    let local_head = local_after
        .last()
        .map(|row| row.event_hash.clone())
        .expect("non-empty");
    let remote_head = remote_after
        .last()
        .map(|row| row.event_hash.clone())
        .expect("non-empty");
    ForkDecision::Rebase {
        local_head,
        remote_head,
    }
}

pub fn after_shared<'a>(
    events: &'a [AuditObservation],
    shared_head: Option<&str>,
) -> &'a [AuditObservation] {
    match shared_head {
        None => events,
        Some(head) => {
            if let Some(index) = events.iter().position(|row| row.event_hash == head) {
                &events[index + 1..]
            } else {
                events
            }
        }
    }
}

/// Replay remote observations onto the locked local store, then write
/// `sync-resolution` naming both original heads.
pub fn rebase_onto_local(
    store: &HubStore,
    remote_after: &[AuditObservation],
    local_head: &str,
    remote_head: &str,
) -> Result<AuditEvent, SyncError> {
    if !lock::is_held(store.data_dir()) {
        return Err(SyncError::Invalid(
            "sync lock is required for rebase".into(),
        ));
    }
    for row in remote_after {
        store
            .record_audit_for_sync(
                Path::new("sync"),
                Path::new(&row.path),
                &row.operation,
                r#"{"kind":"sync-rebase-replay"}"#,
                row.content_hash.as_deref(),
            )
            .map_err(|error| SyncError::Invalid(error.to_string()))?;
    }
    let payload = format!(
        r#"{{"kind":"sync-resolution","local_head":"{local_head}","remote_head":"{remote_head}"}}"#
    );
    store
        .record_audit_for_sync(
            Path::new("sync"),
            Path::new("sync-resolution"),
            "sync-resolution",
            &payload,
            None,
        )
        .map_err(|error| SyncError::Invalid(error.to_string()))
}

pub fn try_rebase_homes(local: &Path, remote: &Path) -> Result<Option<AuditEvent>, SyncError> {
    if !lock::is_held(local) {
        return Ok(None);
    }
    if !looks_like_sqlite(&local.join("hub.db")) || !looks_like_sqlite(&remote.join("hub.db")) {
        return Ok(None);
    }
    let local_store =
        HubStore::open(local).map_err(|error| SyncError::Invalid(error.to_string()))?;
    let remote_store = HubStore::open_existing_read_only(remote)
        .map_err(|error| SyncError::Invalid(error.to_string()))?;
    let local_events = observations_from_store(&local_store)?;
    let remote_events = observations_from_store(&remote_store)?;
    let shared = shared_head(&local_events, &remote_events);
    let local_after = after_shared(&local_events, shared.as_deref());
    let remote_after = after_shared(&remote_events, shared.as_deref());
    match decide_fork(local_after, remote_after) {
        ForkDecision::Rebase {
            local_head,
            remote_head,
        } => Ok(Some(rebase_onto_local(
            &local_store,
            remote_after,
            &local_head,
            &remote_head,
        )?)),
        _ => Ok(None),
    }
}

fn looks_like_sqlite(path: &Path) -> bool {
    fs::read(path)
        .ok()
        .is_some_and(|bytes| bytes.starts_with(b"SQLite format 3"))
}

fn shared_head(local: &[AuditObservation], remote: &[AuditObservation]) -> Option<String> {
    let mut last = None;
    for (left, right) in local.iter().zip(remote.iter()) {
        if left.event_hash == right.event_hash {
            last = Some(left.event_hash.clone());
        } else {
            break;
        }
    }
    last
}

pub fn observations_from_store(store: &HubStore) -> Result<Vec<AuditObservation>, SyncError> {
    let events = store
        .list_audit_events(false)
        .map_err(|error| SyncError::Invalid(error.to_string()))?;
    Ok(events
        .into_iter()
        .map(|event| AuditObservation {
            event_hash: event.event_hash,
            previous_hash: event.previous_hash,
            path: event.path,
            content_hash: event.content_hash,
            operation: event.operation,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::lock::SyncLockGuard;
    use tempfile::tempdir;

    fn obs(hash: &str, prev: Option<&str>, path: &str, content: &str) -> AuditObservation {
        AuditObservation {
            event_hash: hash.into(),
            previous_hash: prev.map(str::to_string),
            path: path.into(),
            content_hash: Some(content.into()),
            operation: "modified".into(),
        }
    }

    #[test]
    fn disjoint_paths_rebase_and_same_path_is_review() {
        let local = vec![obs("a1", Some("base"), "markdown/a.md", "ha")];
        let remote = vec![obs("b1", Some("base"), "markdown/b.md", "hb")];
        match decide_fork(&local, &remote) {
            ForkDecision::Rebase {
                local_head,
                remote_head,
            } => {
                assert_eq!(local_head, "a1");
                assert_eq!(remote_head, "b1");
            }
            other => panic!("expected rebase, got {other:?}"),
        }
        let clash = vec![obs("b2", Some("base"), "markdown/a.md", "other")];
        assert_eq!(
            decide_fork(&local, &clash),
            ForkDecision::Review {
                reason: "same-path-fork"
            }
        );
    }

    #[test]
    fn rebase_writes_resolution_with_both_heads_and_verifies() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let root = dir.path();
        let shared = store
            .record_audit_event(
                root,
                Path::new("markdown/base.md"),
                "modified",
                r#"{"pid":1}"#,
                Some("base-hash"),
            )
            .unwrap();
        let local = store
            .record_audit_event(
                root,
                Path::new("markdown/a.md"),
                "modified",
                r#"{"pid":2}"#,
                Some("hash-a"),
            )
            .unwrap();
        let remote = vec![obs(
            "remote-head",
            Some(&shared.event_hash),
            "markdown/b.md",
            "hash-b",
        )];
        let _guard = SyncLockGuard::acquire(store.data_dir(), "sync").unwrap();
        let resolution =
            rebase_onto_local(&store, &remote, &local.event_hash, "remote-head").unwrap();
        assert_eq!(resolution.operation, "sync-resolution");
        assert!(resolution.process_json.contains(&local.event_hash));
        assert!(resolution.process_json.contains("remote-head"));
        assert!(!resolution.process_json.contains("token"));
        assert_eq!(store.verify_audit_chain().unwrap(), 4);
        drop(_guard);
        let refused = decide_fork(
            &[obs(
                &local.event_hash,
                Some(&shared.event_hash),
                "markdown/a.md",
                "hash-a",
            )],
            &[obs(
                "other",
                Some(&shared.event_hash),
                "markdown/a.md",
                "hash-other",
            )],
        );
        assert_eq!(
            refused,
            ForkDecision::Review {
                reason: "same-path-fork"
            }
        );
        assert_eq!(store.verify_audit_chain().unwrap(), 4);
    }
}
