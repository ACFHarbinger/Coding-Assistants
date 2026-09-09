//! S7 Mistral Vibe capture adapter.
//!
//! Reads the on-disk Vibe event log the CLI writes:
//! `<vibe-sessions-root>/session_<YYYYMMDD>_<HHMMSS>_<short-id>/messages.jsonl`
//! (path and record shapes verified live against vibe 2.25.1).
//! Only assistant **text** is captured: `role == "assistant"` and
//! `injected != true` (skip app-injected task/wake prompts), take `content`
//! — not `reasoning_content` (same rule as Claude skipping `thinking`).

use hub::HubStore;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// The CLI-side session id for a Hub session id: Hub managed workers
/// register as `managed-<uuid>` while `vibe --resume` requires a bare UUID,
/// so the registered id must be normalized before locating the log
/// directory. An explicit id that is already a UUID passes through.
fn disk_session_id(session_id: &str) -> Option<String> {
    hub::vibe_disk_session_id(session_id)
}

/// Resolve a session directory name from the session id. Vibe session
/// directories are named `session_<YYYYMMDD>_<HHMMSS>_<short-id>` but the
/// `meta.json.session_id` UUID is the registered disk id. We look up the
/// directory by scanning for a matching `meta.json.session_id`.
fn latest_session_log(sessions_root: &Path, session_id: Option<&str>) -> Option<PathBuf> {
    let session_id = session_id?;
    // Fast path: the session_id might be the directory name itself.
    if let Some(path) = hub::vibe_session_log_path(sessions_root, session_id) {
        return Some(path);
    }
    // The registered id is a UUID from meta.json.session_id; we need to
    // find the session directory whose meta.json has that session_id.
    let Ok(entries) = std::fs::read_dir(sessions_root) else {
        return None;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let messages_path = path.join("messages.jsonl");
        if !messages_path.is_file() {
            continue;
        }
        if let Some(meta_id) = hub::vibe_session_id(&path) {
            if meta_id == session_id || disk_session_id(session_id).is_some_and(|d| d == meta_id) {
                return Some(messages_path);
            }
        }
    }
    None
}

/// Extracts assistant texts from a messages.jsonl tail. Only the last
/// `tail_lines` lines are parsed — logs grow unboundedly over long sessions
/// and older lines were already captured on a prior poll (the hub dedups on
/// content hash regardless).
///
/// Vibe-specific filter: `role == "assistant"` **and** `injected != true`
/// (skip app-injected task/wake prompts), take `content` — **not**
/// `reasoning_content` (same rule as Claude skipping `thinking`).
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
        // Only assistant messages.
        if value.get("role").and_then(|item| item.as_str()) != Some("assistant") {
            continue;
        }
        // Skip app-injected task/wake prompts.
        if value
            .get("injected")
            .and_then(|item| item.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        // Take `content`, not `reasoning_content`.
        if let Some(text) = value.get("content").and_then(|item| item.as_str()) {
            if !text.trim().is_empty() {
                texts.push(text.to_string());
            }
        }
    }
    texts
}

#[derive(Debug, Clone, Serialize)]
pub struct VibeCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<hub::MessageRecord>,
}

pub fn capture_vibe_session(
    store: &HubStore,
    workspace: &Path,
    vibe_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<VibeCaptureOutcome, String> {
    // #165 capture-identity gate: only capture a session the app has
    // registered (observed/managed) or explicitly named.
    let Some(session_id) =
        super::resolve_capture_session_id(store, "vibe", workspace, vibe_session_id)?
    else {
        return Ok(VibeCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_vibe_session_from(
        &hub::vibe_logs_root(),
        store,
        &workspace,
        Some(&session_id),
        hub_session_id,
    )
}

pub fn capture_vibe_session_from(
    sessions_root: &Path,
    store: &HubStore,
    workspace: &Path,
    vibe_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<VibeCaptureOutcome, String> {
    let Some(path) = latest_session_log(sessions_root, vibe_session_id) else {
        return Ok(VibeCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace_key = workspace.to_string_lossy();
    if store
        .get_harness_session("vibe", &workspace_key)
        .map_err(|error| error.to_string())?
        .is_none()
    {
        // Observed-session self-registration: fill a missing registration
        // only and never overwrite a managed one. The disk_session_id we
        // register is the meta.json.session_id UUID, not the directory name.
        let session_dir = path.parent().ok_or("session directory not found")?;
        let disk_id = hub::vibe_session_id(session_dir).unwrap_or_else(|| {
            session_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        store
            .register_harness_session("vibe", &workspace_key, &disk_id, None)
            .map_err(|error| error.to_string())?;
    }
    let texts = recent_assistant_texts(&path, 200);
    let mut captured = Vec::new();
    for text in &texts {
        // Recorded as ("vibe", "mistral") — the harness is "vibe" and the
        // agent identity is "mistral" per the S1 roster seed.
        if let Some(record) = store
            .record_harness_capture(
                "vibe",
                "mistral",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }
    Ok(VibeCaptureOutcome {
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
    const SESSION_DIR: &str = "session_20260909_120000_abc";

    fn write_messages(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        let mut file = std::fs::File::create(dir.join("messages.jsonl")).unwrap();
        // User message — skipped.
        writeln!(file, r#"{{"role":"user","content":"do the thing"}}"#).unwrap();
        // Assistant message — captured.
        writeln!(
            file,
            r#"{{"role":"assistant","content":"working on S7 capture"}}"#
        )
        .unwrap();
        // Assistant with reasoning_content — only content is captured, but
        // this one has no content field so it is skipped.
        writeln!(
            file,
            r#"{{"role":"assistant","reasoning_content":"thinking..."}}"#
        )
        .unwrap();
        // Injected assistant message — skipped (app-injected task/wake).
        writeln!(
            file,
            r#"{{"role":"assistant","injected":true,"content":"[TASK] do the thing"}}"#
        )
        .unwrap();
        // Tool message — skipped (role != assistant).
        writeln!(file, r#"{{"role":"tool","content":"tool output"}}"#).unwrap();
    }

    fn write_meta(dir: &Path, workspace: &str) {
        let meta = serde_json::json!({
            "session_id": SESSION_UUID,
            "environment": {"working_directory": workspace},
            "stats": {}
        });
        std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
    }

    #[test]
    fn captures_assistant_text_and_skips_injected_reasoning_and_non_assistant() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c14-vibe-capture");
        let session_dir = root.path().join(SESSION_DIR);
        write_messages(&session_dir);
        write_meta(&session_dir, "/tmp/c14-vibe-capture");
        store
            .register_harness_session("vibe", "/tmp/c14-vibe-capture", SESSION_UUID, None)
            .unwrap();

        let first = capture_vibe_session_from(
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
        assert_eq!(first.captured[0].from_agent, "mistral");
        assert_eq!(first.captured[0].body, "working on S7 capture");

        // Dedup: second poll captures nothing.
        let second = capture_vibe_session_from(
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
        let workspace = PathBuf::from("/tmp/c14-vibe-capture");
        let session_dir = root.path().join(SESSION_DIR);
        write_messages(&session_dir);
        write_meta(&session_dir, "/tmp/c14-vibe-capture");
        let managed = format!("managed-{SESSION_UUID}");
        store
            .register_managed_harness_session("vibe", "/tmp/c14-vibe-capture", &managed, 1234)
            .unwrap();

        let outcome =
            capture_vibe_session_from(root.path(), &store, &workspace, Some(&managed), None)
                .unwrap();
        assert!(outcome.transcript_found);
        assert_eq!(outcome.scanned, 1);
        let registration = store
            .get_harness_session("vibe", "/tmp/c14-vibe-capture")
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
        let outcome = capture_vibe_session_from(
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
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let gated =
            capture_vibe_session(&store, Path::new("/tmp/c14-vibe-capture"), None, None).unwrap();
        assert!(!gated.transcript_found);
        assert!(gated.captured.is_empty());
    }

    #[test]
    fn capture_records_as_vibe_mistral_not_vibe_vibe() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c14-vibe-attribution");
        let session_dir = root.path().join(SESSION_DIR);
        write_messages(&session_dir);
        write_meta(&session_dir, "/tmp/c14-vibe-attribution");
        store
            .register_harness_session("vibe", "/tmp/c14-vibe-attribution", SESSION_UUID, None)
            .unwrap();

        let outcome = capture_vibe_session_from(
            root.path(),
            &store,
            &workspace,
            Some(SESSION_UUID),
            Some("hub-1"),
        )
        .unwrap();
        assert_eq!(outcome.captured.len(), 1);
        assert_eq!(outcome.captured[0].from_agent, "mistral");
    }

    #[test]
    fn real_vibe_session_capture_and_filter() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c14-vibe-e2e");
        let session_dir = root.path().join("session_20260909_200716_f00b253a");
        std::fs::create_dir_all(&session_dir).unwrap();

        let mut file = std::fs::File::create(session_dir.join("messages.jsonl")).unwrap();
        writeln!(
            file,
            r#"{{"role":"user","content":"say hello in one word","injected":false}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"assistant","content":"Hello.","injected":false}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"assistant","reasoning_content":"Thinking...","injected":false}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"assistant","content":"[TASK] injected prompt","injected":true}}"#
        )
        .unwrap();

        let meta = serde_json::json!({
            "session_id": "f00b253a-b8a9-8b90-5f6d-fd56f27746dc",
            "environment": {"working_directory": "/tmp/c14-vibe-e2e"},
            "stats": {}
        });
        std::fs::write(session_dir.join("meta.json"), meta.to_string()).unwrap();

        store
            .register_harness_session(
                "vibe",
                "/tmp/c14-vibe-e2e",
                "f00b253a-b8a9-8b90-5f6d-fd56f27746dc",
                None,
            )
            .unwrap();

        let outcome = capture_vibe_session_from(
            root.path(),
            &store,
            &workspace,
            Some("f00b253a-b8a9-8b90-5f6d-fd56f27746dc"),
            Some("hub-session-1"),
        )
        .unwrap();

        assert!(outcome.transcript_found);
        assert_eq!(outcome.captured.len(), 1);
        assert_eq!(outcome.captured[0].from_agent, "mistral");
        assert_eq!(outcome.captured[0].body, "Hello.");
    }
}

#[cfg(test)]
mod manual_smoke {
    use super::*;

    #[test]
    #[ignore = "reads real ~/.vibe/logs/session data on this machine; run manually with --ignored"]
    fn real_transcript_smoke_check() {
        let store_dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from(
            "/home/pkhunter/Repositories/Repo/Coding-Assistants/target/vibe-test-workspace",
        );
        let outcome = capture_vibe_session(
            &store,
            &workspace,
            Some("f00b253a-b8a9-8b90-5f6d-fd56f27746dc"),
            None,
        )
        .unwrap();
        eprintln!(
            "vibe smoke: transcript_found={} scanned={} captured={}",
            outcome.transcript_found,
            outcome.scanned,
            outcome.captured.len()
        );
        assert!(outcome.transcript_found);
        assert_eq!(outcome.captured.len(), 1);
        assert_eq!(outcome.captured[0].from_agent, "mistral");
        assert_eq!(outcome.captured[0].body, "Hello.");
    }
}
