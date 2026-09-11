//! C14.13 Qwen Code capture adapter (#308).
//!
//! Reads the on-disk Claude-Code-format JSONL the CLI writes:
//! `~/.qwen/projects/<sanitised-cwd>/chats/<sessionId>.jsonl`
//! (shape verified live against qwen 0.23.2).
//! Only final assistant **text** is captured: `type == "assistant"` text
//! parts, skip `type == "system"`. `provenance` (`real_user` vs `system`)
//! distinguishes injected user prompts, which this adapter already skips
//! by type. Recorded as `("qwen", "qwen")` — no identity split.

use hub::HubStore;
use serde::Serialize;
use std::path::{Path, PathBuf};

fn disk_session_id(session_id: &str) -> Option<String> {
    hub::qwen_disk_session_id(session_id)
}

fn latest_session_log(
    projects_dir: &Path,
    workspace: &Path,
    session_id: Option<&str>,
) -> Option<PathBuf> {
    if let Some(session_id) = session_id {
        if let Some(path) = hub::qwen_session_log_path(projects_dir, workspace, session_id) {
            return Some(path);
        }
        if let Some(stripped) = disk_session_id(session_id) {
            if let Some(path) = hub::qwen_session_log_path(projects_dir, workspace, &stripped) {
                return Some(path);
            }
        }
    }
    None
}

/// Extracts assistant texts from a JSONL tail. Only the last `tail_lines`
/// lines are parsed — transcripts grow unboundedly and older lines were
/// already captured on a prior poll (the hub dedups on content hash).
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
        if value.get("type").and_then(|item| item.as_str()) != Some("assistant") {
            continue;
        }
        let Some(parts) = value
            .get("message")
            .and_then(|message| message.get("parts"))
            .and_then(|parts| parts.as_array())
        else {
            continue;
        };
        let text: String = parts
            .iter()
            .filter_map(|part| part.get("text").and_then(|item| item.as_str()))
            .filter(|item| !item.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        if !text.trim().is_empty() {
            texts.push(text);
        }
    }
    texts
}

#[derive(Debug, Clone, Serialize)]
pub struct QwenCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<hub::MessageRecord>,
}

pub fn capture_qwen_session(
    store: &HubStore,
    workspace: &Path,
    qwen_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<QwenCaptureOutcome, String> {
    let Some(session_id) =
        super::resolve_capture_session_id(store, "qwen", workspace, qwen_session_id)?
    else {
        return Ok(QwenCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_qwen_session_from(
        &hub::qwen_projects_dir(),
        store,
        &workspace,
        Some(&session_id),
        hub_session_id,
    )
}

pub fn capture_qwen_session_from(
    projects_dir: &Path,
    store: &HubStore,
    workspace: &Path,
    qwen_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<QwenCaptureOutcome, String> {
    let Some(path) = latest_session_log(projects_dir, workspace, qwen_session_id) else {
        return Ok(QwenCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let texts = recent_assistant_texts(&path, 200);
    let mut captured = Vec::new();
    for text in &texts {
        if let Some(record) = store
            .record_harness_capture(
                "qwen",
                "qwen",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }
    Ok(QwenCaptureOutcome {
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

    const SESSION_UUID: &str = "7acdda1c-9a68-4d08-8ca0-6f2bce8327b7";
    const WS: &str = "/tmp/c14-qwen-capture";

    fn write_transcript(projects_dir: &Path, workspace: &str) -> PathBuf {
        let chats = projects_dir
            .join(hub::encode_qwen_workspace_dir_name(Path::new(workspace)))
            .join("chats");
        std::fs::create_dir_all(&chats).unwrap();
        let path = chats.join(format!("{SESSION_UUID}.jsonl"));
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"uuid":"a","sessionId":"{SESSION_UUID}","type":"system","provenance":"system","cwd":"{workspace}","message":{{"role":"system","parts":[{{"text":"boot"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"uuid":"b","sessionId":"{SESSION_UUID}","type":"user","provenance":"real_user","cwd":"{workspace}","message":{{"role":"user","parts":[{{"text":"say hi"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"uuid":"c","sessionId":"{SESSION_UUID}","type":"assistant","provenance":"system","cwd":"{workspace}","message":{{"role":"assistant","parts":[{{"text":"Hello from Qwen"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"uuid":"d","sessionId":"{SESSION_UUID}","type":"assistant","provenance":"system","cwd":"{workspace}","message":{{"role":"assistant","parts":[{{"text":""}}]}}}}"#
        )
        .unwrap();
        path
    }

    #[test]
    fn captures_assistant_text_and_skips_system_and_user() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from(WS);
        write_transcript(root.path(), WS);
        store
            .register_harness_session("qwen", WS, SESSION_UUID, None)
            .unwrap();

        let first = capture_qwen_session_from(
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
        assert_eq!(first.captured[0].from_agent, "qwen");
        assert_eq!(first.captured[0].body, "Hello from Qwen");

        let second = capture_qwen_session_from(
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
        let workspace = PathBuf::from(WS);
        write_transcript(root.path(), WS);
        let managed = format!("managed-{SESSION_UUID}");
        store
            .register_managed_harness_session("qwen", WS, &managed, 1234)
            .unwrap();

        let outcome =
            capture_qwen_session_from(root.path(), &store, &workspace, Some(&managed), None)
                .unwrap();
        assert!(outcome.transcript_found);
        assert_eq!(outcome.scanned, 1);
        let registration = store.get_harness_session("qwen", WS).unwrap().unwrap();
        assert_eq!(registration.mode, hub::HarnessSessionMode::Managed);
        assert_eq!(registration.disk_session_id, managed);
    }

    #[test]
    fn missing_transcript_is_a_noop() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let outcome = capture_qwen_session_from(
            root.path(),
            &store,
            Path::new("/tmp/does-not-exist-c14-qwen"),
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
            capture_qwen_session(&store, Path::new("/tmp/c14-qwen-capture"), None, None).unwrap();
        assert!(!gated.transcript_found);
        assert!(gated.captured.is_empty());
    }
}
