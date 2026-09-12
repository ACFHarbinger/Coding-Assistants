//! Hermes Agent transcript capture (C14.16, #322).
//!
//! Runs the first-party export (`hermes sessions export --format jsonl
//! --session-id <id>`, verified live against `hermes 0.21.2`) into a temp
//! file and records assistant messages into Chat & Memory. No scraping:
//! the export schema carries `role`/`content` plus a separate `reasoning`
//! field that is never read, so chain-of-thought stays out by construction.
//!
//! Message filters: `role == "assistant"` only, non-blank string `content`,
//! tool-call-only rows (blank content) skipped. `record_harness_capture`
//! deduplicates repeat polls by content hash.

use hub::HubStore;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

const EXPORT_TIMEOUT: Duration = Duration::from_secs(60);
const TAIL_MESSAGES: usize = 200;

/// Assistant texts from an export payload (one full session object per
/// line), newest session lines win by file order. Pure for tests.
pub fn assistant_texts_from_export(stdout: &str, tail_messages: usize) -> Vec<String> {
    let mut texts = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(session) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let messages = session
            .get("messages")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default();
        let start = messages.len().saturating_sub(tail_messages);
        for message in &messages[start..] {
            if message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
                continue;
            }
            if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                if !content.trim().is_empty() {
                    texts.push(content.to_string());
                }
            }
        }
    }
    texts
}

/// Run the export for one session into a temp file and return its text.
/// Thread + timeout so a stuck CLI cannot hang the capture poll.
fn export_session_text(session_id: &str) -> Result<String, String> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let out_path = std::env::temp_dir().join(format!(
        "hermes-capture-{}-{millis}.jsonl",
        std::process::id()
    ));
    let out_arg = out_path.to_string_lossy().into_owned();
    let id = session_id.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let output = Command::new("hermes")
            .args([
                "sessions",
                "export",
                "--format",
                "jsonl",
                "--session-id",
                &id,
                &out_arg,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output();
        let _ = tx.send(output);
    });
    let output = rx
        .recv_timeout(EXPORT_TIMEOUT)
        .map_err(|_| "hermes sessions export timed out".to_string())?
        .map_err(|e| format!("could not run `hermes sessions export`: {e}"))?;
    if !output.status.success() {
        let _ = fs::remove_file(&out_path);
        return Err(format!(
            "`hermes sessions export` failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .trim()
                .chars()
                .take(200)
                .collect::<String>()
        ));
    }
    let text =
        fs::read_to_string(&out_path).map_err(|e| format!("cannot read hermes export: {e}"))?;
    let _ = fs::remove_file(&out_path);
    Ok(text)
}

#[derive(Debug, Clone, Serialize)]
pub struct HermesCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<hub::MessageRecord>,
}

pub fn capture_hermes_session(
    store: &HubStore,
    workspace: &Path,
    hermes_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<HermesCaptureOutcome, String> {
    let Some(session_id) =
        super::resolve_capture_session_id(store, "hermes", workspace, hermes_session_id)?
    else {
        return Ok(HermesCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_hermes_session_from(store, &workspace, &session_id, hub_session_id)
}

pub fn capture_hermes_session_from(
    store: &HubStore,
    workspace: &Path,
    session_id: &str,
    hub_session_id: Option<&str>,
) -> Result<HermesCaptureOutcome, String> {
    let workspace_key = workspace.to_string_lossy();
    if store
        .get_harness_session("hermes", &workspace_key)
        .map_err(|error| error.to_string())?
        .is_none()
    {
        // Observed-session self-registration.
        store
            .register_harness_session("hermes", &workspace_key, session_id, None)
            .map_err(|error| error.to_string())?;
    }

    let text = export_session_text(session_id)?;
    capture_hermes_export_text(store, workspace, session_id, hub_session_id, &text)
}

/// Parse export text and record assistant messages. Split out so tests drive
/// fixtures without spawning the CLI.
pub fn capture_hermes_export_text(
    store: &HubStore,
    workspace: &Path,
    _session_id: &str,
    hub_session_id: Option<&str>,
    text: &str,
) -> Result<HermesCaptureOutcome, String> {
    let texts = assistant_texts_from_export(text, TAIL_MESSAGES);
    let mut captured = Vec::new();
    for text in &texts {
        if let Some(record) = store
            .record_harness_capture(
                "hermes",
                "hermes",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }

    Ok(HermesCaptureOutcome {
        transcript_found: true,
        scanned: texts.len(),
        captured,
    })
}

// Note: Hermes sessions live in SQLite (`~/.hermes`), reached only through
// `sessions export` — there is no transcript path to resolve.

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const EXPORT: &str = r#"{"id":"20260912_232548_916eae","cwd":"/tmp/ws","messages":[{"id":"3","role":"user","content":"Reply with the single word: pong","reasoning":"None"},{"id":"4","role":"assistant","content":"pong","reasoning":"The user wants pong.","finish_reason":"stop"},{"id":"5","role":"assistant","content":"   ","reasoning":"None"},{"id":"6","role":"tool","content":"tool output","reasoning":"None"}]}
{"id":"20260911_000000_aaaaaa","cwd":"/other","messages":[{"id":"7","role":"assistant","content":"other session text","reasoning":"r"}]}"#;

    #[test]
    fn assistant_texts_skip_reasoning_tool_and_blank() {
        let texts = assistant_texts_from_export(EXPORT, 100);
        assert_eq!(texts, vec!["pong", "other session text"]);
    }

    #[test]
    fn assistant_texts_honor_the_tail_window() {
        let texts = assistant_texts_from_export(EXPORT, 1);
        // Tail applies per session line: last message of each session.
        assert_eq!(texts, vec!["other session text"]);
    }

    #[test]
    fn capture_records_and_deduplicates() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path().join("hub")).unwrap();

        let out1 = capture_hermes_export_text(
            &store,
            Path::new("/tmp/ws"),
            "20260912_232548_916eae",
            None,
            EXPORT,
        )
        .unwrap();
        assert!(out1.transcript_found);
        assert_eq!(out1.scanned, 2);
        assert_eq!(out1.captured.len(), 2);
        assert_eq!(out1.captured[0].body, "pong");

        let out2 = capture_hermes_export_text(
            &store,
            Path::new("/tmp/ws"),
            "20260912_232548_916eae",
            None,
            EXPORT,
        )
        .unwrap();
        assert_eq!(out2.captured.len(), 0);
    }
}
