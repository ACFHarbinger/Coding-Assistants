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
//! Task delivery re-enters the managed session headlessly: `muse exec
//! --session-id <uuid> <task>` appends a run to the same transcript the
//! capture poller reads (verified #273 spike), so delivery both performs
//! the task and arms capture via the writer-lease release. Observed
//! external sessions stay capture-only: delivery into a live TUI the app
//! does not own would inject a headless run into someone else's session,
//! so those return `unavailable` without spawning. (`muse session-message`
//! ingress is closed in current builds — `external_agent_ingress_closed`,
//! verified #273 spike — so a live message-bus delivery is not an option.)

use crate::{
    HarnessInjectRequest, HarnessInjectResult, HarnessSessionMode, HarnessSessionState, HubError,
    HubStore,
};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

/// Deliver a task into a managed Muse session and arm its capture.
///
/// Runs `muse exec --session-id <uuid> <task>` headlessly against the
/// registered managed session id, holding the single-writer lease for the
/// duration. On success the lease releases to `Ready`, which arms the
/// capture poller for the transcript the run just appended; on failure it
/// releases back to `Queued` and the task stays queued for retry.
///
/// Managed-only: without an app-owned managed registration there is no
/// session the app may append to — an observed external TUI stays
/// capture-only and the task stays queued. Never writes a TTY/PTY and
/// never starts an interactive TUI.
pub fn deliver_muse_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_muse_task_with(store, request, run_muse_worker)
}

#[allow(dead_code)]
pub fn deliver_muse_task_with(
    store: &HubStore,
    request: &HarnessInjectRequest,
    runner: impl FnOnce(&Path, &str, &str, Option<&str>, Option<&str>) -> Result<Option<u32>, String>,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Muse active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let workspace_str = workspace.to_string_lossy().into_owned();

    let registration = store.get_harness_session("muse", &workspace_str)?;
    let is_managed = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);
    if !is_managed {
        return Ok(unavailable(
            "Muse active-session delivery requires an app-owned managed session. Register one with hub_register_managed_harness_session. Task stays queued.",
        ));
    }
    // `request.session_id` is the Hub work-session id, not Muse's disk
    // session id. Never pass it to `muse exec --session-id`.
    let registered_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .unwrap_or_default();
    let Some(disk_uuid) = crate::harness::muse_disk_session_id(&registered_id) else {
        return Ok(unavailable(&format!(
            "registered Muse session id {registered_id:?} is not a UUID and cannot be resumed with `muse exec --session-id`. Task stays queued."
        )));
    };

    let writer_owner = format!(
        "muse-worker:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );
    if let Err(error) = store.acquire_harness_writer("muse", &workspace_str, &writer_owner) {
        return Ok(queued(&format!(
            "Muse managed worker in workspace {workspace_str} is busy; task stays queued for retry: {error}"
        )));
    }

    let run_res = runner(
        &workspace,
        &request.body,
        &disk_uuid,
        request.model.as_deref(),
        request.effort.as_deref(),
    );
    let (next_state, result) = match run_res {
        Ok(pid) => {
            if let Some(message_id) = request.message_id.as_deref() {
                let _ = store.set_message_status(message_id, crate::MessageStatus::Acked);
            }
            (
                HarnessSessionState::Ready,
                HarnessInjectResult {
                    harness: "muse".into(),
                    pid,
                    status: "delivered".into(),
                    detail: format!(
                        "forwarded to managed Muse session {disk_uuid}; the run's transcript is captured on the next poll"
                    ),
                },
            )
        }
        Err(error) => (
            HarnessSessionState::Queued,
            HarnessInjectResult {
                harness: "muse".into(),
                pid: None,
                status: "errored".into(),
                detail: format!("Muse worker failed: {error}"),
            },
        ),
    };
    let _ = store.release_harness_writer("muse", &workspace_str, &writer_owner, next_state);
    Ok(result)
}

/// Run one headless `muse exec --session-id <uuid>` task turn and wait for
/// it, returning the worker pid. Stdout is drained and discarded — the
/// event log (not the rendered terminal text) is the capture source of
/// truth, so recording stdout here would risk double-captures under
/// different content hashes. Stderr is dropped for the same pipe-stall
/// reason; a non-zero exit still reports its code.
///
/// No approval-bypass flags are passed: a run that needs an approval the
/// headless worker cannot obtain fails truthfully instead of hanging on a
/// prompt no one will answer (stdin is null).
fn run_muse_worker(
    workspace: &Path,
    prompt: &str,
    disk_uuid: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Option<u32>, String> {
    let args =
        crate::harness::muse_managed_spawn_args(workspace, prompt, Some(disk_uuid), model, effort)
            .map_err(|error| format!("invalid Muse spawn args: {error}"))?;
    let mut child = Command::new("muse")
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to spawn muse: {error}"))?;
    let pid = Some(child.id());
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = line;
        }
    }
    let status = child
        .wait()
        .map_err(|error| format!("error waiting for muse: {error}"))?;
    if status.success() {
        Ok(pid)
    } else {
        Err(format!("muse exited with code {:?}", status.code()))
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "muse".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

fn queued(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "muse".into(),
        pid: None,
        status: "queued".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HarnessInjectRequest, HubStore};
    use std::io::Write;

    const MANAGED_WS: &str = "/tmp/c14-muse-deliver";
    const MANAGED_UUID: &str = "123e4567-e89b-42d3-a456-426614174000";

    fn managed_uuid() -> String {
        format!("managed-{MANAGED_UUID}")
    }

    fn register_queued_managed(store: &HubStore) {
        store
            .register_managed_harness_session_with_state(
                "muse",
                MANAGED_WS,
                &managed_uuid(),
                None,
                crate::HarnessSessionState::Queued,
            )
            .unwrap();
    }

    fn task_request() -> HarnessInjectRequest {
        HarnessInjectRequest {
            harness: "muse".into(),
            workspace: PathBuf::from(MANAGED_WS),
            session_id: Some("hub-session-1".into()),
            message_id: None,
            body: "do the thing".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        }
    }

    fn no_spawn_runner(
        _: &Path,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<Option<u32>, String> {
        panic!("deliver must not spawn without an armed managed session")
    }

    #[test]
    fn deliver_requires_a_managed_registration() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let outcome = deliver_muse_task_with(&store, &task_request(), no_spawn_runner).unwrap();
        assert_eq!(outcome.status, "unavailable");
        assert_eq!(outcome.pid, None);
        assert!(!outcome.detail.contains("spawned"));
    }

    #[test]
    fn deliver_refuses_observed_sessions_without_spawning() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session("muse", MANAGED_WS, MANAGED_UUID, None)
            .unwrap();
        let outcome = deliver_muse_task_with(&store, &task_request(), no_spawn_runner).unwrap();
        assert_eq!(outcome.status, "unavailable");
        assert_eq!(outcome.pid, None);
    }

    #[test]
    fn deliver_runs_the_worker_and_arms_capture() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        register_queued_managed(&store);
        let outcome = deliver_muse_task_with(&store, &task_request(), |_, body, uuid, _, _| {
            // The Hub work-session id must never reach the CLI; the runner
            // gets the stripped disk UUID.
            assert_eq!(body, "do the thing");
            assert_eq!(uuid, MANAGED_UUID);
            Ok(Some(4321))
        })
        .unwrap();
        assert_eq!(outcome.status, "delivered");
        assert_eq!(outcome.pid, Some(4321));
        let row = store
            .get_harness_session("muse", MANAGED_WS)
            .unwrap()
            .unwrap();
        assert_eq!(row.state, crate::HarnessSessionState::Ready);
        assert_eq!(row.disk_session_id, managed_uuid());
        // The lease was released: a new owner can acquire it.
        assert!(store
            .acquire_harness_writer("muse", MANAGED_WS, "next-turn")
            .is_ok());
    }

    #[test]
    fn deliver_failure_returns_the_session_to_queued() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        register_queued_managed(&store);
        let outcome =
            deliver_muse_task_with(&store, &task_request(), |_, _, _, _, _| Err("boom".into()))
                .unwrap();
        assert_eq!(outcome.status, "errored");
        assert_eq!(outcome.pid, None);
        let row = store
            .get_harness_session("muse", MANAGED_WS)
            .unwrap()
            .unwrap();
        assert_eq!(row.state, crate::HarnessSessionState::Queued);
        assert!(store
            .acquire_harness_writer("muse", MANAGED_WS, "retry-turn")
            .is_ok());
    }

    #[test]
    fn deliver_while_busy_stays_queued_without_spawning() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        register_queued_managed(&store);
        store
            .acquire_harness_writer("muse", MANAGED_WS, "other-turn")
            .unwrap();
        let outcome = deliver_muse_task_with(&store, &task_request(), no_spawn_runner).unwrap();
        assert_eq!(outcome.status, "queued");
        assert_eq!(outcome.pid, None);
    }

    #[test]
    fn deliver_rejects_an_unusable_registered_id_without_spawning() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_managed_harness_session_with_state(
                "muse",
                MANAGED_WS,
                "chat-1",
                None,
                crate::HarnessSessionState::Queued,
            )
            .unwrap();
        let outcome = deliver_muse_task_with(&store, &task_request(), no_spawn_runner).unwrap();
        assert_eq!(outcome.status, "unavailable");
        assert!(outcome.detail.contains("UUID"));
    }

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
