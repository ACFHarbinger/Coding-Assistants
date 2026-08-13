use hub::HubStore;
use std::path::PathBuf;

#[derive(serde::Serialize)]
pub(crate) struct HarnessCaptureOutcome {
    harness: String,
    transcript_found: bool,
    scanned: usize,
    captured: Vec<hub::MessageRecord>,
}

pub(crate) fn home_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
}

pub(crate) fn recent_json_lines(
    path: &std::path::Path,
    tail_lines: usize,
) -> Vec<serde_json::Value> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = raw.lines().collect();
    let start = lines.len().saturating_sub(tail_lines);
    lines[start..]
        .iter()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .collect()
}

/// Grok TUI: `~/.grok/sessions/<percent-encoded-abs-workspace>/<session>/chat_history.jsonl`.
pub(crate) fn grok_encode_workspace(workspace: &std::path::Path) -> String {
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

pub(crate) fn grok_transcript_path(
    sessions_root: &std::path::Path,
    workspace: &std::path::Path,
    disk_session: Option<&str>,
) -> Option<PathBuf> {
    let root = sessions_root.join(grok_encode_workspace(workspace));
    if let Some(session) = disk_session {
        let candidate = root.join(session).join("chat_history.jsonl");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|dir| {
            let history = dir.join("chat_history.jsonl");
            let modified = std::fs::metadata(&history).ok()?.modified().ok()?;
            Some((modified, history))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

pub(crate) fn grok_assistant_texts(path: &std::path::Path) -> Vec<String> {
    recent_json_lines(path, 200)
        .into_iter()
        .filter(|value| value.get("type").and_then(|t| t.as_str()) == Some("assistant"))
        .filter_map(|value| {
            value
                .get("content")
                .and_then(|c| c.as_str())
                .map(str::to_string)
        })
        .filter(|text| !text.trim().is_empty())
        .collect()
}

/// Claude Code: `~/.claude/projects/<workspace-with-slashes-as-dashes>/<session>.jsonl`.
pub(crate) fn claude_encode_workspace(workspace: &std::path::Path) -> String {
    workspace.to_string_lossy().replace('/', "-")
}

pub(crate) fn claude_transcript_path(
    projects_dir: &std::path::Path,
    workspace: &std::path::Path,
    disk_session: Option<&str>,
) -> Option<PathBuf> {
    let dir = projects_dir.join(claude_encode_workspace(workspace));
    if let Some(session) = disk_session {
        let candidate = dir.join(format!("{session}.jsonl"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|path| {
            let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

pub(crate) fn claude_assistant_texts(path: &std::path::Path) -> Vec<String> {
    recent_json_lines(path, 500)
        .into_iter()
        .filter(|value| value.get("type").and_then(|t| t.as_str()) == Some("assistant"))
        .filter_map(|value| {
            let content = value.get("message")?.get("content")?.as_array()?;
            let text: String = content
                .iter()
                .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n\n");
            (!text.trim().is_empty()).then_some(text)
        })
        .collect()
}

/// Codex: `~/.codex/sessions/YYYY/MM/DD/*.jsonl`, matched by `session_meta.cwd`.
pub(crate) fn codex_transcript_paths(sessions_root: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Ok(years) = std::fs::read_dir(sessions_root) else {
        return paths;
    };
    for year in years.filter_map(Result::ok).map(|entry| entry.path()) {
        let Ok(months) = std::fs::read_dir(&year) else {
            continue;
        };
        for month in months.filter_map(Result::ok).map(|entry| entry.path()) {
            let Ok(days) = std::fs::read_dir(&month) else {
                continue;
            };
            for day in days.filter_map(Result::ok).map(|entry| entry.path()) {
                let Ok(files) = std::fs::read_dir(&day) else {
                    continue;
                };
                paths.extend(
                    files
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl")),
                );
            }
        }
    }
    paths
}

pub(crate) fn codex_transcript_metadata(path: &std::path::Path) -> Option<(String, String)> {
    let raw = std::fs::read_to_string(path).ok()?;
    raw.lines().take(16).find_map(|line| {
        let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
        if value.get("type").and_then(|t| t.as_str()) != Some("session_meta") {
            return None;
        }
        let payload = value.get("payload")?;
        Some((
            payload.get("cwd")?.as_str()?.to_string(),
            payload.get("session_id")?.as_str()?.to_string(),
        ))
    })
}

pub(crate) fn codex_transcript_path(
    sessions_root: &std::path::Path,
    workspace: &std::path::Path,
    disk_session: Option<&str>,
) -> Option<PathBuf> {
    let workspace = workspace.to_string_lossy();
    codex_transcript_paths(sessions_root)
        .into_iter()
        .filter_map(|path| {
            let (cwd, session_id) = codex_transcript_metadata(&path)?;
            if cwd != workspace || disk_session.is_some_and(|id| id != session_id) {
                return None;
            }
            Some((std::fs::metadata(&path).ok()?.modified().ok()?, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

pub(crate) fn codex_assistant_texts(path: &std::path::Path) -> Vec<String> {
    recent_json_lines(path, 500)
        .into_iter()
        .filter(|value| value.get("type").and_then(|t| t.as_str()) == Some("response_item"))
        .filter_map(|value| {
            let payload = value.get("payload")?;
            if payload.get("type").and_then(|t| t.as_str()) != Some("message")
                || payload.get("role").and_then(|r| r.as_str()) != Some("assistant")
            {
                return None;
            }
            let content = payload.get("content")?.as_array()?;
            let text: String = content
                .iter()
                .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("output_text"))
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n\n");
            (!text.trim().is_empty()).then_some(text)
        })
        .collect()
}

/// Antigravity CLI: `~/.gemini/antigravity-cli/brain/<conv-id>/.system_generated/logs/transcript.jsonl`.
pub(crate) fn gemini_transcript_path(
    brain_dir: &std::path::Path,
    disk_session: Option<&str>,
) -> Option<PathBuf> {
    if let Some(conv_id) = disk_session {
        let candidate = brain_dir
            .join(conv_id)
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    std::fs::read_dir(brain_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter_map(|dir| {
            let log_file = dir
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");
            let modified = std::fs::metadata(&log_file).ok()?.modified().ok()?;
            Some((modified, log_file))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

pub(crate) fn gemini_assistant_texts(path: &std::path::Path) -> Vec<String> {
    recent_json_lines(path, 500)
        .into_iter()
        .filter(|value| {
            value.get("source").and_then(|s| s.as_str()) == Some("MODEL")
                || value.get("type").and_then(|t| t.as_str()) == Some("PLANNER_RESPONSE")
        })
        .filter_map(|value| {
            value
                .get("content")
                .and_then(|c| c.as_str())
                .map(str::trim)
                .map(str::to_string)
        })
        .filter(|text| !text.is_empty() && !text.starts_with("```json"))
        .collect()
}

pub(crate) fn capture_harness_session(
    store: &HubStore,
    harness: &str,
    workspace: &std::path::Path,
    disk_session: Option<&str>,
    hub_session: Option<&str>,
) -> anyhow::Result<HarnessCaptureOutcome> {
    let (agent_id, texts, transcript_found) = match harness {
        "grok" => {
            let workspace = workspace
                .canonicalize()
                .unwrap_or_else(|_| workspace.to_path_buf());
            match grok_transcript_path(
                &home_dir().join(".grok").join("sessions"),
                &workspace,
                disk_session,
            ) {
                Some(path) => ("grok", grok_assistant_texts(&path), true),
                None => ("grok", Vec::new(), false),
            }
        }
        "claude" => {
            match claude_transcript_path(
                &home_dir().join(".claude").join("projects"),
                workspace,
                disk_session,
            ) {
                Some(path) => ("claude", claude_assistant_texts(&path), true),
                None => ("claude", Vec::new(), false),
            }
        }
        "chat" | "codex" => {
            let workspace = workspace
                .canonicalize()
                .unwrap_or_else(|_| workspace.to_path_buf());
            match codex_transcript_path(
                &home_dir().join(".codex").join("sessions"),
                &workspace,
                disk_session,
            ) {
                Some(path) => ("chat", codex_assistant_texts(&path), true),
                None => ("chat", Vec::new(), false),
            }
        }
        "gemini" | "agy" => {
            let brain_dir = home_dir()
                .join(".gemini")
                .join("antigravity-cli")
                .join("brain");
            match gemini_transcript_path(&brain_dir, disk_session) {
                Some(path) => ("gemini", gemini_assistant_texts(&path), true),
                None => ("gemini", Vec::new(), false),
            }
        }
        other => anyhow::bail!("unknown harness: {other} (expected grok, claude, chat, or gemini)"),
    };

    let mut captured = Vec::new();
    if transcript_found {
        for text in &texts {
            if let Some(record) = store.record_harness_capture(
                harness,
                agent_id,
                hub_session,
                text,
                Some(&workspace.to_string_lossy()),
            )? {
                captured.push(record);
            }
        }
    }
    Ok(HarnessCaptureOutcome {
        harness: harness.to_string(),
        transcript_found,
        scanned: texts.len(),
        captured,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tagged_dispatch_workspace;
    use std::path::Path;

    #[test]
    fn tagged_dispatch_is_opt_in_and_requires_an_absolute_workspace() {
        assert_eq!(tagged_dispatch_workspace(false, None).unwrap(), None);
        assert!(tagged_dispatch_workspace(true, None).is_err());
        assert!(tagged_dispatch_workspace(true, Some("relative/workspace")).is_err());
        assert_eq!(
            tagged_dispatch_workspace(true, Some("/tmp/c12-cli-dispatch"))
                .unwrap()
                .as_deref(),
            Some(Path::new("/tmp/c12-cli-dispatch"))
        );
    }

    #[test]
    fn unknown_harness_id_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result =
            capture_harness_session(&store, "not-a-harness", Path::new("/tmp/x"), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn grok_capture_extracts_assistant_text_and_dedups() {
        let sessions_root = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = Path::new("/tmp/cli-c12-grok");
        let dir = sessions_root
            .path()
            .join(grok_encode_workspace(workspace))
            .join("sess-1");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("chat_history.jsonl"),
            "{\"type\":\"reasoning\",\"content\":\"thinking\"}\n{\"type\":\"assistant\",\"content\":\"cli capture works\"}\n",
        )
        .unwrap();
        let path = grok_transcript_path(sessions_root.path(), workspace, None).unwrap();
        let texts = grok_assistant_texts(&path);
        assert_eq!(texts, vec!["cli capture works".to_string()]);

        // Dedup happens in the store, not in the path/parsing helpers above —
        // exercise that boundary directly here.
        let first = store
            .record_harness_capture("grok", "grok", Some("hub-1"), &texts[0], None)
            .unwrap();
        assert!(first.is_some());
        let second = store
            .record_harness_capture("grok", "grok", Some("hub-1"), &texts[0], None)
            .unwrap();
        assert!(second.is_none(), "repeat capture must dedup");
    }

    #[test]
    fn claude_capture_skips_thinking_and_tool_use_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        let session_dir = dir.path().join(claude_encode_workspace(workspace));
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(
            session_dir.join("s1.jsonl"),
            "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"x\"}]}}\n\
             {\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"cli claude capture\"}]}}\n",
        )
        .unwrap();
        let path = claude_transcript_path(dir.path(), workspace, None).unwrap();
        assert_eq!(
            claude_assistant_texts(&path),
            vec!["cli claude capture".to_string()]
        );
    }

    #[test]
    fn codex_capture_matches_by_workspace_and_disk_session() {
        let root = tempfile::tempdir().unwrap();
        let day_dir = root.path().join("2026").join("08").join("13");
        std::fs::create_dir_all(&day_dir).unwrap();
        let workspace = Path::new("/tmp/cli-c12-codex");
        std::fs::write(
            day_dir.join("rollout.jsonl"),
            format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}\",\"session_id\":\"disk-a\"}}}}\n\
                 {{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{{\"type\":\"output_text\",\"text\":\"cli codex capture\"}}]}}}}\n",
                workspace.display()
            ),
        )
        .unwrap();
        let path = codex_transcript_path(root.path(), workspace, Some("disk-a")).unwrap();
        assert_eq!(
            codex_assistant_texts(&path),
            vec!["cli codex capture".to_string()]
        );
        assert!(codex_transcript_path(root.path(), workspace, Some("disk-b")).is_none());
    }

    #[test]
    fn gemini_capture_extracts_model_responses_only() {
        let brain_dir = tempfile::tempdir().unwrap();
        let logs_dir = brain_dir
            .path()
            .join("conv-1")
            .join(".system_generated")
            .join("logs");
        std::fs::create_dir_all(&logs_dir).unwrap();
        std::fs::write(
            logs_dir.join("transcript.jsonl"),
            "{\"source\":\"USER_EXPLICIT\",\"content\":\"hi\"}\n\
             {\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"content\":\"cli gemini capture\"}\n",
        )
        .unwrap();
        let path = gemini_transcript_path(brain_dir.path(), None).unwrap();
        assert_eq!(
            gemini_assistant_texts(&path),
            vec!["cli gemini capture".to_string()]
        );
    }
}
