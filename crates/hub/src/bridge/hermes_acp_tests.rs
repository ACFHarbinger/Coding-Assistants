//! Hermes ACP client tests (issue #329). Frame asserts plus a hermetic
//! round-trip against a stub stdio ACP server (shell script): the stub
//! issues one `session/request_permission`, and the test proves the client
//! answers with a deny — never an approval.

use super::*;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

const STUB: &str = r#"#!/bin/sh
OUT="$1"
read -r _init
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}'
read -r _new
printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"stub-session-1"}}'
read -r _prompt
printf '%s\n' '{"jsonrpc":"2.0","id":"perm-1","method":"session/request_permission","params":{"reason":"stub tool call"}}'
read -r deny
printf '%s\n' "$deny" > "$OUT"
printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"stub-session-1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"po"}}}}'
printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"stub-session-1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"ng"}}}}'
printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}'
"#;

#[test]
fn acp_frames_use_documented_methods() {
    assert_eq!(hermes_acp_args(), vec!["acp".to_string()]);
    assert!(
        !hermes_acp_args().iter().any(|arg| arg == "--accept-hooks"),
        "strict sandbox: never auto-approve hooks"
    );
    assert_eq!(acp_initialize()["method"], "initialize");
    let new = acp_session_new(Path::new("/tmp/ws"));
    assert_eq!(new["method"], "session/new");
    assert_eq!(new["params"]["cwd"], "/tmp/ws");
    let prompt = acp_session_prompt("sess-1", "do the task");
    assert_eq!(prompt["method"], "session/prompt");
    assert_eq!(prompt["params"]["sessionId"], "sess-1");
    assert_eq!(prompt["params"]["prompt"][0]["text"], "do the task");
}

#[test]
fn permission_deny_echoes_the_request_id_and_cancels() {
    let deny = acp_permission_deny(&json!("perm-1"));
    assert_eq!(deny["id"], "perm-1");
    assert_eq!(deny["result"]["outcome"]["outcome"], "cancelled");
    let numeric = acp_permission_deny(&json!(7));
    assert_eq!(numeric["id"], 7);
    assert!(numeric["result"].get("error").is_none());
}

#[test]
fn empty_prompt_is_rejected_before_spawn() {
    let dir = tempdir().unwrap();
    let error = run_hermes_acp_prompt(Path::new("/nonexistent-binary"), dir.path(), "   ")
        .expect_err("empty prompt must fail");
    assert!(error.contains("must not be empty"), "unexpected: {error}");
}

#[test]
fn round_trip_collects_chunks_and_denies_permission() {
    let dir = tempdir().unwrap();
    let stub = dir.path().join("stub-hermes-acp.sh");
    let captured = dir.path().join("deny.json");
    std::fs::write(&stub, STUB).unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

    // The client passes "acp" as argv[1]; the capture path goes first so
    // the stub sees it as $1.
    let script = format!(
        "#!/bin/sh\nexec {} {} \"$@\"\n",
        stub.display(),
        captured.display()
    );
    let runner = dir.path().join("run-stub.sh");
    std::fs::write(&runner, script).unwrap();
    std::fs::set_permissions(&runner, std::fs::Permissions::from_mode(0o755)).unwrap();

    let (session_id, reply) =
        run_hermes_acp_prompt(&runner, dir.path(), "reply pong").expect("stub round trip");
    assert_eq!(session_id, "stub-session-1");
    assert_eq!(reply, "pong");

    let deny_raw = std::fs::read_to_string(&captured).unwrap();
    let deny: Value = serde_json::from_str(deny_raw.trim()).unwrap();
    assert_eq!(deny["id"], "perm-1", "deny must echo the request id");
    let outcome = deny["result"]["outcome"]["outcome"].as_str().unwrap_or("");
    assert_eq!(outcome, "cancelled", "never an approval");
}

#[test]
fn missing_binary_is_a_clean_error() {
    let dir = tempdir().unwrap();
    let error = run_hermes_acp_prompt(Path::new("/nonexistent-hermes-binary"), dir.path(), "hello")
        .expect_err("missing binary must fail");
    assert!(error.contains("could not start"), "unexpected: {error}");
}
