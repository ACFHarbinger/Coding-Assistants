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
    let _lock = PEER_TOKEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
                branch: None,
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
    let _lock = PEER_TOKEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
                branch: None,
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

#[test]
fn remote_result_is_screened_as_output_before_the_handoff_is_stored() {
    let _lock = PEER_TOKEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("CA_TCP_AUTH_TOKEN", "test-pairing-token");
    let peer = stub_peer_once("test-pairing-token", "peer-done with a secret");
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .add_guardrail(
            "no-secret",
            None,
            crate::guardrails::GuardrailStage::Output,
            crate::guardrails::GuardrailMode::Blocking,
            crate::guardrails::GuardrailSpec::DenyPhrases {
                phrases: vec!["secret".into()],
                case_insensitive: true,
            },
        )
        .unwrap();
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
                peer: Some(peer),
                branch: None,
            }],
        )
        .unwrap();
    let error = store
        .advance_task(&task.id, Some("lead"), None)
        .expect_err("output trip must reject the remote result");
    std::env::remove_var("CA_TCP_AUTH_TOKEN");
    assert!(
        error.to_string().contains("no-secret"),
        "unexpected: {error}"
    );
    let after = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(after.status, "pending");
    assert!(after.last_message_id.is_none());
    assert!(store.poll_messages("dev", false).unwrap().is_empty());
}

#[test]
fn failed_wake_now_member_stays_pending_with_later_window_members() {
    let _lock = PEER_TOKEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("CA_TCP_AUTH_TOKEN");
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let grouped = |agent: &str, peer: Option<String>| WorkflowStep {
        agent: agent.into(),
        role: None,
        instruction: format!("work {agent}"),
        max_retries: 0,
        parallel_group: Some("g".into()),
        peer,
        branch: None,
    };
    let task = store
        .create_task_with_parallel(
            "par",
            None,
            &[
                grouped("dev_a", None),
                grouped("dev_b", Some("127.0.0.1:1".into())),
                grouped("dev_c", None),
            ],
            2,
            true,
        )
        .unwrap();
    let error = store
        .advance_task(&task.id, Some("lead"), None)
        .expect_err("second wake_now member has no pairing token");
    assert!(
        error.to_string().contains("pairing token"),
        "unexpected: {error}"
    );
    let after = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(after.status, "running");
    assert_eq!(after.open_agents, vec!["dev_a".to_string()]);
    assert_eq!(
        after.pending_agents,
        vec!["dev_b".to_string(), "dev_c".to_string()],
        "the failed window member and everyone after it must remain pending"
    );
    assert!(store.poll_messages("dev_a", false).unwrap().len() == 1);
    assert!(store.poll_messages("dev_b", false).unwrap().is_empty());
    assert!(store.poll_messages("dev_c", false).unwrap().is_empty());
    let result = store
        .get_message(after.last_message_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(result.kind, "handoff");
    assert_ne!(after.last_message_id, after.output_watermark_id);
}

#[test]
fn refill_dispatch_is_persisted_before_the_next_slot_is_attempted() {
    let _lock = PEER_TOKEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("CA_TCP_AUTH_TOKEN");
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let grouped = |agent: &str, peer: Option<String>| WorkflowStep {
        agent: agent.into(),
        role: None,
        instruction: format!("work {agent}"),
        max_retries: 0,
        parallel_group: Some("g".into()),
        peer,
        branch: None,
    };
    let task = store
        .create_task_with_parallel(
            "par",
            None,
            &[
                grouped("dev_a", None),
                grouped("dev_b", None),
                grouped("dev_c", None),
                grouped("dev_d", Some("127.0.0.1:1".into())),
            ],
            2,
            true,
        )
        .unwrap();
    let started = store.advance_task(&task.id, Some("lead"), None).unwrap();
    assert_eq!(started.open_agents, vec!["dev_a", "dev_b"]);
    assert_eq!(started.pending_agents, vec!["dev_c", "dev_d"]);
    let watermark = started.output_watermark_id.clone();

    let after = store
        .complete_parallel_member(&task.id, "dev_a", None)
        .unwrap();
    assert_eq!(after.output_watermark_id, watermark);
    assert_eq!(
        after.open_agents,
        vec!["dev_b".to_string(), "dev_c".to_string()]
    );
    assert_eq!(after.pending_agents, vec!["dev_d".to_string()]);
    assert_eq!(store.poll_messages("dev_c", false).unwrap().len(), 1);
    let result = store
        .get_message(after.last_message_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(result.kind, "handoff");
    assert_eq!(result.to_agent, "dev_c");
}

#[test]
fn failed_refill_keeps_the_already_persisted_completion() {
    let _lock = PEER_TOKEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("CA_TCP_AUTH_TOKEN");
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let grouped = |agent: &str, peer: Option<String>| WorkflowStep {
        agent: agent.into(),
        role: None,
        instruction: format!("work {agent}"),
        max_retries: 0,
        parallel_group: Some("g".into()),
        peer,
        branch: None,
    };
    let task = store
        .create_task_with_parallel(
            "par",
            None,
            &[
                grouped("dev_a", None),
                grouped("dev_b", None),
                grouped("dev_c", Some("127.0.0.1:1".into())),
            ],
            2,
            true,
        )
        .unwrap();
    let started = store.advance_task(&task.id, Some("lead"), None).unwrap();
    let watermark = started.output_watermark_id.clone();
    let error = store
        .complete_parallel_member(&task.id, "dev_a", None)
        .expect_err("queued remote refill has no pairing token");
    assert!(
        error.to_string().contains("pairing token"),
        "unexpected: {error}"
    );
    let after = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(after.output_watermark_id, watermark);
    assert_eq!(after.open_agents, vec!["dev_b".to_string()]);
    assert_eq!(after.pending_agents, vec!["dev_c".to_string()]);
    assert!(!after.open_agents.iter().any(|a| a == "dev_a"));
}
