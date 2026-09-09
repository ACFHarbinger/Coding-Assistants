use super::*;
use crate::HubStore;
use tempfile::tempdir;

/// Restores $HOME on drop even if the test body panics mid-way.
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
fn unmanaged_gemini_delivery_returns_unavailable() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let result = deliver_gemini_task(
        &store,
        &HarnessInjectRequest {
            harness: "gemini".into(),
            workspace: dir.path().to_path_buf(),
            session_id: None,
            message_id: Some("msg-1".into()),
            body: "hello gemini".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.status, "unavailable");
    assert!(result.detail.contains("managed session"));
}

#[test]
fn latest_gemini_session_finds_an_interactive_tui_conversation_dir() {
    // #165: an interactive agy TUI conversation (brain/<uuid>/conversation/)
    // must be discoverable for "Resume in terminal", not just the
    // managed worker's transcript.jsonl layout.
    static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _lock = HOME_LOCK.lock().unwrap();
    let _guard = HomeEnvGuard(std::env::var("HOME").ok());
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    let conversation = gemini_brain_dir().join("conv-tui-1").join("conversation");
    std::fs::create_dir_all(&conversation).unwrap();
    std::fs::write(conversation.join("data.json"), "{}").unwrap();
    assert_eq!(
        latest_gemini_session_id(Path::new("/unused")).as_deref(),
        Some("conv-tui-1")
    );
}

#[test]
fn start_managed_registers_the_id_agy_reports_and_errors_without_one() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let ws = dir.path();
    let ws_str = ws.to_string_lossy().into_owned();

    let (start, reg) = start_gemini_managed_harness_with(&store, ws, "kick off", |_w, p, conv| {
        assert_eq!(p, "kick off");
        assert_eq!(conv, None, "a fresh start passes no --conversation");
        Ok((
            Some(4321),
            AgyStreamOutput {
                conversation_id: Some("conv-fresh-1".into()),
                assistant_texts: vec![],
            },
        ))
    })
    .unwrap();
    assert_eq!(start.status, "started");
    assert_eq!(reg.disk_session_id, "conv-fresh-1");
    assert_eq!(reg.mode, HarnessSessionMode::Managed);
    assert_eq!(reg.state, HarnessSessionState::Ready);

    let stored = store
        .get_harness_session("gemini", &ws_str)
        .unwrap()
        .unwrap();
    assert_eq!(stored.disk_session_id, "conv-fresh-1");
    assert_eq!(stored.managed_pid, Some(4321));

    let err = start_gemini_managed_harness_with(&store, ws, "again", |_w, _p, _c| {
        Ok((Some(1), AgyStreamOutput::default()))
    })
    .unwrap_err();
    assert!(err.contains("no conversation id"), "{err}");
}

#[test]
fn parse_agy_stream_json_line() {
    let line = r#"{"source":"MODEL","type":"PLANNER_RESPONSE","content":"Analyzed codebase","conversation_id":"conv-123"}"#;
    let parsed = parse_agy_stream_line(line).unwrap();
    assert_eq!(parsed.conversation_id.as_deref(), Some("conv-123"));
    assert_eq!(
        parsed.assistant_texts,
        vec!["Analyzed codebase".to_string()]
    );
}

#[test]
fn managed_gemini_delivery_acquires_and_releases_writer_lease() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path();
    let ws_str = workspace.to_string_lossy().into_owned();

    store
        .register_managed_harness_session("gemini", &ws_str, "conv-owned-1", 1234)
        .unwrap();

    let req = HarnessInjectRequest {
        harness: "gemini".into(),
        workspace: workspace.to_path_buf(),
        session_id: None,
        message_id: Some("msg-test-1".into()),
        body: "build worker component".into(),
        is_task: true,
        is_wake: false,
        ..Default::default()
    };

    let result = deliver_gemini_task_with(&store, &req, |_ws, _prompt, conv_id| {
        assert_eq!(conv_id, Some("conv-owned-1"));
        Ok((
            Some(1234),
            AgyStreamOutput {
                conversation_id: Some("conv-owned-1".into()),
                assistant_texts: vec!["Done".into()],
            },
        ))
    })
    .unwrap();

    assert_eq!(result.status, "ok");
    assert_eq!(result.pid, Some(1234));

    let sess = store
        .get_harness_session("gemini", &ws_str)
        .unwrap()
        .unwrap();
    assert_eq!(sess.state, HarnessSessionState::Ready);
    assert!(sess.writer_owner.is_none());
}

#[test]
fn managed_gemini_delivery_returns_queued_when_writer_is_busy() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path();
    let ws_str = workspace.to_string_lossy().into_owned();

    store
        .register_managed_harness_session("gemini", &ws_str, "conv-owned-1", 1234)
        .unwrap();

    store
        .acquire_harness_writer("gemini", &ws_str, "existing-writer")
        .unwrap();

    let req = HarnessInjectRequest {
        harness: "gemini".into(),
        workspace: workspace.to_path_buf(),
        session_id: None,
        message_id: Some("msg-test-2".into()),
        body: "second task".into(),
        is_task: true,
        is_wake: false,
        ..Default::default()
    };

    let result = deliver_gemini_task_with(&store, &req, |_ws, _prompt, _conv| {
        panic!("runner should not execute when busy");
    })
    .unwrap();

    assert_eq!(result.status, "queued");
    assert!(result.detail.contains("busy"));
}

#[test]
fn relative_workspace_is_rejected() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let result = deliver_gemini_task(
        &store,
        &HarnessInjectRequest {
            harness: "gemini".into(),
            workspace: PathBuf::from("relative-workspace"),
            session_id: None,
            message_id: None,
            body: "hello".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        },
    );
    assert!(result.is_err());
}
