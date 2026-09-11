//! Moonshot Kimi Code CLI transcript capture adapter (C14.14, #309).
//!
//! Reads the on-disk Kimi wire event log:
//! `<kimi-sessions-root>/wd_<slug>_<hash>/session_<uuid>/agents/main/wire.jsonl`.
//!
//! Only assistant text output is captured:
//! - Loop events: `type == "context.append_loop_event"` where `event.type == "content.part"`
//!   and `part.type == "text"` (extract `part.text`).
//!   Thinking/reasoning content (`part.type == "think"`) is skipped.
//! - Messages: `type == "context.append_message"` where `message.role == "assistant"`
//!   and `message.origin.kind != "injected"`.
//!
//! Deduplication is handled via SHA-256 in `store.record_harness_capture`.

use hub::HubStore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Normalize session ID for disk matching.
fn disk_session_id(session_id: &str) -> Option<String> {
    hub::kimi_disk_session_id(session_id)
}

/// Locate `agents/main/wire.jsonl` under the Kimi sessions root for a given session ID.
pub fn kimi_session_log_path(sessions_root: &Path, session_id: &str) -> Option<PathBuf> {
    let session_id = disk_session_id(session_id)?;
    if !sessions_root.is_dir() {
        return None;
    }

    let Ok(wd_entries) = fs::read_dir(sessions_root) else {
        return None;
    };

    for wd_entry in wd_entries.filter_map(Result::ok) {
        let wd_path = wd_entry.path();
        if !wd_path.is_dir() {
            continue;
        }

        // Check if wd_path contains a subdirectory matching session_id
        let direct_candidate = wd_path.join(&session_id);
        let wire = direct_candidate
            .join("agents")
            .join("main")
            .join("wire.jsonl");
        if wire.is_file() {
            return Some(wire);
        }
        let fallback_wire = direct_candidate.join("wire.jsonl");
        if fallback_wire.is_file() {
            return Some(fallback_wire);
        }

        let prefix_candidate = wd_path.join(format!("session_{session_id}"));
        let wire = prefix_candidate
            .join("agents")
            .join("main")
            .join("wire.jsonl");
        if wire.is_file() {
            return Some(wire);
        }
        let fallback_wire = prefix_candidate.join("wire.jsonl");
        if fallback_wire.is_file() {
            return Some(fallback_wire);
        }

        // Also inspect subdirectories with state.json
        let Ok(sub_entries) = fs::read_dir(&wd_path) else {
            continue;
        };
        for sub_entry in sub_entries.filter_map(Result::ok) {
            let sub_path = sub_entry.path();
            if !sub_path.is_dir() {
                continue;
            }
            let state_file = sub_path.join("state.json");
            if let Ok(raw) = fs::read(&state_file) {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&raw) {
                    if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
                        if id == session_id
                            || id.strip_prefix("session_") == Some(session_id.as_str())
                            || session_id.strip_prefix("session_") == Some(id)
                        {
                            let wire = sub_path.join("agents").join("main").join("wire.jsonl");
                            if wire.is_file() {
                                return Some(wire);
                            }
                            let fallback = sub_path.join("wire.jsonl");
                            if fallback.is_file() {
                                return Some(fallback);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extracts assistant texts from a `wire.jsonl` tail.
///
/// Filters:
/// - `context.append_loop_event`: where `event.type == "content.part"` and `part.type == "text"`.
///   Skips `part.type == "think"`.
/// - `context.append_message`: where `message.role == "assistant"` and `message.origin.kind != "injected"`.
pub fn recent_assistant_texts(path: &Path, tail_lines: usize) -> Vec<String> {
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
        let event_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");

        if event_type == "context.append_loop_event" {
            if let Some(event) = value.get("event") {
                if event.get("type").and_then(|t| t.as_str()) == Some("content.part") {
                    if let Some(part) = event.get("part") {
                        let part_type = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if part_type == "text" {
                            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                if !text.trim().is_empty() {
                                    texts.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        } else if event_type == "context.append_message" {
            if let Some(msg) = value.get("message") {
                let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
                let kind = msg
                    .pointer("/origin/kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("");
                if role == "assistant" && kind != "injected" {
                    if let Some(content) = msg.get("content") {
                        if let Some(s) = content.as_str() {
                            if !s.trim().is_empty() {
                                texts.push(s.to_string());
                            }
                        } else if let Some(arr) = content.as_array() {
                            for item in arr {
                                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    if let Some(t) = item.get("text").and_then(|s| s.as_str()) {
                                        if !t.trim().is_empty() {
                                            texts.push(t.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    texts
}

#[derive(Debug, Clone, Serialize)]
pub struct KimiCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<hub::MessageRecord>,
}

pub fn capture_kimi_session(
    store: &HubStore,
    workspace: &Path,
    kimi_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<KimiCaptureOutcome, String> {
    let Some(session_id) =
        super::resolve_capture_session_id(store, "kimi", workspace, kimi_session_id)?
    else {
        return Ok(KimiCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_kimi_session_from(
        &hub::kimi_sessions_root(),
        store,
        &workspace,
        Some(&session_id),
        hub_session_id,
    )
}

pub fn capture_kimi_session_from(
    sessions_root: &Path,
    store: &HubStore,
    workspace: &Path,
    kimi_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<KimiCaptureOutcome, String> {
    let session_id = kimi_session_id.unwrap_or("");
    let Some(path) = kimi_session_log_path(sessions_root, session_id) else {
        return Ok(KimiCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };

    let workspace_key = workspace.to_string_lossy();
    if store
        .get_harness_session("kimi", &workspace_key)
        .map_err(|error| error.to_string())?
        .is_none()
    {
        // Observed-session self-registration
        store
            .register_harness_session("kimi", &workspace_key, session_id, None)
            .map_err(|error| error.to_string())?;
    }

    let texts = recent_assistant_texts(&path, 200);
    let mut captured = Vec::new();
    for text in &texts {
        if let Some(record) = store
            .record_harness_capture(
                "kimi",
                "kimi",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }

    Ok(KimiCaptureOutcome {
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
    fn recent_assistant_texts_extracts_text_and_skips_think_and_injected() {
        let dir = tempdir().unwrap();
        let log = dir.path().join("wire.jsonl");

        let content = r#"{"type":"permission.set_mode","agentId":"main","mode":"auto","time":100}
{"type":"turn.prompt","agentId":"main","input":[{"type":"text","text":"user prompt"}],"origin":{"kind":"user"},"time":101}
{"type":"context.append_message","agentId":"main","message":{"role":"user","content":[{"type":"text","text":"user prompt"}]},"time":102}
{"type":"context.append_message","agentId":"main","message":{"role":"assistant","content":"system injection prompt","origin":{"kind":"injected"}},"time":103}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"think","think":"Internal reasoning to ignore"}},"time":104}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"First assistant response"}},"time":105}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"step.end","turnId":"0","step":1},"time":106}
{"type":"turn.ended","agentId":"main","turnId":0,"time":107}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"Second assistant turn"}},"time":108}
"#;
        let mut f = fs::File::create(&log).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        drop(f);

        let texts = recent_assistant_texts(&log, 100);
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0], "First assistant response");
        assert_eq!(texts[1], "Second assistant turn");
    }

    #[test]
    fn capture_kimi_session_from_deduplicates_and_records_to_store() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path().join("hub")).unwrap();
        let sessions_root = dir.path().join("sessions");
        let ws_dir = sessions_root.join("wd_test_123");
        let session_dir = ws_dir.join("session_abc-123");
        let wire_dir = session_dir.join("agents").join("main");
        fs::create_dir_all(&wire_dir).unwrap();

        let state = serde_json::json!({
            "id": "session_abc-123",
            "cwd": "/test/workspace",
            "updatedAt": 1000_u64,
            "archived": false
        });
        fs::write(
            session_dir.join("state.json"),
            serde_json::to_vec(&state).unwrap(),
        )
        .unwrap();

        let wire_log = wire_dir.join("wire.jsonl");
        let log_content = r#"{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"Captured text from Kimi"}},"time":200}
"#;
        fs::write(&wire_log, log_content).unwrap();

        // 1. First capture: records new message
        let out1 = capture_kimi_session_from(
            &sessions_root,
            &store,
            Path::new("/test/workspace"),
            Some("session_abc-123"),
            None,
        )
        .unwrap();
        assert!(out1.transcript_found);
        assert_eq!(out1.scanned, 1);
        assert_eq!(out1.captured.len(), 1);
        assert_eq!(out1.captured[0].body, "Captured text from Kimi");

        // 2. Second capture: deduplicated via SHA-256
        let out2 = capture_kimi_session_from(
            &sessions_root,
            &store,
            Path::new("/test/workspace"),
            Some("session_abc-123"),
            None,
        )
        .unwrap();
        assert!(out2.transcript_found);
        assert_eq!(out2.scanned, 1);
        assert_eq!(out2.captured.len(), 0);
    }
}
