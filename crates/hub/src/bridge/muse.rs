//! C14.11 Meta Muse Code bridge (#273).
//!
//! Headless runs use `muse exec <prompt>` with cwd set to the workspace
//! (see `harness::muse_spawn_args`); managed runs additionally pin
//! `--session-id <uuid>` so the transcript below stays addressable.
//! Capture reads the same on-disk event log the CLI writes:
//! `<sessions-root>/YYYY/MM/DD/<session-uuid>/session.jsonl` (verified live
//! against muse 1.0.3 — the store is date-sharded, and assistant text lives
//! in `payload_type == "runtime.session"` records whose
//! `payload.event.kind == "assistant_message_committed"`).
//!
//! Observed external sessions stay capture-only: without a registration or
//! an explicit id the adapter returns `unavailable`/empty rather than
//! grabbing the newest outside transcript. `muse session-message` ingress
//! is closed in current builds (`external_agent_ingress_closed`, verified
//! #273 spike), so task-only injects stay queued — there is no
//! live-delivery bridge to route through.

use std::fs;
use std::path::{Path, PathBuf};

/// Sessions root honoring `${XDG_DATA_HOME:-$HOME/.local/share}/muse/sessions`.
pub fn muse_sessions_root() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("muse").join("sessions")
}

/// Bound on directory entries visited while locating one session log — the
/// store grows a directory per session per day, so an unbounded walk would
/// punish long-lived installs on every resume click.
const MAX_VISITED_DIRS: usize = 2000;

/// Directory names that are never session logs: view caches, spilled tool
/// output, and delegated-subagent trees (their findings live in the parent
/// session, not as resumable top-level sessions).
fn is_session_tree_excluded(name: &str) -> bool {
    name.starts_with('.')
        || name == "tool-outputs"
        || name == "subagent"
        || name == "approval-review"
}

/// Locate `<session-id>/session.jsonl` under a sessions root, tolerating the
/// date-sharded layout (`YYYY/MM/DD/<uuid>/`) as well as a flat
/// `<uuid>/` fixture layout in tests.
pub fn muse_session_log_path(sessions_root: &Path, session_id: &str) -> Option<PathBuf> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return None;
    }
    // Fast path: the caller already knows the shard (or the flat layout).
    let mut stack = vec![sessions_root.to_path_buf()];
    let mut visited = 0;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > MAX_VISITED_DIRS {
            return None;
        }
        if dir.file_name().and_then(|name| name.to_str()) == Some(session_id) {
            let log = dir.join("session.jsonl");
            if log.is_file() {
                return Some(log);
            }
        }
        // Recurse newest-first so a resume scan meets fresh sessions early.
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut subdirs: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            // Never wander into tool-output or peer-log trees.
            .filter(|path| {
                !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(is_session_tree_excluded)
            })
            .map(|path| {
                let modified = fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                (modified, path)
            })
            .collect();
        subdirs.sort_by_key(|(modified, _)| *modified);
        stack.extend(subdirs.into_iter().map(|(_, path)| path));
    }
    None
}

/// The workspace a session log belongs to: `payload.record.workspace_root`
/// of the first `runtime.session.metadata` or
/// `session.workspace_branch.observed` record. Only the log head is read —
/// identity records are written at session open, so a bounded prefix
/// suffices and multi-megabyte transcripts stay cheap to attribute.
pub fn muse_session_workspace(log_path: &Path) -> Option<String> {
    const HEAD_BYTES: usize = 64 * 1024;
    let raw = fs::read(log_path).ok()?;
    let head = &raw[..raw.len().min(HEAD_BYTES)];
    let text = String::from_utf8_lossy(head);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || !line.contains("workspace_root") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let is_identity = value
            .get("payload_type")
            .and_then(|item| item.as_str())
            .is_some_and(|kind| {
                kind == "runtime.session.metadata" || kind == "session.workspace_branch.observed"
            });
        if !is_identity {
            continue;
        }
        if let Some(root) = value
            .pointer("/payload/record/workspace_root")
            .and_then(|item| item.as_str())
        {
            let root = root.trim();
            if !root.is_empty() {
                return Some(root.to_string());
            }
        }
    }
    None
}

fn workspace_matches(recorded: &str, workspace: &Path) -> bool {
    if recorded == workspace.to_string_lossy() {
        return true;
    }
    let wanted = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    if recorded == wanted.to_string_lossy() {
        return true;
    }
    Path::new(recorded).canonicalize().ok().as_ref() == Some(&wanted)
}

/// Most recent Muse session id for `workspace`, newest transcript first.
/// Used by "Resume in terminal" — never guessed from process output.
pub fn latest_muse_session_id(workspace: &Path) -> Option<String> {
    latest_muse_session_id_from(&muse_sessions_root(), workspace, 200)
}

fn latest_muse_session_id_from(
    sessions_root: &Path,
    workspace: &Path,
    candidate_cap: usize,
) -> Option<String> {
    // Collect session-log candidates newest-first, then attribute only until
    // the first workspace match — attributing the whole store on every
    // resume click would re-read hundreds of log heads.
    let mut logs: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    let mut stack = vec![sessions_root.to_path_buf()];
    let mut visited = 0;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > MAX_VISITED_DIRS {
            break;
        }
        let log = dir.join("session.jsonl");
        if dir
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| !is_session_tree_excluded(name))
            && log.is_file()
        {
            let modified = fs::metadata(&log)
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            logs.push((modified, log));
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        stack.extend(
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .filter(|path| {
                    !path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(is_session_tree_excluded)
                }),
        );
    }
    logs.sort_by_key(|(modified, _)| *modified);
    logs.iter()
        .rev()
        .take(candidate_cap)
        .filter_map(|(_, log)| {
            let id = log.parent()?.file_name()?.to_string_lossy().into_owned();
            let recorded = muse_session_workspace(log)?;
            workspace_matches(&recorded, workspace).then_some(id)
        })
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_log(dir: &Path, workspace_root: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let mut file = std::fs::File::create(dir.join("session.jsonl")).unwrap();
        writeln!(
            file,
            r#"{{"payload_type":"runtime.session.metadata","payload":{{"kind":"metadata","record":{{"workspace_root":"{workspace_root}"}}}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"payload_type":"runtime.session","payload":{{"kind":"run","event":{{"kind":"assistant_message_committed","text":"hi"}}}}}}"#
        )
        .unwrap();
    }

    #[test]
    fn finds_a_sharded_session_log_by_id() {
        let root = tempfile::tempdir().unwrap();
        let dir = root
            .path()
            .join("2026")
            .join("09")
            .join("08")
            .join("sess-1");
        write_log(&dir, "/tmp/ws");
        let found = muse_session_log_path(root.path(), "sess-1").unwrap();
        assert_eq!(found, dir.join("session.jsonl"));
        assert!(muse_session_log_path(root.path(), "nope").is_none());
        assert!(muse_session_log_path(root.path(), "  ").is_none());
    }

    #[test]
    fn attributes_a_log_to_its_workspace_and_picks_newest() {
        use std::time::{Duration, SystemTime};
        let root = tempfile::tempdir().unwrap();
        // Fixture creation is faster than mtime granularity, so pin explicit
        // mtimes — otherwise "newest" is filesystem luck.
        let stamp = |dir: &Path, age_secs: u64| {
            let time = SystemTime::now() - Duration::from_secs(age_secs);
            std::fs::File::options()
                .write(true)
                .open(dir.join("session.jsonl"))
                .unwrap()
                .set_modified(time)
                .unwrap();
        };
        // Older session for another workspace must not win.
        let old = root
            .path()
            .join("2026")
            .join("09")
            .join("07")
            .join("sess-old");
        write_log(&old, "/tmp/other-ws");
        stamp(&old, 200);
        // Newer session for the wanted workspace wins.
        let new = root
            .path()
            .join("2026")
            .join("09")
            .join("08")
            .join("sess-new");
        write_log(&new, "/tmp/ws");
        stamp(&new, 100);
        // Same-workspace older log loses to the newer one.
        let older = root
            .path()
            .join("2026")
            .join("09")
            .join("06")
            .join("sess-older");
        write_log(&older, "/tmp/ws");
        stamp(&older, 300);

        let latest = latest_muse_session_id_from(root.path(), Path::new("/tmp/ws"), 200).unwrap();
        assert_eq!(latest, "sess-new");
        assert!(latest_muse_session_id_from(root.path(), Path::new("/tmp/unknown"), 200).is_none());
    }

    #[test]
    fn skips_hidden_and_tool_output_trees() {
        let root = tempfile::tempdir().unwrap();
        let hidden = root.path().join(".msp-view-v1").join("sess-hidden");
        write_log(&hidden, "/tmp/ws");
        assert!(muse_session_log_path(root.path(), "sess-hidden").is_none());
    }
}
