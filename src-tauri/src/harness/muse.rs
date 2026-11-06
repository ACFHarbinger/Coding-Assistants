//! C14.11 Meta Muse Code capture adapter (#273).
//!
//! Reads the on-disk Muse event log the CLI writes for a Hub-owned session:
//! `<sessions-root>/YYYY/MM/DD/<session-uuid>/session.jsonl` (path and
//! record shapes verified live against muse 1.0.3 — see the #273 spike).
//! Only committed assistant **text** (`assistant_message_committed`) is
//! captured: user prompts, tool calls, and lifecycle records are the
//! session's plumbing, not messages authored to the team.

use hub::HubStore;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// The CLI-side session uuid for a Hub session id: Hub managed workers
/// register as `managed-<uuid>` while `muse exec --session-id` requires a
/// bare UUID, so the registered id must be normalized before locating the
/// log directory. An explicit id that is already a UUID passes through.
fn disk_session_id(session_id: &str) -> Option<String> {
    hub::muse_disk_session_id(session_id)
}

fn latest_session_log(sessions_root: &Path, session_id: Option<&str>) -> Option<PathBuf> {
    let session_id = session_id?;
    if let Some(path) = hub::muse_session_log_path(sessions_root, session_id) {
        return Some(path);
    }
    // Hub registrations key on the Hub session id (`managed-<uuid>`); the
    // on-disk directory uses the stripped UUID (see `muse_spawn_args`).
    let disk_id = disk_session_id(session_id)?;
    if disk_id != session_id {
        return hub::muse_session_log_path(sessions_root, &disk_id);
    }
    None
}

/// Extracts committed assistant texts from an event-log tail. Only the last
/// `tail_lines` lines are parsed — logs grow unboundedly over long sessions
/// and older lines were already captured on a prior poll (the hub dedups on
/// content hash regardless).
fn recent_assistant_texts(path: &Path, tail_lines: usize) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = raw.lines().collect();
    let start = lines.len().saturating_sub(tail_lines);
    let mut texts = Vec::new();
    for line in &lines[start..] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("payload_type").and_then(|item| item.as_str()) != Some("runtime.session") {
            continue;
        }
        let Some(event) = value.pointer("/payload/event") else {
            continue;
        };
        if event.get("kind").and_then(|item| item.as_str()) != Some("assistant_message_committed") {
            continue;
        }
        if let Some(text) = event.get("text").and_then(|item| item.as_str()) {
            if !text.trim().is_empty() {
                texts.push(text.to_string());
            }
        }
    }
    texts
}

#[derive(Debug, Clone, Serialize)]
pub struct MuseCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<hub::MessageRecord>,
}

pub fn capture_muse_session(
    store: &HubStore,
    workspace: &Path,
    muse_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<MuseCaptureOutcome, String> {
    // #165 capture-identity gate: only capture a session the app has
    // registered (observed/managed) or explicitly named. Without it, the
    // desktop poll would grab the newest external transcript and attribute
    // a live outside conversation to the active work session.
    let Some(session_id) =
        super::resolve_capture_session_id(store, "muse", workspace, muse_session_id)?
    else {
        return Ok(MuseCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_muse_session_from(
        &hub::muse_sessions_root(),
        store,
        &workspace,
        Some(&session_id),
        hub_session_id,
    )
}

pub fn capture_muse_session_from(
    sessions_root: &Path,
    store: &HubStore,
    workspace: &Path,
    muse_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<MuseCaptureOutcome, String> {
    let Some(path) = latest_session_log(sessions_root, muse_session_id) else {
        return Ok(MuseCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace_key = workspace.to_string_lossy();
    if store
        .get_harness_session("muse", &workspace_key)
        .map_err(|error| error.to_string())?
        .is_none()
    {
        let Some(disk_session_id) = path.parent().and_then(|dir| dir.file_name()) else {
            return Ok(MuseCaptureOutcome {
                transcript_found: false,
                scanned: 0,
                captured: Vec::new(),
            });
        };
        // Muse has no leader-socket concept (that is Grok's); observed
        // sessions register without one. Preserve an existing managed
        // registration: capture must never downgrade ownership or discard
        // its writer-lease state.
        store
            .register_harness_session(
                "muse",
                &workspace_key,
                &disk_session_id.to_string_lossy(),
                None,
            )
            .map_err(|error| error.to_string())?;
    }
    let texts = recent_assistant_texts(&path, 200);
    let mut captured = Vec::new();
    for text in &texts {
        if let Some(record) = store
            .record_harness_capture(
                "muse",
                "muse",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }
    Ok(MuseCaptureOutcome {
        transcript_found: true,
        scanned: texts.len(),
        captured,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    const SESSION_UUID: &str = "123e4567-e89b-42d3-a456-426614174000";

    fn write_log(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        let mut file = std::fs::File::create(dir.join("session.jsonl")).unwrap();
        writeln!(
            file,
            r#"{{"payload_type":"runtime.session.metadata","payload":{{"kind":"metadata","record":{{"workspace_root":"/tmp/c14-muse-capture"}}}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"payload_type":"runtime.user_intent.accepted","payload":{{"refill_blocks":[{{"kind":"text","text":"do the thing"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"payload_type":"runtime.session","payload":{{"kind":"run","event":{{"kind":"assistant_message_committed","text":"working on C14.11"}}}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"payload_type":"runtime.session","payload":{{"kind":"run","event":{{"kind":"started","prompt":"do the thing"}}}}}}"#
        )
        .unwrap();
    }

    #[test]
    fn captures_committed_assistant_text_and_skips_plumbing() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c14-muse-capture");
        let session_dir = root
            .path()
            .join("2026")
            .join("09")
            .join("08")
            .join(SESSION_UUID);
        write_log(&session_dir);
        store
            .register_harness_session("muse", "/tmp/c14-muse-capture", SESSION_UUID, None)
            .unwrap();

        let first = capture_muse_session_from(
            root.path(),
            &store,
            &workspace,
            Some(SESSION_UUID),
            Some("hub-1"),
        )
        .unwrap();
        assert!(first.transcript_found);
        assert_eq!(first.scanned, 1);
        assert_eq!(first.captured.len(), 1);
        assert_eq!(first.captured[0].from_agent, "muse");
        assert_eq!(first.captured[0].body, "working on C14.11");

        let second = capture_muse_session_from(
            root.path(),
            &store,
            &workspace,
            Some(SESSION_UUID),
            Some("hub-1"),
        )
        .unwrap();
        assert!(second.captured.is_empty());
    }

    #[test]
    fn managed_hub_id_resolves_to_the_stripped_uuid_log() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c14-muse-capture");
        write_log(&root.path().join(SESSION_UUID));
        let managed = format!("managed-{SESSION_UUID}");
        store
            .register_managed_harness_session("muse", "/tmp/c14-muse-capture", &managed, 1234)
            .unwrap();

        let outcome =
            capture_muse_session_from(root.path(), &store, &workspace, Some(&managed), None)
                .unwrap();
        assert!(outcome.transcript_found);
        assert_eq!(outcome.scanned, 1);
        let registration = store
            .get_harness_session("muse", "/tmp/c14-muse-capture")
            .unwrap()
            .unwrap();
        assert_eq!(registration.mode, hub::HarnessSessionMode::Managed);
        assert_eq!(registration.disk_session_id, managed);
    }

    #[test]
    fn missing_transcript_is_a_noop() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let outcome = capture_muse_session_from(
            root.path(),
            &store,
            Path::new("/tmp/does-not-exist-c14"),
            Some(SESSION_UUID),
            None,
        )
        .unwrap();
        assert!(!outcome.transcript_found);
        assert!(outcome.captured.is_empty());
    }

    #[test]
    fn unregistered_workspace_captures_nothing() {
        // No registration and no explicit id: the #165 gate holds before
        // any transcript is even looked at.
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let gated =
            capture_muse_session(&store, Path::new("/tmp/c14-muse-capture"), None, None).unwrap();
        assert!(!gated.transcript_found);
        assert!(gated.captured.is_empty());
    }
}
