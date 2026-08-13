//! C12 Codex adapter — capture assistant replies from Codex session JSONL.
//!
//! Codex stores durable rollout transcripts under `~/.codex/sessions/YYYY/MM/DD`.
//! The initial `session_meta` record identifies the workspace, so polling can
//! select the newest transcript for the active workspace without attaching to
//! a terminal or scraping a UI.  Only assistant `output_text` blocks enter the
//! hub transcript; user, developer, tool, and reasoning records stay local.

use ca_hub::HubStore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

fn codex_sessions_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".codex").join("sessions")
}

fn transcript_paths(sessions_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Ok(years) = fs::read_dir(sessions_root) else {
        return paths;
    };
    for year in years.filter_map(Result::ok).map(|entry| entry.path()) {
        let Ok(months) = fs::read_dir(year) else {
            continue;
        };
        for month in months.filter_map(Result::ok).map(|entry| entry.path()) {
            let Ok(days) = fs::read_dir(month) else {
                continue;
            };
            for day in days.filter_map(Result::ok).map(|entry| entry.path()) {
                let Ok(files) = fs::read_dir(day) else {
                    continue;
                };
                paths.extend(
                    files
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension()
                                .is_some_and(|extension| extension == "jsonl")
                        }),
                );
            }
        }
    }
    paths
}

fn transcript_metadata(path: &Path) -> Option<(String, String)> {
    let raw = fs::read_to_string(path).ok()?;
    raw.lines().take(16).find_map(|line| {
        let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
        if value.get("type").and_then(|item| item.as_str()) != Some("session_meta") {
            return None;
        }
        let payload = value.get("payload")?;
        Some((
            payload.get("cwd")?.as_str()?.to_string(),
            payload.get("session_id")?.as_str()?.to_string(),
        ))
    })
}

fn latest_codex_transcript(
    sessions_root: &Path,
    workspace: &Path,
    disk_session_id: Option<&str>,
) -> Option<PathBuf> {
    let workspace = workspace.to_string_lossy();
    transcript_paths(sessions_root)
        .into_iter()
        .filter_map(|path| {
            let (cwd, session_id) = transcript_metadata(&path)?;
            if cwd != workspace || disk_session_id.is_some_and(|id| id != session_id) {
                return None;
            }
            Some((fs::metadata(&path).ok()?.modified().ok()?, path))
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
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("type").and_then(|item| item.as_str()) != Some("response_item") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if payload.get("type").and_then(|item| item.as_str()) != Some("message")
            || payload.get("role").and_then(|item| item.as_str()) != Some("assistant")
        {
            continue;
        }
        let Some(content) = payload.get("content").and_then(|item| item.as_array()) else {
            continue;
        };
        let text = content
            .iter()
            .filter(|item| item.get("type").and_then(|kind| kind.as_str()) == Some("output_text"))
            .filter_map(|item| item.get("text").and_then(|text| text.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n");
        if !text.trim().is_empty() {
            texts.push(text);
        }
    }
    texts
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<ca_hub::MessageRecord>,
}

/// Captures Codex's assistant-authored session transcript into the selected
/// Chat & Memory work-session. `disk_session_id` names Codex's local session;
/// `hub_session_id` identifies the CA work-session and must remain separate.
pub fn capture_codex_session(
    store: &HubStore,
    workspace: &Path,
    disk_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<CodexCaptureOutcome, String> {
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_codex_session_from(
        &codex_sessions_dir(),
        store,
        &workspace,
        disk_session_id,
        hub_session_id,
    )
}

pub(crate) fn capture_codex_session_from(
    sessions_root: &Path,
    store: &HubStore,
    workspace: &Path,
    disk_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<CodexCaptureOutcome, String> {
    let Some(path) = latest_codex_transcript(sessions_root, workspace, disk_session_id) else {
        return Ok(CodexCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let texts = recent_assistant_texts(&path, 500);
    let mut captured = Vec::new();
    for text in &texts {
        if let Some(record) = store
            .record_harness_capture(
                "codex",
                "chat",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }
    Ok(CodexCaptureOutcome {
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

    fn write_transcript(
        root: &Path,
        name: &str,
        cwd: &Path,
        session_id: &str,
        lines: &[&str],
    ) -> PathBuf {
        let dir = root.join("2026").join("08").join("13");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"session_meta","payload":{{"cwd":"{}","session_id":"{}"}}}}"#,
            cwd.display(),
            session_id
        )
        .unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    #[test]
    fn captures_only_assistant_output_text_and_deduplicates_polls() {
        let root = tempdir().unwrap();
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = Path::new("/tmp/c12-codex-capture");
        write_transcript(
            root.path(),
            "rollout-a.jsonl",
            workspace,
            "disk-a",
            &[
                r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"hi"}]}}"#,
                r#"{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"reasoning","text":"private"},{"type":"output_text","text":"C12 is underway."}]}}"#,
                r#"{"type":"response_item","payload":{"type":"function_call","name":"Read"}}"#,
            ],
        );
        let first = capture_codex_session_from(root.path(), &store, workspace, None, Some("hub-a"))
            .unwrap();
        assert!(first.transcript_found);
        assert_eq!(first.scanned, 1);
        assert_eq!(first.captured.len(), 1);
        assert_eq!(first.captured[0].from_agent, "chat");
        assert_eq!(first.captured[0].body, "C12 is underway.");
        let second =
            capture_codex_session_from(root.path(), &store, workspace, None, Some("hub-a"))
                .unwrap();
        assert!(second.captured.is_empty());
    }

    #[test]
    fn filters_by_workspace_and_explicit_disk_session_id() {
        let root = tempdir().unwrap();
        let workspace = Path::new("/tmp/c12-codex-workspace");
        let other = Path::new("/tmp/c12-other-workspace");
        let expected = write_transcript(root.path(), "rollout-a.jsonl", workspace, "disk-a", &[]);
        write_transcript(root.path(), "rollout-b.jsonl", other, "disk-b", &[]);
        assert_eq!(
            latest_codex_transcript(root.path(), workspace, Some("disk-a")),
            Some(expected)
        );
        assert_eq!(
            latest_codex_transcript(root.path(), workspace, Some("disk-b")),
            None
        );
    }
}
