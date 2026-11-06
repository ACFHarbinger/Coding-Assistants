//! C14.12 Cursor `agent` harness capture adapter (#275).
//!
//! Reads on-disk agent transcripts from
//! `~/.cursor/projects/<workspace-with-slashes-as-dashes>/agent-transcripts/<chat-id>/<chat-id>.jsonl`.
//! Only final assistant text is captured (not thinking or tool calls).

use hub::HubStore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

fn cursor_projects_dir() -> PathBuf {
    hub::cursor_projects_dir()
}

pub(crate) fn encode_workspace_dir_name(workspace: &Path) -> String {
    hub::encode_workspace_dir_name(workspace)
}

fn agent_transcripts_dir(projects_dir: &Path, workspace: &Path) -> PathBuf {
    projects_dir
        .join(encode_workspace_dir_name(workspace))
        .join("agent-transcripts")
}

fn latest_transcript_path(
    projects_dir: &Path,
    workspace: &Path,
    chat_id: Option<&str>,
) -> Option<PathBuf> {
    let root = agent_transcripts_dir(projects_dir, workspace);
    if let Some(chat_id) = chat_id {
        let candidate = root.join(chat_id).join(format!("{chat_id}.jsonl"));
        return candidate.is_file().then_some(candidate);
    }
    let entries = fs::read_dir(&root).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter_map(|dir| {
            if !dir.is_dir() {
                return None;
            }
            let chat_id = dir.file_name()?.to_string_lossy().into_owned();
            let transcript = dir.join(format!("{chat_id}.jsonl"));
            if !transcript.is_file() {
                return None;
            }
            let modified = fs::metadata(&transcript).ok()?.modified().ok()?;
            Some((modified, transcript))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn recent_assistant_texts(path: &Path, tail_lines: usize) -> Vec<String> {
    let Ok(raw) = fs::read_to_string(path) else {
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

        if value.get("type").and_then(|item| item.as_str()) == Some("assistant") {
            if let Some(text) = assistant_text_from_stream_line(&value) {
                texts.push(text);
                continue;
            }
        }

        if value.get("role").and_then(|item| item.as_str()) == Some("assistant") {
            if let Some(text) = assistant_text_from_transcript_line(&value) {
                texts.push(text);
            }
        }
    }
    texts
}

fn assistant_text_from_stream_line(value: &serde_json::Value) -> Option<String> {
    let content = value.get("message")?.get("content")?.as_array()?;
    collect_text_parts(content)
}

fn assistant_text_from_transcript_line(value: &serde_json::Value) -> Option<String> {
    let content = value.get("message")?.get("content")?.as_array()?;
    collect_text_parts(content)
}

fn collect_text_parts(content: &[serde_json::Value]) -> Option<String> {
    let mut parts = Vec::new();
    for item in content {
        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CursorCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<hub::MessageRecord>,
}

pub fn capture_cursor_session(
    store: &HubStore,
    workspace: &Path,
    cursor_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<CursorCaptureOutcome, String> {
    let Some(session_id) =
        super::resolve_capture_session_id(store, "cursor", workspace, cursor_session_id)?
    else {
        return Ok(CursorCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_cursor_session_from(
        &cursor_projects_dir(),
        store,
        &workspace,
        Some(&session_id),
        hub_session_id,
    )
}

pub fn capture_cursor_session_from(
    projects_dir: &Path,
    store: &HubStore,
    workspace: &Path,
    cursor_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<CursorCaptureOutcome, String> {
    let Some(path) = latest_transcript_path(projects_dir, workspace, cursor_session_id) else {
        return Ok(CursorCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace_key = workspace.to_string_lossy();
    if store
        .get_harness_session("cursor", &workspace_key)
        .map_err(|error| error.to_string())?
        .is_none()
    {
        let Some(disk_session_id) = path.parent().and_then(|dir| dir.file_name()) else {
            return Ok(CursorCaptureOutcome {
                transcript_found: false,
                scanned: 0,
                captured: Vec::new(),
            });
        };
        store
            .register_harness_session(
                "cursor",
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
                "cursor",
                "cursor",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }
    Ok(CursorCaptureOutcome {
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

    #[test]
    fn encodes_workspace_like_the_cursor_cli() {
        assert_eq!(
            encode_workspace_dir_name(Path::new(
                "/home/pkhunter/Repositories/Repo/Coding-Assistants"
            )),
            "home-pkhunter-Repositories-Repo-Coding-Assistants"
        );
    }

    #[test]
    fn captures_assistant_text_and_skips_thinking() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c14-cursor-capture");
        let session_dir = root
            .path()
            .join(encode_workspace_dir_name(&workspace))
            .join("agent-transcripts")
            .join("chat-1");
        std::fs::create_dir_all(&session_dir).unwrap();
        let mut file = std::fs::File::create(session_dir.join("chat-1.jsonl")).unwrap();
        writeln!(
            file,
            r#"{{"role":"user","message":{{"content":[{{"type":"text","text":"hi"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"thinking","subtype":"delta","text":"thinking"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"role":"assistant","message":{{"content":[{{"type":"text","text":"working on C14.12"}}]}}}}"#
        )
        .unwrap();

        store
            .register_harness_session("cursor", "/tmp/c14-cursor-capture", "chat-1", None)
            .unwrap();

        let first = capture_cursor_session_from(
            root.path(),
            &store,
            &workspace,
            Some("chat-1"),
            Some("hub-1"),
        )
        .unwrap();
        assert!(first.transcript_found);
        assert_eq!(first.scanned, 1);
        assert_eq!(first.captured.len(), 1);
        assert_eq!(first.captured[0].from_agent, "cursor");
        assert_eq!(first.captured[0].body, "working on C14.12");

        let second = capture_cursor_session_from(
            root.path(),
            &store,
            &workspace,
            Some("chat-1"),
            Some("hub-1"),
        )
        .unwrap();
        assert!(second.captured.is_empty());
    }

    #[test]
    fn missing_transcript_is_a_noop() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        store
            .register_harness_session("cursor", "/tmp/does-not-exist-c14", "chat-1", None)
            .unwrap();
        let outcome = capture_cursor_session_from(
            root.path(),
            &store,
            Path::new("/tmp/does-not-exist-c14"),
            Some("chat-1"),
            None,
        )
        .unwrap();
        assert!(!outcome.transcript_found);
        assert!(outcome.captured.is_empty());
    }

    #[test]
    fn missing_explicit_transcript_never_falls_back_to_the_newest_chat() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c14-cursor-identity-gate");
        let unrelated = root
            .path()
            .join(encode_workspace_dir_name(&workspace))
            .join("agent-transcripts")
            .join("unrelated-chat");
        std::fs::create_dir_all(&unrelated).unwrap();
        std::fs::write(
            unrelated.join("unrelated-chat.jsonl"),
            r#"{"role":"assistant","message":{"content":[{"type":"text","text":"private unrelated reply"}]}}"#,
        )
        .unwrap();

        let outcome = capture_cursor_session_from(
            root.path(),
            &store,
            &workspace,
            Some("managed-id-with-no-transcript"),
            Some("hub-1"),
        )
        .unwrap();

        assert!(!outcome.transcript_found);
        assert!(outcome.captured.is_empty());
        assert!(store
            .list_channel_messages("session:hub-1", 10)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn capture_preserves_an_existing_managed_registration() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c14-cursor-managed-capture");
        let chat_id = "chat-owned-1";
        let session_dir = root
            .path()
            .join(encode_workspace_dir_name(&workspace))
            .join("agent-transcripts")
            .join(chat_id);
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(
            session_dir.join(format!("{chat_id}.jsonl")),
            r#"{"role":"assistant","message":{"content":[{"type":"text","text":"done"}]}}"#,
        )
        .unwrap();
        store
            .register_managed_harness_session("cursor", &workspace.to_string_lossy(), chat_id, 1234)
            .unwrap();

        capture_cursor_session_from(root.path(), &store, &workspace, Some(chat_id), None).unwrap();

        let registration = store
            .get_harness_session("cursor", &workspace.to_string_lossy())
            .unwrap()
            .unwrap();
        assert_eq!(registration.mode, hub::HarnessSessionMode::Managed);
        assert_eq!(registration.disk_session_id, chat_id);
    }
}
