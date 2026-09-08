use super::*;
use crate::HubStore;
use tempfile::tempdir;

#[test]
fn parse_cursor_stream_json_line() {
    let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Done"}]},"session_id":"chat-123"}"#;
    let parsed = parse_cursor_stream_line(line).unwrap();
    assert_eq!(parsed.session_id.as_deref(), Some("chat-123"));
    assert_eq!(parsed.assistant_texts, vec!["Done".to_string()]);
}

#[test]
fn unmanaged_cursor_delivery_returns_unavailable() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let result = deliver_cursor_task(
        &store,
        &HarnessInjectRequest {
            harness: "cursor".into(),
            workspace: dir.path().to_path_buf(),
            session_id: None,
            message_id: Some("msg-1".into()),
            body: "hello cursor".into(),
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
fn managed_cursor_delivery_acquires_and_releases_writer_lease() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path();
    let ws_str = workspace.to_string_lossy().into_owned();

    store
        .register_managed_harness_session("cursor", &ws_str, "chat-owned-1", 1234)
        .unwrap();

    let req = HarnessInjectRequest {
        harness: "cursor".into(),
        workspace: workspace.to_path_buf(),
        session_id: None,
        message_id: Some("msg-test-1".into()),
        body: "build worker component".into(),
        is_task: true,
        is_wake: false,
        ..Default::default()
    };

    let result = deliver_cursor_task_with(&store, &req, |_ws, _prompt, chat_id, _model| {
        assert_eq!(chat_id, Some("chat-owned-1"));
        Ok((
            Some(1234),
            CursorStreamOutput {
                session_id: Some("chat-owned-1".into()),
                assistant_texts: vec!["Done".into()],
            },
        ))
    })
    .unwrap();

    assert_eq!(result.status, "ok");
    assert_eq!(result.pid, None);

    let sess = store
        .get_harness_session("cursor", &ws_str)
        .unwrap()
        .unwrap();
    assert_eq!(sess.state, HarnessSessionState::Ready);
    assert!(sess.writer_owner.is_none());
}

#[test]
fn managed_cursor_delivery_without_persisted_chat_id_is_unavailable() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path();
    let ws_str = workspace.to_string_lossy().into_owned();
    store
        .register_managed_harness_session("cursor", &ws_str, "managed-fresh", 1234)
        .unwrap();

    let result = deliver_cursor_task(
        &store,
        &HarnessInjectRequest {
            harness: "cursor".into(),
            workspace: workspace.to_path_buf(),
            session_id: None,
            message_id: Some("msg-1".into()),
            body: "hello".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.status, "unavailable");
    assert!(result.detail.contains("persisted chat id"));
}

#[test]
fn persisted_chat_id_rejects_managed_placeholder() {
    let registration = crate::HarnessSessionRegistration {
        harness: "cursor".into(),
        workspace: "/tmp/ws".into(),
        disk_session_id: "managed-abc".into(),
        leader_socket: None,
        registered_at: "0".into(),
        mode: crate::HarnessSessionMode::Managed,
        state: crate::HarnessSessionState::Queued,
        managed_pid: None,
        writer_owner: None,
        writer_acquired_at: None,
    };
    assert!(persisted_cursor_chat_id(Some(&registration)).is_none());
}

#[test]
fn delivery_ignores_hub_session_routing_metadata_for_resume() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path();
    let ws_str = workspace.to_string_lossy().into_owned();
    store
        .register_managed_harness_session("cursor", &ws_str, "cursor-chat-real", 1234)
        .unwrap();

    let req = HarnessInjectRequest {
        harness: "cursor".into(),
        workspace: workspace.to_path_buf(),
        session_id: Some("hub-work-session-routing-id".into()),
        message_id: Some("msg-test-2".into()),
        body: "continue task".into(),
        is_task: true,
        is_wake: false,
        ..Default::default()
    };

    let result = deliver_cursor_task_with(&store, &req, |_ws, _prompt, chat_id, _model| {
        assert_eq!(chat_id, Some("cursor-chat-real"));
        Ok((
            None,
            CursorStreamOutput {
                session_id: Some("cursor-chat-real".into()),
                assistant_texts: vec![],
            },
        ))
    })
    .unwrap();
    assert_eq!(result.status, "ok");
}

#[test]
fn managed_start_persists_stream_session_id_as_disk_session_id() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    let ws_str = workspace.to_string_lossy().into_owned();

    let (started, registration) = start_cursor_managed_harness_with(
        &store,
        &workspace,
        "Coding-Assistants managed session",
        |_ws, _prompt, _chat_id, _model| {
            Ok((
                None,
                CursorStreamOutput {
                    session_id: Some("stream-chat-42".into()),
                    assistant_texts: vec![],
                },
            ))
        },
    )
    .unwrap();

    assert_eq!(started.status, "started");
    assert_eq!(registration.disk_session_id, "stream-chat-42");
    let row = store
        .get_harness_session("cursor", &ws_str)
        .unwrap()
        .unwrap();
    assert_eq!(row.disk_session_id, "stream-chat-42");
    assert!(row.writer_owner.is_none());
    assert_eq!(row.managed_pid, None);
}

#[test]
fn managed_start_missing_stream_session_id_releases_writer_lease() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    let ws_str = workspace.to_string_lossy().into_owned();

    let error = start_cursor_managed_harness_with(
        &store,
        &workspace,
        "Coding-Assistants managed session",
        |_ws, _prompt, _chat_id, _model| Ok((None, CursorStreamOutput::default())),
    )
    .unwrap_err();

    assert!(error.contains("did not include a session_id"));
    let row = store
        .get_harness_session("cursor", &ws_str)
        .unwrap()
        .unwrap();
    assert!(row.writer_owner.is_none());
    assert_eq!(row.state, HarnessSessionState::Queued);
}

#[test]
fn managed_start_replaces_an_observed_workspace_registration() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    let ws_str = workspace.to_string_lossy().into_owned();
    store
        .register_harness_session("cursor", &ws_str, "observed-chat", None)
        .unwrap();

    let (_, registration) = start_cursor_managed_harness_with(
        &store,
        &workspace,
        "Start a Hub-owned chat",
        |_ws, _prompt, _chat_id, _model| {
            Ok((
                None,
                CursorStreamOutput {
                    session_id: Some("managed-chat".into()),
                    assistant_texts: vec![],
                },
            ))
        },
    )
    .unwrap();

    assert_eq!(registration.mode, HarnessSessionMode::Managed);
    assert_eq!(registration.disk_session_id, "managed-chat");
    assert!(registration.writer_owner.is_none());
}

#[test]
fn managed_restart_failure_clears_the_stale_chat_id() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    let ws_str = workspace.to_string_lossy().into_owned();
    store
        .register_managed_harness_session("cursor", &ws_str, "old-chat", 1234)
        .unwrap();

    let error = start_cursor_managed_harness_with(
        &store,
        &workspace,
        "Start a fresh chat",
        |_ws, _prompt, _chat_id, _model| Err("worker failed".into()),
    )
    .unwrap_err();

    assert!(error.contains("worker failed"));
    let registration = store
        .get_harness_session("cursor", &ws_str)
        .unwrap()
        .unwrap();
    assert_eq!(registration.disk_session_id, "pending");
    assert_eq!(registration.state, HarnessSessionState::Queued);
    assert!(registration.writer_owner.is_none());
}

#[test]
fn managed_start_uses_the_canonical_workspace_registration_key() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let nested = dir.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    let noncanonical = nested.join("..");
    let canonical = noncanonical.canonicalize().unwrap();

    let (_, registration) = start_cursor_managed_harness_with(
        &store,
        &noncanonical,
        "Start a managed chat",
        |_ws, _prompt, _chat_id, _model| {
            Ok((
                None,
                CursorStreamOutput {
                    session_id: Some("canonical-chat".into()),
                    assistant_texts: vec![],
                },
            ))
        },
    )
    .unwrap();

    assert_eq!(registration.workspace, canonical.to_string_lossy());
    assert!(store
        .get_harness_session("cursor", &noncanonical.to_string_lossy())
        .unwrap()
        .is_none());
}

#[test]
fn delivery_surfaces_writer_release_failure() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let workspace = dir.path().canonicalize().unwrap();
    let ws_str = workspace.to_string_lossy().into_owned();
    store
        .register_managed_harness_session("cursor", &ws_str, "chat", 1234)
        .unwrap();
    let request = HarnessInjectRequest {
        harness: "cursor".into(),
        workspace,
        message_id: Some("release".into()),
        body: "continue".into(),
        is_task: true,
        ..Default::default()
    };

    let error = deliver_cursor_task_with(&store, &request, |_ws, _prompt, _chat, _model| {
        store
            .release_harness_writer(
                "cursor",
                &ws_str,
                "cursor-worker:release",
                HarnessSessionState::Ready,
            )
            .unwrap();
        Ok((None, CursorStreamOutput::default()))
    })
    .unwrap_err();

    assert!(error.to_string().contains("lease is not held"));
}

#[test]
fn latest_cursor_session_id_finds_the_newest_transcript_dir() {
    let dir = tempdir().unwrap();
    let transcripts = dir.path();
    for (session, marker) in [("older-chat", "a"), ("newer-chat", "b")] {
        let session_dir = transcripts.join(session);
        fs::create_dir_all(&session_dir).unwrap();
        fs::write(session_dir.join(format!("{session}.jsonl")), marker).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        latest_cursor_session_id_from(transcripts).as_deref(),
        Some("newer-chat")
    );
}
