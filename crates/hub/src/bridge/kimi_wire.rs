//! Kimi Code CLI on-disk wire transcript capture (#309, hardened for #363).
//!
//! A Kimi session directory holds one wire event log per agent:
//! `<session>/agents/main/wire.jsonl` for the main loop and — under swarm
//! mode (`/swarm`, #363) — sibling `<session>/agents/<worker>/wire.jsonl`
//! transcripts per subagent. Subagent events may also appear multiplexed
//! into the main wire carrying their own `agentId` field.
//!
//! Only assistant text output is captured:
//! - Loop events: `type == "context.append_loop_event"` where `event.type
//!   == "content.part"` and `part.type == "text"` (`think` parts skipped).
//! - Messages: `type == "context.append_message"` where
//!   `message.role == "assistant"` and `message.origin.kind != "injected"`.
//!
//! Unknown event types and malformed lines are tolerated (skipped), so an
//! undocumented swarm-mode event shape cannot break capture. Attribution is
//! preserved: captures record under agent `kimi` (main loop) or
//! `kimi-<worker>` (subagents). Deduplication is handled via SHA-256 in
//! [crate::HubStore::record_harness_capture]. Ported from
//! `src-tauri/src/harness/kimi.rs` (#363) so the parsing rules are
//! unit-testable under `cargo test -p hub --lib`; the Tauri adapter
//! delegates here.

use crate::HubStore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// One attributed assistant text extracted from a wire log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KimiWireCapture {
    /// Wire-level agent name: the `agents/<name>` directory, or the event's
    /// own `agentId` when a multiplexed event carries one.
    pub agent: String,
    pub text: String,
}

/// Result of one capture poll over a session's wire logs.
#[derive(Debug, Clone, Serialize)]
pub struct KimiCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<crate::MessageRecord>,
}

/// Locate `agents/main/wire.jsonl` under the Kimi sessions root for a given
/// session ID (with the legacy `<session>/wire.jsonl` fallback).
pub fn kimi_session_log_path(sessions_root: &Path, session_id: &str) -> Option<PathBuf> {
    let session_id = crate::harness::kimi_disk_session_id(session_id)?;
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

/// Enumerate every agent wire log in the session that owns `main_wire`,
/// main first: `agents/<name>/wire.jsonl` siblings (swarm subagents,
/// #363) plus the legacy single-wire layout as `("main", main_wire)`.
pub fn kimi_session_wire_logs(main_wire: &Path) -> Vec<(String, PathBuf)> {
    let agents_dir = main_wire.parent().and_then(|main_dir| {
        if main_dir.file_name().is_some_and(|n| n == "main") {
            main_dir.parent().filter(|agents| {
                agents.file_name().is_some_and(|n| n == "agents") && agents.is_dir()
            })
        } else {
            None
        }
    });
    let Some(agents_dir) = agents_dir else {
        return vec![("main".to_string(), main_wire.to_path_buf())];
    };

    let mut logs = vec![("main".to_string(), main_wire.to_path_buf())];
    if let Ok(entries) = fs::read_dir(agents_dir) {
        let mut siblings: Vec<(String, PathBuf)> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let dir = entry.path();
                if !dir.is_dir() {
                    return None;
                }
                let name = dir.file_name()?.to_string_lossy().into_owned();
                if name == "main" {
                    return None;
                }
                let wire = dir.join("wire.jsonl");
                wire.is_file().then_some((name, wire))
            })
            .collect();
        siblings.sort_by(|a, b| a.0.cmp(&b.0));
        logs.extend(siblings);
    }
    logs
}

/// Hub agent id a wire-level agent's captures record under: `kimi` for the
/// main loop (unchanged from #309, so existing dedup hashes still match), or
/// `kimi-<worker>` for subagents.
pub fn kimi_capture_agent_id(wire_agent: &str) -> String {
    if wire_agent == "main" {
        return "kimi".to_string();
    }
    let sanitized: String = wire_agent
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        "kimi-subagent".to_string()
    } else {
        format!("kimi-{sanitized}")
    }
}

/// Extract the assistant text from one wire event, if it is one of the two
/// known assistant-output shapes. Unknown event types yield `None`.
fn extract_assistant_text(value: &serde_json::Value) -> Option<String> {
    let event_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");
    if event_type == "context.append_loop_event" {
        let event = value.get("event")?;
        if event.get("type").and_then(|t| t.as_str()) != Some("content.part") {
            return None;
        }
        let part = event.get("part")?;
        if part.get("type").and_then(|t| t.as_str()) != Some("text") {
            return None;
        }
        let text = part.get("text").and_then(|t| t.as_str())?;
        return (!text.trim().is_empty()).then(|| text.to_string());
    }
    if event_type == "context.append_message" {
        let msg = value.get("message")?;
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        let kind = msg
            .pointer("/origin/kind")
            .and_then(|k| k.as_str())
            .unwrap_or("");
        if role != "assistant" || kind == "injected" {
            return None;
        }
        let content = msg.get("content")?;
        if let Some(s) = content.as_str() {
            return (!s.trim().is_empty()).then(|| s.to_string());
        }
        if let Some(arr) = content.as_array() {
            let texts: Vec<String> = arr
                .iter()
                .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|item| item.get("text").and_then(|s| s.as_str()))
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
                .collect();
            return (!texts.is_empty()).then_some(texts.join("\n"));
        }
    }
    None
}

/// Extract attributed assistant texts from the tail of one wire log.
/// `wire_agent` is the log's own agent (directory name); an event carrying
/// an explicit `agentId` overrides it, preserving subagent attribution for
/// swarm events multiplexed into the main wire (#363).
pub fn recent_kimi_wire_texts(
    path: &Path,
    tail_lines: usize,
    wire_agent: &str,
) -> Vec<KimiWireCapture> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = raw.lines().collect();
    let start = lines.len().saturating_sub(tail_lines);

    let mut captures = Vec::new();
    for line in &lines[start..] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(text) = extract_assistant_text(&value) else {
            continue;
        };
        let agent = value
            .get("agentId")
            .and_then(|a| a.as_str())
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .unwrap_or(wire_agent)
            .to_string();
        captures.push(KimiWireCapture { agent, text });
    }
    captures
}

/// Capture assistant texts from every wire log of one Kimi session into the
/// Hub transcript. Swarm-aware (#363): sibling subagent wires and
/// multiplexed `agentId` events keep their attribution.
pub fn capture_kimi_session_from(
    sessions_root: &Path,
    store: &HubStore,
    workspace: &Path,
    kimi_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<KimiCaptureOutcome, String> {
    let session_id = kimi_session_id.unwrap_or("");
    let Some(main_wire) = kimi_session_log_path(sessions_root, session_id) else {
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

    let mut scanned = 0usize;
    let mut captured = Vec::new();
    for (wire_agent, wire) in kimi_session_wire_logs(&main_wire) {
        for capture in recent_kimi_wire_texts(&wire, 200, &wire_agent) {
            scanned += 1;
            let agent_id = kimi_capture_agent_id(&capture.agent);
            if let Some(record) = store
                .record_harness_capture(
                    "kimi",
                    &agent_id,
                    hub_session_id,
                    &capture.text,
                    Some(&workspace.to_string_lossy()),
                )
                .map_err(|error| error.to_string())?
            {
                captured.push(record);
            }
        }
    }

    Ok(KimiCaptureOutcome {
        transcript_found: true,
        scanned,
        captured,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_wire(dir: &Path, agent: &str, content: &str) -> PathBuf {
        let wire_dir = dir.join("agents").join(agent);
        fs::create_dir_all(&wire_dir).unwrap();
        let wire = wire_dir.join("wire.jsonl");
        let mut f = fs::File::create(&wire).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        wire
    }

    #[test]
    fn extracts_text_and_skips_think_injected_and_unknown_events() {
        let dir = tempdir().unwrap();
        let log = dir.path().join("wire.jsonl");
        let content = r#"{"type":"permission.set_mode","agentId":"main","mode":"auto","time":100}
{"type":"turn.prompt","agentId":"main","input":[{"type":"text","text":"user prompt"}],"origin":{"kind":"user"},"time":101}
{"type":"context.append_message","agentId":"main","message":{"role":"user","content":[{"type":"text","text":"user prompt"}]},"time":102}
{"type":"context.append_message","agentId":"main","message":{"role":"assistant","content":"system injection prompt","origin":{"kind":"injected"}},"time":103}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"think","think":"Internal reasoning to ignore"}},"time":104}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"First assistant response"}},"time":105}
{"type":"swarm.dispatch","agentId":"main","workers":["coder","explore"],"time":106}
{"type":"subagent.spawn","agentId":"coder","task":"scan","time":107}
not json at all
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"Second assistant turn"}},"time":108}
"#;
        fs::write(&log, content).unwrap();
        let texts = recent_kimi_wire_texts(&log, 100, "main");
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0].text, "First assistant response");
        assert_eq!(texts[1].text, "Second assistant turn");
        assert!(texts.iter().all(|t| t.agent == "main"));
    }

    #[test]
    fn multiplexed_agent_id_events_keep_subagent_attribution() {
        let dir = tempdir().unwrap();
        let log = dir.path().join("wire.jsonl");
        let content = r#"{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"main says"}},"time":1}
{"type":"context.append_loop_event","agentId":"coder","event":{"type":"content.part","part":{"type":"text","text":"coder says"}},"time":2}
{"type":"context.append_message","message":{"role":"assistant","content":[{"type":"text","text":"no agentId, falls back to wire agent"}]},"time":3}
"#;
        fs::write(&log, content).unwrap();
        let texts = recent_kimi_wire_texts(&log, 100, "main");
        assert_eq!(texts.len(), 3);
        assert_eq!(texts[0].agent, "main");
        assert_eq!(texts[1].agent, "coder");
        assert_eq!(texts[2].agent, "main");
    }

    #[test]
    fn wire_logs_enumerates_main_then_sorted_subagent_siblings() {
        let dir = tempdir().unwrap();
        let main = write_wire(dir.path(), "main", "{}\n");
        write_wire(dir.path(), "explore", "{}\n");
        write_wire(dir.path(), "coder", "{}\n");
        let logs = kimi_session_wire_logs(&main);
        let names: Vec<&str> = logs.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["main", "coder", "explore"]);
    }

    #[test]
    fn legacy_single_wire_layout_yields_main_only() {
        let dir = tempdir().unwrap();
        let wire = dir.path().join("wire.jsonl");
        fs::write(&wire, "{}\n").unwrap();
        let logs = kimi_session_wire_logs(&wire);
        assert_eq!(logs, vec![("main".to_string(), wire)]);
    }

    #[test]
    fn capture_agent_id_namespaces_subagents() {
        assert_eq!(kimi_capture_agent_id("main"), "kimi");
        assert_eq!(kimi_capture_agent_id("coder"), "kimi-coder");
        assert_eq!(
            kimi_capture_agent_id("Explore Worker 2"),
            "kimi-explore-worker-2"
        );
        assert_eq!(kimi_capture_agent_id("///"), "kimi-subagent");
    }

    #[test]
    fn capture_records_swarm_subagents_with_attribution_and_dedups() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path().join("hub")).unwrap();
        let sessions_root = dir.path().join("sessions");
        let session_dir = sessions_root.join("wd_test_123").join("session_abc-123");
        fs::create_dir_all(&session_dir).unwrap();
        fs::write(
            session_dir.join("state.json"),
            serde_json::to_vec(&serde_json::json!({
                "id": "session_abc-123",
                "cwd": "/test/workspace",
                "updatedAt": 1000_u64,
                "archived": false
            }))
            .unwrap(),
        )
        .unwrap();

        write_wire(
            &session_dir,
            "main",
            concat!(
                "{\"type\":\"context.append_loop_event\",\"agentId\":\"main\",\"event\":{\"type\":\"content.part\",\"part\":{\"type\":\"text\",\"text\":\"main summary\"}},\"time\":1}\n",
                "{\"type\":\"context.append_loop_event\",\"agentId\":\"coder\",\"event\":{\"type\":\"content.part\",\"part\":{\"type\":\"text\",\"text\":\"coder multiplexed into main wire\"}},\"time\":2}\n",
                "{\"type\":\"swarm.unknown_future_event\",\"agentId\":\"plan\",\"payload\":{}}\n",
            ),
        );
        write_wire(
            &session_dir,
            "coder",
            "{\"type\":\"context.append_loop_event\",\"event\":{\"type\":\"content.part\",\"part\":{\"type\":\"text\",\"text\":\"coder own wire\"}},\"time\":3}\n",
        );
        write_wire(
            &session_dir,
            "explore",
            "{\"type\":\"context.append_message\",\"message\":{\"role\":\"assistant\",\"content\":\"explore own wire\"},\"time\":4}\n",
        );

        let out = capture_kimi_session_from(
            &sessions_root,
            &store,
            Path::new("/test/workspace"),
            Some("session_abc-123"),
            None,
        )
        .unwrap();
        assert!(out.transcript_found);
        assert_eq!(out.scanned, 4);
        assert_eq!(out.captured.len(), 4);
        let by_agent: std::collections::BTreeMap<String, Vec<String>> =
            out.captured
                .iter()
                .fold(std::collections::BTreeMap::new(), |mut acc, record| {
                    acc.entry(record.from_agent.clone())
                        .or_default()
                        .push(record.body.clone());
                    acc
                });
        assert_eq!(by_agent["kimi"], vec!["main summary".to_string()]);
        assert_eq!(
            by_agent["kimi-coder"],
            vec![
                "coder multiplexed into main wire".to_string(),
                "coder own wire".to_string()
            ]
        );
        assert_eq!(
            by_agent["kimi-explore"],
            vec!["explore own wire".to_string()]
        );

        // Second poll dedups everything, across all wires.
        let out2 = capture_kimi_session_from(
            &sessions_root,
            &store,
            Path::new("/test/workspace"),
            Some("session_abc-123"),
            None,
        )
        .unwrap();
        assert_eq!(out2.scanned, 4);
        assert_eq!(out2.captured.len(), 0);
    }
}
