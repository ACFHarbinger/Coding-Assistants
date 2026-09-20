//! Remote workflow-step dispatch tests (P11a / #334). A stub TCP peer plays
//! the server side (Authenticate → StartTask → TaskComplete) so `advance_task`
//! proves the remote transcript shape hermetically.

use super::super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use tempfile::tempdir;

/// Serializes the `CA_TCP_AUTH_TOKEN` override below; the key is process
/// global and this is its only test user.
static PEER_TOKEN_LOCK: Mutex<()> = Mutex::new(());

fn stub_peer_once(expected_token: &str, result: &str) -> String {
    let expected = expected_token.to_string();
    let result = result.to_string();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let auth: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(auth["type"], "Authenticate");
        assert_eq!(auth["token"], expected.as_str());
        writeln!(
            writer,
            r#"{{"type":"Status","running":true,"message":"Authenticated"}}"#
        )
        .unwrap();
        writer.flush().unwrap();
        line.clear();
        reader.read_line(&mut line).unwrap();
        let start: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(start["type"], "StartTask");
        writeln!(writer, r#"{{"type":"TaskComplete","result":"{result}"}}"#).unwrap();
        writer.flush().unwrap();
    });
    addr
}

#[test]
fn remote_step_records_result_as_handoff_without_local_wake() {
    let _lock = PEER_TOKEN_LOCK.lock().unwrap();
    std::env::set_var("CA_TCP_AUTH_TOKEN", "test-pairing-token");
    let peer = stub_peer_once("test-pairing-token", "peer-done");
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task(
            "remote delegation",
            None,
            &[WorkflowStep {
                agent: "dev".into(),
                role: None,
                instruction: "do it remotely".into(),
                max_retries: 0,
                parallel_group: None,
                peer: Some(peer.clone()),
            }],
        )
        .unwrap();
    let advanced = store.advance_task(&task.id, Some("lead"), None).unwrap();
    std::env::remove_var("CA_TCP_AUTH_TOKEN");

    let message_id = advanced
        .last_message_id
        .expect("remote dispatch writes a message");
    let message = store.get_message(&message_id).unwrap().unwrap();
    assert_eq!(message.from_agent, "lead");
    assert_eq!(message.to_agent, "dev");
    assert_eq!(message.kind, "handoff");
    assert!(
        message.body.contains("do it remotely"),
        "missing instruction"
    );
    assert!(message.body.contains("peer-done"), "missing remote result");
    assert!(message.body.contains(&peer), "missing peer attribution");
    assert!(
        store.list_wakes(Some("dev"), true).unwrap().is_empty(),
        "remote work must not raise a local wake"
    );
}

#[test]
fn remote_step_without_pairing_token_fails_closed() {
    let _lock = PEER_TOKEN_LOCK.lock().unwrap();
    std::env::remove_var("CA_TCP_AUTH_TOKEN");
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task(
            "remote delegation",
            None,
            &[WorkflowStep {
                agent: "dev".into(),
                role: None,
                instruction: "do it remotely".into(),
                max_retries: 0,
                parallel_group: None,
                peer: Some("127.0.0.1:1".into()),
            }],
        )
        .unwrap();
    let error = store
        .advance_task(&task.id, Some("lead"), None)
        .expect_err("must fail");
    assert!(
        error.to_string().contains("pairing token"),
        "unexpected: {error}"
    );
}
