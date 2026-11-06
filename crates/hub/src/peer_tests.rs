//! Peer delegation client tests (P11a / #334). A stub TCP peer speaks the
//! exact documented framing (one JSON line per request) so the client loop,
//! auth gating, and terminal-frame handling are covered hermetically.

use super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc::channel;
use std::thread;

fn read_line(reader: &mut BufReader<TcpStream>) -> String {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    line.trim().to_string()
}

fn write_line(stream: &mut TcpStream, value: &Value) {
    writeln!(stream, "{value}").unwrap();
    stream.flush().unwrap();
}

/// Spawn a scripted stub peer. `script` maps each received request line to
/// the response lines to send back. Returns the `host:port` to dial.
fn stub_peer(script: Vec<(String, Vec<Value>)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let (ready_tx, ready_rx) = channel::<()>();
    thread::spawn(move || {
        ready_tx.send(()).unwrap();
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        for (want_type, replies) in &script {
            let line = read_line(&mut reader);
            let request: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(
                request.get("type").and_then(|t| t.as_str()),
                Some(want_type.as_str())
            );
            for reply in replies {
                write_line(&mut writer, reply);
            }
        }
    });
    ready_rx.recv().unwrap();
    addr
}

#[test]
fn frames_match_the_tcp_protocol() {
    let auth = authenticate_frame("secret");
    assert_eq!(auth["type"], "Authenticate");
    assert_eq!(auth["token"], "secret");
    let start = start_task_frame("dev", "do it");
    assert_eq!(start["type"], "StartTask");
    assert_eq!(start["task"], "do it");
    assert_eq!(start["config"]["roles"][0]["name"], "dev");
    let model = &start["config"]["roles"][0]["config"];
    for key in [
        "provider",
        "model",
        "endpoint",
        "prompt_file",
        "rule_file",
        "workflow_file",
    ] {
        assert!(model.get(key).is_some(), "missing model key {key}");
    }
}

#[test]
fn round_trip_skips_broadcasts_and_returns_the_result() {
    let peer = stub_peer(vec![
        (
            "Authenticate".into(),
            vec![json!({"type": "Status", "running": true, "message": "Authenticated"})],
        ),
        (
            "StartTask".into(),
            vec![
                json!({"type": "TaskEvent", "source": "dev", "event_type": "thought", "content": "on it"}),
                json!({"type": "TaskComplete", "result": "remote says hi"}),
            ],
        ),
    ]);
    let result = delegate_remote_step(&peer, "dev", "do it", "pairing-token").unwrap();
    assert_eq!(result, "remote says hi");
}

#[test]
fn rejected_auth_fails_with_the_peer_message() {
    let peer = stub_peer(vec![(
        "Authenticate".into(),
        vec![json!({"type": "Error", "message": "authentication required"})],
    )]);
    let error = delegate_remote_step(&peer, "dev", "do it", "wrong").expect_err("must fail");
    assert!(error.contains("authentication"), "unexpected: {error}");
}

#[test]
fn peer_error_surfaces_as_failure() {
    let peer = stub_peer(vec![
        (
            "Authenticate".into(),
            vec![json!({"type": "Status", "running": true, "message": "Authenticated"})],
        ),
        (
            "StartTask".into(),
            vec![json!({"type": "Error", "message": "no runners"})],
        ),
    ]);
    let error =
        delegate_remote_step(&peer, "dev", "do it", "pairing-token").expect_err("must fail");
    assert!(error.contains("no runners"), "unexpected: {error}");
}

#[test]
fn empty_token_fails_closed_before_connecting() {
    let error = delegate_remote_step("127.0.0.1:1", "dev", "do it", "   ").expect_err("must fail");
    assert!(error.contains("auth token"), "unexpected: {error}");
}

#[test]
fn validation_rejects_empty_fields() {
    assert!(delegate_remote_step("127.0.0.1:1", "", "do it", "t").is_err());
    assert!(delegate_remote_step("127.0.0.1:1", "dev", "  ", "t").is_err());
    assert!(delegate_remote_step("not-a-socket-addr", "dev", "do it", "t").is_err());
}
