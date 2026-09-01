//! Grok bridge tests (split out of bridge/grok.rs for the 500-LoC
//! cap, #158). A child module, so it globs grok's private items.

use super::*;
use tempfile::tempdir;

#[test]
fn encode_matches_grok_session_folder_names() {
    assert_eq!(
        encode_workspace_dir_name(Path::new(
            "/home/pkhunter/Repositories/Repos/Coding-Assistants"
        )),
        "%2Fhome%2Fpkhunter%2FRepositories%2FRepos%2FCoding-Assistants"
    );
}

#[test]
fn parse_active_sessions_reads_the_live_tui_record() {
    let raw =
        r#"[{"session_id":"019ffa19-d2c4-7452-9f51-66623841870a","pid":690024,"cwd":"/tmp/ws"}]"#;
    let rows = parse_active_grok_sessions(raw);
    assert_eq!(rows[0].session_id, "019ffa19-d2c4-7452-9f51-66623841870a");
    assert_eq!(rows[0].pid, 690024);
}

#[test]
fn latest_grok_session_prefers_the_live_active_tui_over_disk_history() {
    // #165: "Resume in terminal" must continue the conversation the
    // user is actually looking at. The live TUI's session id (from
    // active_sessions.json) wins over any on-disk chat_history.jsonl,
    // which can lag behind what the live TUI is writing.
    static GROK_HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _lock = GROK_HOME_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    std::env::set_var("GROK_HOME", dir.path());

    std::fs::write(
        grok_home().join("active_sessions.json"),
        format!(
            r#"[{{"session_id":"live-tui-1","pid":4242,"cwd":"{}"}}]"#,
            workspace.display()
        ),
    )
    .unwrap();
    // An on-disk session directory that would otherwise be "latest".
    let old = grok_home()
        .join("sessions")
        .join(encode_workspace_dir_name(&workspace))
        .join("disk-old");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("chat_history.jsonl"), "x").unwrap();

    assert_eq!(
        latest_grok_session_id(&workspace).as_deref(),
        Some("live-tui-1")
    );
    std::env::remove_var("GROK_HOME");
}

#[test]
fn acp_frames_use_documented_methods() {
    assert_eq!(acp_initialize()["method"], "initialize");
    let load = acp_session_load("sess-1", Path::new("/tmp/ws"));
    assert_eq!(load["method"], "session/load");
    let prompt = acp_session_prompt("sess-1", "do the task");
    assert_eq!(prompt["params"]["prompt"][0]["text"], "do the task");
    assert_eq!(
        grok_acp_client_args(Path::new("/tmp/leader.sock")),
        vec![
            "agent",
            "--leader",
            "--leader-socket",
            "/tmp/leader.sock",
            "stdio"
        ]
    );
}

#[test]
fn missing_leader_is_unavailable_and_does_not_spawn_a_tui() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .register_harness_session(
            "grok",
            "/tmp/ca-grok-bridge",
            "019ff7ff-69a1-70e0-9d50-8e4544861f12",
            Some("/tmp/missing-ca-grok-leader.sock"),
        )
        .unwrap();
    let result = deliver_grok_task(
        &store,
        &HarnessInjectRequest {
            harness: "grok".into(),
            workspace: PathBuf::from("/tmp/ca-grok-bridge"),
            session_id: None,
            message_id: Some("msg-1".into()),
            body: "review the hub".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.status, "unavailable");
    assert!(result.detail.contains("leader socket") || result.detail.contains("--leader"));
    assert_eq!(result.pid, None);
}

#[test]
fn registration_is_required_or_inferred_from_disk() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let result = deliver_grok_task(
        &store,
        &HarnessInjectRequest {
            harness: "grok".into(),
            workspace: PathBuf::from("/tmp/no-such-ca-workspace-xyz"),
            session_id: None,
            message_id: None,
            body: "hello".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.status, "unavailable");
    assert!(
        result.detail.contains("no registered")
            || result.detail.contains("leader socket")
            || result.detail.contains("--leader")
    );
}
