//! C12/C14.2 Codex / Chat bridge: deliver a queued Hub task into a persisted
//! Codex thread through the documented app-server JSON-RPC protocol
//! (`initialize`, `thread/resume`, `turn/start`), then route Codex's actual
//! reply back into the Hub once it completes.
//!
//! This never writes another process's PTY, never invents a control socket,
//! and never starts a replacement `codex exec` process for a task-only
//! inject. Without a registered or on-disk thread id the task stays
//! `unavailable` / queued.
//!
//! Split by concern, mirroring `bridge::channels::claude`: this file owns
//! thread discovery and delivery orchestration; [`turn`] speaks the
//! app-server JSON-RPC protocol and captures a turn's reply text;
//! [`reply`] routes that text back into the Hub, addressed to the original
//! sender.

mod reply;
mod turn;

use crate::{
    HarnessInjectRequest, HarnessInjectResult, HarnessSessionMode, HarnessSessionState, HubError,
    HubStore,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use turn::CodexTurnOutcome;

pub const CODEX_AGENT_ID: &str = "chat";

pub use reply::record_codex_reply;

pub fn latest_codex_thread_id(workspace: &Path) -> Option<String> {
    latest_codex_thread_id_from(&codex_sessions_dir(), workspace)
}

fn codex_sessions_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".codex").join("sessions")
}

fn latest_codex_thread_id_from(sessions_root: &Path, workspace: &Path) -> Option<String> {
    let mut best: Option<(std::time::SystemTime, String)> = None;
    let years = std::fs::read_dir(sessions_root).ok()?;
    for year in years.filter_map(Result::ok) {
        let months = std::fs::read_dir(year.path()).ok().into_iter().flatten();
        for month in months.filter_map(Result::ok) {
            let days = std::fs::read_dir(month.path()).ok().into_iter().flatten();
            for day in days.filter_map(Result::ok) {
                let files = std::fs::read_dir(day.path()).ok().into_iter().flatten();
                for file in files.filter_map(Result::ok) {
                    let path = file.path();
                    if path.extension().is_none_or(|ext| ext != "jsonl") {
                        continue;
                    }
                    let Some((cwd, session_id)) = session_meta(&path) else {
                        continue;
                    };
                    if !same_workspace(Path::new(&cwd), workspace) {
                        continue;
                    }
                    let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
                    if best.as_ref().is_none_or(|(stamp, _)| modified > *stamp) {
                        best = Some((modified, session_id));
                    }
                }
            }
        }
    }
    best.map(|(_, id)| id)
}

/// Codex persists a literal cwd in its transcript. Match it by canonical path
/// when both paths exist, so a symlink, `.` segment, or trailing separator
/// cannot make a real thread invisible to the Hub. Fall back to the literal
/// comparison only when the historical workspace no longer exists.
fn same_workspace(recorded: &Path, selected: &Path) -> bool {
    match (recorded.canonicalize(), selected.canonicalize()) {
        (Ok(recorded), Ok(selected)) => recorded == selected,
        _ => recorded == selected,
    }
}

fn session_meta(path: &Path) -> Option<(String, String)> {
    let raw = std::fs::read_to_string(path).ok()?;
    raw.lines().take(16).find_map(|line| {
        let value = serde_json::from_str::<Value>(line).ok()?;
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            return None;
        }
        let payload = value.get("payload")?;
        Some((
            payload.get("cwd")?.as_str()?.to_string(),
            payload.get("session_id")?.as_str()?.to_string(),
        ))
    })
}

pub fn deliver_codex_task(
    store: &HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    deliver_codex_task_with(store, request, turn::send_codex_turn)
}

fn deliver_codex_task_with(
    store: &HubStore,
    request: &HarnessInjectRequest,
    send_turn: impl FnOnce(&str, &str) -> Result<CodexTurnOutcome, String>,
) -> Result<HarnessInjectResult, HubError> {
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Codex active-session delivery requires an absolute workspace".into(),
        ));
    }
    let workspace = request
        .workspace
        .canonicalize()
        .unwrap_or_else(|_| request.workspace.clone());
    let registration = store.get_harness_session("chat", &workspace.to_string_lossy())?;
    let thread_id = registration
        .as_ref()
        .map(|row| row.disk_session_id.clone())
        .or_else(|| latest_codex_thread_id(&workspace));
    let Some(thread_id) = thread_id else {
        return Ok(unavailable(
            "no registered or on-disk Codex thread for this workspace. Register the persisted thread id in Managed harness readiness before task delivery; a separately opened Codex terminal is observed-only and cannot be written directly. Task stays queued.",
        ));
    };

    // C14.8: a manually-started Codex session never registers itself with
    // the Hub, so the first delivery here silently discovers it via the
    // on-disk fallback above and nothing durable records that discovery —
    // "Managed harness readiness" stays empty and every later delivery
    // re-scans the whole sessions/ tree from scratch. Persist it as
    // *observed* (never managed — the Hub didn't spawn this process and
    // must not claim ownership of it) so it becomes visible and the next
    // lookup hits the registration directly. Only when nothing was
    // registered at all: register_harness_session() unconditionally resets
    // mode/writer/pid on conflict, so calling it over an existing managed
    // row would silently downgrade it.
    if registration.is_none() {
        let _ =
            store.register_harness_session("chat", &workspace.to_string_lossy(), &thread_id, None);
    }

    // C14.2: a deliberately app-managed thread has one durable Hub writer.
    // Observed C12 registrations retain their conservative existing behavior;
    // no lease is claimed for somebody else's terminal/session.
    let writer_owner = format!(
        "codex-turn:{}",
        request.message_id.as_deref().unwrap_or("untracked")
    );
    let has_managed_lease = registration
        .as_ref()
        .is_some_and(|row| row.mode == HarnessSessionMode::Managed);
    if has_managed_lease {
        if let Err(error) =
            store.acquire_harness_writer("chat", &workspace.to_string_lossy(), &writer_owner)
        {
            return Ok(queued(&format!(
                "Codex managed thread {thread_id} is busy; task stays queued for retry: {error}"
            )));
        }
    }

    let sent = send_turn(&thread_id, &request.body);
    if has_managed_lease {
        let next_state = if sent.is_ok() {
            HarnessSessionState::Ready
        } else {
            HarnessSessionState::Queued
        };
        let _ = store.release_harness_writer(
            "chat",
            &workspace.to_string_lossy(),
            &writer_owner,
            next_state,
        );
    }

    match sent {
        Ok(outcome) => {
            if let Some(message_id) = request.message_id.as_deref() {
                let _ = store.set_message_status(message_id, crate::MessageStatus::Acked);
            }
            // Route Codex's captured reply back into the Hub, addressed to
            // whoever sent the task. A long-running turn that produced no
            // reply within the read budget (reply_text: None) still counts
            // as delivered — turn/start already acked the turn.
            if let Some(reply_text) = outcome.reply_text.as_deref() {
                if let Ok(reply_message) = reply::record_codex_reply(
                    store,
                    request.message_id.as_deref(),
                    request.session_id.as_deref(),
                    reply_text,
                ) {
                    // This turn's response now genuinely appears in Codex's
                    // own on-disk session transcript too (the turn/start we
                    // just ran did a thread/resume on that same thread), so
                    // the separate C12 passive transcript poller
                    // (capture_codex_session, polled independently by the
                    // desktop UI) would otherwise rediscover this exact text
                    // and post it a second time. Pre-mark it seen so that
                    // poller's own dedup check skips it.
                    let _ = store.mark_harness_capture_seen(
                        "codex",
                        CODEX_AGENT_ID,
                        request.session_id.as_deref(),
                        reply_text,
                        Some(&reply_message.id),
                    );
                }
            }
            Ok(HarnessInjectResult {
                harness: "chat".into(),
                pid: None,
                status: "delivered".into(),
                detail: format!(
                    "forwarded to Codex thread {thread_id} via app-server ({})",
                    outcome.turn_id
                ),
            })
        }
        Err(error) if error.contains("already has an active writer") => Ok(queued(&format!(
            "Codex thread {thread_id} already has an external active writer; task stays queued for retry. {error}"
        ))),
        Err(error) => Ok(unavailable(&error)),
    }
}

fn unavailable(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "chat".into(),
        pid: None,
        status: "unavailable".into(),
        detail: detail.into(),
    }
}

fn queued(detail: &str) -> HarnessInjectResult {
    HarnessInjectResult {
        harness: "chat".into(),
        pid: None,
        status: "queued".into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;
    use tempfile::tempdir;

    fn request(workspace: &Path) -> HarnessInjectRequest {
        HarnessInjectRequest {
            harness: "chat".into(),
            workspace: workspace.to_path_buf(),
            session_id: Some("hub-session".into()),
            message_id: Some("msg-1".into()),
            body: "review the adapter".into(),
            is_task: true,
            is_wake: false,
        }
    }

    fn outcome(turn_id: &str) -> CodexTurnOutcome {
        CodexTurnOutcome {
            turn_id: turn_id.into(),
            reply_text: None,
        }
    }

    #[test]
    fn missing_thread_is_unavailable_and_does_not_spawn() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result = deliver_codex_task_with(
            &store,
            &request(Path::new("/tmp/c12-codex")),
            |_thread, _body| {
                panic!("must not contact app-server without a thread");
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.pid, None);
        assert!(result.detail.contains("queued") || result.detail.contains("register"));
    }

    #[test]
    fn persisted_thread_matches_an_equivalent_canonical_workspace_path() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let root = dir.path().join("sessions");
        let transcript_dir = root.join("2026").join("08").join("13");
        std::fs::create_dir_all(&transcript_dir).unwrap();
        std::fs::write(
            transcript_dir.join("thread.jsonl"),
            format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}/.\",\"session_id\":\"thread-canonical\"}}}}\n",
                workspace.display()
            ),
        )
        .unwrap();

        assert_eq!(
            latest_codex_thread_id_from(&root, &workspace),
            Some("thread-canonical".into())
        );
    }

    /// Restores $HOME on drop even if the test body panics mid-way, so one
    /// failing test can't leave every later test in the process pointed at
    /// a deleted tempdir.
    struct HomeEnvGuard(Option<String>);
    impl Drop for HomeEnvGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(home) => std::env::set_var("HOME", home),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn discovered_thread_without_a_registration_is_auto_registered_as_observed() {
        // latest_codex_thread_id() reads $HOME/.codex/sessions directly with
        // no injection point through the public delivery path, so this is
        // the only way to exercise the real fallback+auto-register
        // end-to-end rather than only its inner *_from() helper.
        static HOME_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _lock = HOME_ENV_LOCK.lock().unwrap();
        let _guard = HomeEnvGuard(std::env::var("HOME").ok());

        let dir = tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let fake_home = dir.path().join("home");
        let transcript_dir = fake_home
            .join(".codex")
            .join("sessions")
            .join("2026")
            .join("08")
            .join("13");
        std::fs::create_dir_all(&transcript_dir).unwrap();
        std::fs::write(
            transcript_dir.join("thread.jsonl"),
            format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}\",\"session_id\":\"thread-discovered\"}}}}\n",
                workspace.canonicalize().unwrap().display()
            ),
        )
        .unwrap();
        std::env::set_var("HOME", &fake_home);

        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();

        let result = deliver_codex_task_with(&store, &request(&workspace), |thread, _body| {
            assert_eq!(thread, "thread-discovered");
            Ok(outcome("turn-1"))
        })
        .unwrap();
        assert_eq!(result.status, "delivered");

        let workspace_key = workspace
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let registration = store
            .get_harness_session("chat", &workspace_key)
            .unwrap()
            .expect("discovery must be persisted so Managed harness readiness can show it");
        assert_eq!(registration.disk_session_id, "thread-discovered");
        assert_eq!(registration.mode, HarnessSessionMode::Observed);
    }

    #[test]
    fn registered_thread_is_delivered_through_injected_app_server() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session("chat", "/tmp/c12-codex", "thread-abc", None)
            .unwrap();
        let result = deliver_codex_task_with(
            &store,
            &request(Path::new("/tmp/c12-codex")),
            |thread, body| {
                assert_eq!(thread, "thread-abc");
                assert_eq!(body, "review the adapter");
                Ok(outcome("turn-9"))
            },
        )
        .unwrap();
        assert_eq!(result.status, "delivered");
        assert_eq!(result.pid, None);
        assert!(result.detail.contains("thread-abc"));
    }

    #[test]
    fn managed_thread_with_an_existing_hub_writer_is_queued_without_app_server_io() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_managed_harness_session("chat", "/tmp/c12-codex", "thread-abc", 77)
            .unwrap();
        store
            .acquire_harness_writer("chat", "/tmp/c12-codex", "other-turn")
            .unwrap();
        let result =
            deliver_codex_task_with(&store, &request(Path::new("/tmp/c12-codex")), |_, _| {
                panic!("a second managed writer must not contact app-server")
            })
            .unwrap();
        assert_eq!(result.status, "queued");
        assert!(result.detail.contains("busy"));
    }

    #[test]
    fn external_codex_writer_error_is_queued_for_retry() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .register_harness_session("chat", "/tmp/c12-codex", "thread-abc", None)
            .unwrap();
        let result = deliver_codex_task_with(&store, &request(Path::new("/tmp/c12-codex")), |_, _| {
            Err("codex app-server error: {\"code\":-32600,\"message\":\"thread x already has an active writer\"}".into())
        })
        .unwrap();
        assert_eq!(result.status, "queued");
        assert!(result.detail.contains("external active writer"));
    }

    #[test]
    fn empty_body_and_relative_workspace_are_rejected() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let mut empty = request(Path::new("/tmp/c12-codex"));
        empty.body = "  ".into();
        assert!(deliver_codex_task_with(&store, &empty, |_, _| Ok(outcome("x"))).is_err());
        let relative = request(Path::new("relative"));
        assert!(deliver_codex_task_with(&store, &relative, |_, _| Ok(outcome("x"))).is_err());
    }

    #[test]
    fn a_captured_reply_is_routed_back_into_the_hub_addressed_to_the_sender() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("orchestrator", true).unwrap();
        let task_msg = store
            .send_message(
                "orchestrator",
                CODEX_AGENT_ID,
                crate::MessageKind::Message,
                "please review",
                None,
                None,
                Some("task-1"),
            )
            .unwrap();

        let mut req = request(Path::new("/tmp/c12-codex"));
        req.message_id = Some(task_msg.id.clone());
        req.session_id = None;
        store
            .register_harness_session("chat", "/tmp/c12-codex", "thread-abc", None)
            .unwrap();

        let result = deliver_codex_task_with(&store, &req, |_thread, _body| {
            Ok(CodexTurnOutcome {
                turn_id: "turn-42".into(),
                reply_text: Some("looks good, approved".into()),
            })
        })
        .unwrap();
        assert_eq!(result.status, "delivered");

        let replies = store.list_messages(Some("orchestrator"), None).unwrap();
        assert!(replies
            .iter()
            .any(|m| m.from_agent == CODEX_AGENT_ID && m.body == "looks good, approved"));
    }
}
