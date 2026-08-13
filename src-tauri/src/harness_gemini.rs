//! C12 Gemini / Antigravity CLI adapter.
//!
//! Captures messages authored inside Google Antigravity CLI (`agy`) sessions
//! into the durable hub transcript. Reads on-disk session logs from
//! `~/.gemini/antigravity-cli/brain/<conversation-id>/.system_generated/logs/transcript.jsonl`.

use ca_hub::HubStore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

fn gemini_brain_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".gemini")
        .join("antigravity-cli")
        .join("brain")
}

/// Finds the target `transcript.jsonl` for a session or the most recently modified conversation log.
fn latest_gemini_transcript_path(
    brain_dir: &Path,
    conversation_id: Option<&str>,
) -> Option<PathBuf> {
    if let Some(conv_id) = conversation_id {
        let candidate = brain_dir
            .join(conv_id)
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let entries = fs::read_dir(brain_dir).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter_map(|dir| {
            let log_file = dir
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");
            if log_file.is_file() {
                let modified = fs::metadata(&log_file).ok()?.modified().ok()?;
                Some((modified, log_file))
            } else {
                None
            }
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

/// Extracts assistant text responses from Antigravity `transcript.jsonl` lines.
fn recent_gemini_assistant_texts(path: &Path, tail_lines: usize) -> Vec<String> {
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

        let is_model = value.get("source").and_then(|s| s.as_str()) == Some("MODEL")
            || value.get("type").and_then(|t| t.as_str()) == Some("PLANNER_RESPONSE");
        if !is_model {
            continue;
        }

        if let Some(content) = value.get("content").and_then(|c| c.as_str()) {
            let trimmed = content.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("```json") {
                texts.push(trimmed.to_string());
            }
        }
    }
    texts
}

#[derive(Debug, Clone, Serialize)]
pub struct GeminiCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<ca_hub::MessageRecord>,
}

/// Reads the newest Antigravity CLI transcript for `workspace`, extracts assistant replies,
/// and durably records each as a harness capture scoped to `hub_session_id`.
pub fn capture_gemini_session(
    store: &HubStore,
    workspace: &Path,
    gemini_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<GeminiCaptureOutcome, String> {
    capture_gemini_session_from(
        &gemini_brain_dir(),
        store,
        workspace,
        gemini_session_id,
        hub_session_id,
    )
}

pub fn capture_gemini_session_from(
    brain_dir: &Path,
    store: &HubStore,
    workspace: &Path,
    gemini_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<GeminiCaptureOutcome, String> {
    let Some(path) = latest_gemini_transcript_path(brain_dir, gemini_session_id) else {
        return Ok(GeminiCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let texts = recent_gemini_assistant_texts(&path, 500);
    let mut captured = Vec::new();
    for text in &texts {
        if let Some(record) = store
            .record_harness_capture(
                "gemini",
                "gemini",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }
    Ok(GeminiCaptureOutcome {
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

    fn write_transcript(dir: &Path, conv_id: &str, lines: &[&str]) -> PathBuf {
        let logs_dir = dir.join(conv_id).join(".system_generated").join("logs");
        fs::create_dir_all(&logs_dir).unwrap();
        let path = logs_dir.join("transcript.jsonl");
        let mut file = fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    #[test]
    fn extracts_model_assistant_texts_from_gemini_transcript() {
        let dir = tempdir().unwrap();
        let lines = [
            r#"{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","content":"hello"}"#,
            r#"{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","content":"Hello! How can I assist you?"}"#,
        ];
        write_transcript(dir.path(), "conv-1", &lines);
        let path = dir
            .path()
            .join("conv-1")
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");
        let texts = recent_gemini_assistant_texts(&path, 500);
        assert_eq!(texts, vec!["Hello! How can I assist you?".to_string()]);
    }

    #[test]
    fn picks_most_recent_gemini_session_transcript() {
        let brain_dir = tempdir().unwrap();
        write_transcript(
            brain_dir.path(),
            "conv-older",
            &[r#"{"source":"MODEL","content":"old"}"#],
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
        let newer = write_transcript(
            brain_dir.path(),
            "conv-newer",
            &[r#"{"source":"MODEL","content":"new"}"#],
        );
        let picked = latest_gemini_transcript_path(brain_dir.path(), None);
        assert_eq!(picked, Some(newer));
    }

    #[test]
    fn capture_dedups_repeated_polls_of_the_same_gemini_transcript() {
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let brain_dir = tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        write_transcript(
            brain_dir.path(),
            "gemini-conv",
            &[
                r#"{"source":"MODEL","type":"PLANNER_RESPONSE","content":"Antigravity task completed"}"#,
            ],
        );

        let first =
            capture_gemini_session_from(brain_dir.path(), &store, workspace, None, None).unwrap();
        assert!(first.transcript_found);
        assert_eq!(first.captured.len(), 1);

        let second =
            capture_gemini_session_from(brain_dir.path(), &store, workspace, None, None).unwrap();
        assert!(second.captured.is_empty(), "identical poll must dedup");
    }

    #[test]
    fn gemini_session_id_and_hub_session_id_serve_distinct_purposes() {
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let brain_dir = tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        let disk_id = "real-gemini-conv-uuid";
        let hub_session_id = "work-session-789";

        write_transcript(
            brain_dir.path(),
            disk_id,
            &[
                r#"{"source":"MODEL","type":"PLANNER_RESPONSE","content":"Antigravity response for hub session"}"#,
            ],
        );

        let outcome = capture_gemini_session_from(
            brain_dir.path(),
            &store,
            workspace,
            Some(disk_id),
            Some(hub_session_id),
        )
        .unwrap();

        assert!(outcome.transcript_found);
        assert_eq!(outcome.captured.len(), 1);
        let record = &outcome.captured[0];
        assert_eq!(
            record.subject.as_deref(),
            Some(format!("channel:session:{hub_session_id}:capture").as_str())
        );
    }
}
