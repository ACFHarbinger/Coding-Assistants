//! C12 Grok adapter — capture.
//!
//! Start/inject already live in `ca_hub::harness` (`grok --cwd <abs> <prompt>`).
//! Capture reads the same on-disk session the Grok TUI writes:
//! `~/.grok/sessions/<percent-encoded-abs-workspace>/<session-id>/chat_history.jsonl`.
//! Only `type: "assistant"` text is recorded (not reasoning or tool results).

use ca_hub::HubStore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

fn grok_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".grok")
}

/// Matches Grok's session-folder naming: percent-encode the absolute
/// workspace with no safe characters (`/` → `%2F`).
pub(crate) fn encode_workspace_dir_name(workspace: &Path) -> String {
    workspace
        .to_string_lossy()
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn workspace_sessions_dir(sessions_root: &Path, workspace: &Path) -> PathBuf {
    sessions_root.join(encode_workspace_dir_name(workspace))
}

fn latest_chat_history(
    sessions_root: &Path,
    workspace: &Path,
    grok_session_id: Option<&str>,
) -> Option<PathBuf> {
    let root = workspace_sessions_dir(sessions_root, workspace);
    if let Some(session_id) = grok_session_id {
        let candidate = root.join(session_id).join("chat_history.jsonl");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let entries = fs::read_dir(&root).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|dir| {
            let history = dir.join("chat_history.jsonl");
            let modified = fs::metadata(&history).ok()?.modified().ok()?;
            Some((modified, history))
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
        if value.get("type").and_then(|item| item.as_str()) != Some("assistant") {
            continue;
        }
        let Some(text) = value.get("content").and_then(|item| item.as_str()) else {
            continue;
        };
        if !text.trim().is_empty() {
            texts.push(text.to_string());
        }
    }
    texts
}

#[derive(Debug, Clone, Serialize)]
pub struct GrokCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<ca_hub::MessageRecord>,
}

pub fn capture_grok_session(
    store: &HubStore,
    workspace: &Path,
    grok_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<GrokCaptureOutcome, String> {
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_grok_session_from(
        &grok_home().join("sessions"),
        store,
        &workspace,
        grok_session_id,
        hub_session_id,
    )
}

pub(crate) fn capture_grok_session_from(
    sessions_root: &Path,
    store: &HubStore,
    workspace: &Path,
    grok_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<GrokCaptureOutcome, String> {
    let Some(path) = latest_chat_history(sessions_root, workspace, grok_session_id) else {
        return Ok(GrokCaptureOutcome {
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
                "grok",
                "grok",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }
    Ok(GrokCaptureOutcome {
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
    fn encodes_workspace_like_the_grok_tui() {
        assert_eq!(
            encode_workspace_dir_name(Path::new(
                "/home/pkhunter/Repositories/Repos/Coding-Assistants"
            )),
            "%2Fhome%2Fpkhunter%2FRepositories%2FRepos%2FCoding-Assistants"
        );
    }

    #[test]
    fn captures_assistant_text_and_skips_reasoning() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c12-grok-capture");
        let session_dir = root
            .path()
            .join(encode_workspace_dir_name(&workspace))
            .join("sess-1");
        std::fs::create_dir_all(&session_dir).unwrap();
        let mut file = std::fs::File::create(session_dir.join("chat_history.jsonl")).unwrap();
        writeln!(file, r#"{{"type":"user","content":"hi"}}"#).unwrap();
        writeln!(file, r#"{{"type":"reasoning","content":"thinking"}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","content":"working on C12"}}"#).unwrap();
        writeln!(file, r#"{{"type":"tool_result","content":"ok"}}"#).unwrap();

        let first = capture_grok_session_from(root.path(), &store, &workspace, None, Some("hub-1"))
            .unwrap();
        assert!(first.transcript_found);
        assert_eq!(first.scanned, 1);
        assert_eq!(first.captured.len(), 1);
        assert_eq!(first.captured[0].from_agent, "grok");
        assert_eq!(first.captured[0].body, "working on C12");

        let second =
            capture_grok_session_from(root.path(), &store, &workspace, None, Some("hub-1"))
                .unwrap();
        assert!(second.captured.is_empty());
    }

    #[test]
    fn missing_transcript_is_a_noop() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let outcome = capture_grok_session_from(
            root.path(),
            &store,
            Path::new("/tmp/does-not-exist-c12"),
            None,
            None,
        )
        .unwrap();
        assert!(!outcome.transcript_found);
        assert!(outcome.captured.is_empty());
    }
}
