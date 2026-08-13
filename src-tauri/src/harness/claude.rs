//! C12 Claude adapter.
//!
//! Start/inject reuse the shared `hub::harness` contract (explicit argv,
//! no shell strings, no TUI attach) — nothing Claude-specific is needed
//! there. The real gap this file closes is **capture**: Claude Code has no
//! app-server/JSONL-RPC stream to poll like Codex, but it does write its own
//! session transcript to disk. The official `claude` CLI reads this same
//! directory to render `/resume`; this reverse-engineers the on-disk shape
//! (`~/.claude/projects/<workspace-with-slashes-as-dashes>/<session>.jsonl`)
//! the same way the Claude Code usage-quota adapter reverse-engineered the
//! `/usage` endpoint — real data, not a mock.
//!
//! Only final assistant **text** replies are captured (not `thinking` or
//! `tool_use` blocks) — those are Claude's actual authored messages to a
//! human/team, which is what the session transcript in the hub wants.

use hub::HubStore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

fn claude_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude")
}

/// Mirrors the official `claude` CLI's own project-directory naming: the
/// absolute workspace path with every `/` replaced by `-`.
pub(crate) fn encode_workspace_dir_name(workspace: &Path) -> String {
    workspace.to_string_lossy().replace('/', "-")
}

fn session_transcript_dir(projects_dir: &Path, workspace: &Path) -> PathBuf {
    projects_dir.join(encode_workspace_dir_name(workspace))
}

/// Picks the most recently modified `<session-id>.jsonl` transcript for a
/// workspace, since that's the one an active `claude` session is writing to.
/// An explicit `session_id` is honored when given and present on disk.
/// `projects_dir` is `~/.claude/projects` in production and an injected
/// scratch directory in tests, so tests never depend on or mutate the real
/// `$HOME`.
fn latest_transcript_path(
    projects_dir: &Path,
    workspace: &Path,
    session_id: Option<&str>,
) -> Option<PathBuf> {
    let dir = session_transcript_dir(projects_dir, workspace);
    if let Some(session_id) = session_id {
        let candidate = dir.join(format!("{session_id}.jsonl"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let entries = fs::read_dir(&dir).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|path| {
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

/// Extracts every `type: "assistant"` text-block message from a transcript.
/// Only the last `tail_lines` lines are parsed — Claude Code transcripts can
/// grow to several megabytes over a long session, and captures only need the
/// recent tail; older lines were already captured on a prior poll (the hub
/// dedups on content hash regardless, so re-scanning a bit of overlap is
/// harmless, just wasted work past what's needed).
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
        if value.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let Some(content) = value
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
        else {
            continue;
        };
        let text: String = content
            .iter()
            .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n");
        if !text.trim().is_empty() {
            texts.push(text);
        }
    }
    texts
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeCaptureOutcome {
    pub transcript_found: bool,
    pub scanned: usize,
    pub captured: Vec<hub::MessageRecord>,
}

/// Reads the newest Claude Code session transcript for `workspace`, extracts
/// recent assistant text replies, and durably records each as a harness
/// capture (deduped on content hash by the store, so this is safe to call
/// repeatedly / on every "Refresh" click rather than needing a background
/// watcher).
///
/// Two distinct identifiers are involved here and must not be conflated:
/// - `disk_session_id`: the Claude Code CLI's own transcript filename
///   (`<disk_session_id>.jsonl` under the workspace's project directory).
///   Optional — when absent, the most recently modified transcript is used,
///   which is what an active session is currently writing to anyway.
/// - `hub_session_id`: the Chat & Memory work-session uuid this capture
///   should be attributed to (`channel:session:<hub_session_id>:capture`).
///   Optional — when absent, the capture lands in the team-wide feed instead
///   of a specific session channel.
///
/// Passing the hub session id where the disk id was expected (or vice
/// versa) either silently misses the real transcript file or mislabels the
/// capture into the wrong — or no — session channel, since the two ids come
/// from entirely different systems and have no reason to match.
pub fn capture_claude_session(
    store: &HubStore,
    workspace: &Path,
    disk_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<ClaudeCaptureOutcome, String> {
    capture_claude_session_from(
        &claude_home().join("projects"),
        store,
        workspace,
        disk_session_id,
        hub_session_id,
    )
}

pub(crate) fn capture_claude_session_from(
    projects_dir: &Path,
    store: &HubStore,
    workspace: &Path,
    disk_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<ClaudeCaptureOutcome, String> {
    let Some(path) = latest_transcript_path(projects_dir, workspace, disk_session_id) else {
        return Ok(ClaudeCaptureOutcome {
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
                "claude",
                "claude",
                hub_session_id,
                text,
                Some(&workspace.to_string_lossy()),
            )
            .map_err(|error| error.to_string())?
        {
            captured.push(record);
        }
    }
    Ok(ClaudeCaptureOutcome {
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
    fn encodes_workspace_path_the_same_way_the_official_cli_does() {
        let workspace = Path::new("/home/pkhunter/Repositories/Repos/Coding-Assistants");
        assert_eq!(
            encode_workspace_dir_name(workspace),
            "-home-pkhunter-Repositories-Repos-Coding-Assistants"
        );
    }

    fn write_transcript(dir: &Path, session_id: &str, lines: &[&str]) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!("{session_id}.jsonl"));
        let mut file = fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    #[test]
    fn extracts_only_assistant_text_blocks_not_thinking_or_tool_use() {
        let dir = tempdir().unwrap();
        let lines = [
            r#"{"type":"system","uuid":"a"}"#,
            r#"{"type":"user","message":{"role":"user","content":"hi"}}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"pondering"}]}}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read"}]}}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Here is the review."}]}}"#,
        ];
        write_transcript(dir.path(), "session-1", &lines);
        let texts = recent_assistant_texts(&dir.path().join("session-1.jsonl"), 500);
        assert_eq!(texts, vec!["Here is the review.".to_string()]);
    }

    #[test]
    fn picks_the_most_recently_modified_transcript_when_no_session_given() {
        let projects_dir = tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        let session_dir = session_transcript_dir(projects_dir.path(), workspace);
        write_transcript(&session_dir, "older", &[r#"{"type":"system"}"#]);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let newer = write_transcript(&session_dir, "newer", &[r#"{"type":"system"}"#]);
        let picked = latest_transcript_path(projects_dir.path(), workspace, None);
        assert_eq!(picked, Some(newer));
    }

    #[test]
    fn honors_an_explicit_session_id_even_if_not_the_newest_file() {
        let projects_dir = tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        let session_dir = session_transcript_dir(projects_dir.path(), workspace);
        write_transcript(&session_dir, "target", &[r#"{"type":"system"}"#]);
        std::thread::sleep(std::time::Duration::from_millis(10));
        write_transcript(&session_dir, "other-newer", &[r#"{"type":"system"}"#]);
        let picked = latest_transcript_path(projects_dir.path(), workspace, Some("target"));
        assert_eq!(picked, Some(session_dir.join("target.jsonl")));
    }

    #[test]
    fn capture_dedups_repeated_polls_of_the_same_transcript() {
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let projects_dir = tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        let session_dir = session_transcript_dir(projects_dir.path(), workspace);
        write_transcript(
            &session_dir,
            "poll-session",
            &[
                r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"first reply"}]}}"#,
            ],
        );

        let first = capture_claude_session_from(projects_dir.path(), &store, workspace, None, None)
            .unwrap();
        assert!(first.transcript_found);
        assert_eq!(first.captured.len(), 1);

        let second =
            capture_claude_session_from(projects_dir.path(), &store, workspace, None, None)
                .unwrap();
        assert!(second.captured.is_empty(), "identical poll must dedup");
    }

    #[test]
    fn missing_transcript_directory_reports_not_found_without_erroring() {
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let projects_dir = tempdir().unwrap();
        let outcome = capture_claude_session_from(
            projects_dir.path(),
            &store,
            Path::new("/nonexistent/workspace"),
            None,
            None,
        )
        .unwrap();
        assert!(!outcome.transcript_found);
        assert!(outcome.captured.is_empty());
    }

    #[test]
    fn disk_session_id_and_hub_session_id_serve_distinct_purposes() {
        // The disk session id picks a specific transcript file; the hub
        // session id scopes where the capture is filed. A hub work-session
        // uuid must never be used to look up a transcript filename, and a
        // disk transcript filename must never be used as the hub channel id.
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let projects_dir = tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        let session_dir = session_transcript_dir(projects_dir.path(), workspace);
        // The real Claude Code transcript file is named after its own disk
        // session id, which looks nothing like a hub work-session uuid.
        write_transcript(
            &session_dir,
            "claude-cli-own-session-id",
            &[
                r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"scoped to a hub session"}]}}"#,
            ],
        );

        let hub_session_id = "11111111-2222-3333-4444-555555555555";
        let outcome = capture_claude_session_from(
            projects_dir.path(),
            &store,
            workspace,
            Some("claude-cli-own-session-id"),
            Some(hub_session_id),
        )
        .unwrap();
        assert!(
            outcome.transcript_found,
            "disk session id must resolve the real file"
        );
        assert_eq!(outcome.captured.len(), 1);

        let recorded = store
            .list_channel_messages(&format!("session:{hub_session_id}"), 10)
            .unwrap();
        assert_eq!(
            recorded.len(),
            1,
            "capture must land in the hub session's own channel, not the disk session id's"
        );
    }
}

#[cfg(test)]
mod manual_smoke {
    use super::*;

    #[test]
    #[ignore = "reads real ~/.claude/projects data on this machine; run manually with --ignored"]
    fn real_transcript_smoke_check() {
        let store_dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let outcome = capture_claude_session(&store, &workspace, None, None).unwrap();
        eprintln!(
            "transcript_found={} scanned={} captured={}",
            outcome.transcript_found,
            outcome.scanned,
            outcome.captured.len()
        );
        assert!(outcome.transcript_found);
    }
}
