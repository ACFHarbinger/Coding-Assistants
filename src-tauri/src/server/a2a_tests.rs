//! A2A wire tests (P11b / #335): card hygiene, auth gating, method round
//! trips against a temp `HubStore`, HTTP framing, and one live loopback.

use super::*;
use hub::{AgentCard, HubStore};
use std::io::{Read, Write};
use std::net::TcpStream;
use tempfile::tempdir;

fn temp_store() -> (tempfile::TempDir, HubStore) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    (dir, store)
}

fn seed_teamed_agent(store: &HubStore, id: &str, description: &str, tags: &[&str]) {
    store.upsert_agent(id, id).unwrap();
    store
        .upsert_agent_card(
            id,
            &AgentCard {
                name: id.to_string(),
                description: description.to_string(),
                specializations: tags.iter().map(|t| t.to_string()).collect(),
                input_schema: None,
                output_format: None,
            },
        )
        .unwrap();
    store.set_team_member(id, true).unwrap();
}

#[test]
fn card_lists_only_teamed_agents_with_fixed_keys() {
    let (_dir, store) = temp_store();
    seed_teamed_agent(&store, "dev", "Writes code", &["rust"]);
    store.upsert_agent("ghost", "ghost").unwrap();
    let agents = store.list_agents().unwrap();
    let card = build_agent_card("1.0.0", &agents);
    assert_eq!(card["name"], "Coding Assistants");
    let skills = card["skills"].as_array().unwrap();
    // Fresh stores seed `human` as a team member, so the card also lists it.
    assert_eq!(skills.len(), 2);
    let dev = skills
        .iter()
        .find(|s| s["id"] == "dev")
        .expect("dev skill present");
    assert_eq!(dev["description"], "Writes code");
    assert_eq!(dev["tags"], json!(["rust"]));
    assert!(
        skills.iter().all(|s| s["id"] != "ghost"),
        "ghost is not teamed"
    );
    let top_keys: Vec<&str> = card
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for key in [
        "name",
        "description",
        "version",
        "capabilities",
        "defaultInputModes",
        "defaultOutputModes",
        "authentication",
        "skills",
    ] {
        assert!(top_keys.contains(&key), "missing card key {key}");
    }
}

#[test]
fn card_carries_no_paths() {
    let (_dir, store) = temp_store();
    seed_teamed_agent(&store, "dev", "Writes code", &["rust"]);
    let agents = store.list_agents().unwrap();
    let raw = serde_json::to_string(&build_agent_card("1.0.0", &agents)).unwrap();
    let lowered = raw.to_lowercase();
    for needle in [
        "/home/",
        "/tmp/",
        "/root/",
        "c:\\",
        "work_dir",
        "workspace",
        "hostname",
    ] {
        assert!(!lowered.contains(needle), "card leaks {needle}");
    }
}

#[test]
fn bearer_check_fails_closed() {
    let mut headers = HashMap::new();
    headers.insert("authorization".into(), "Bearer secret".into());
    assert!(!check_bearer(&headers, None));
    assert!(!check_bearer(&headers, Some("")));
    assert!(!check_bearer(&headers, Some("other")));
    assert!(check_bearer(&headers, Some("secret")));
    assert!(!check_bearer(&HashMap::new(), Some("secret")));
}

#[test]
fn task_states_map_to_a2a_states() {
    for (hub, a2a) in [
        ("pending", "submitted"),
        ("running", "working"),
        ("done", "completed"),
        ("cancelled", "canceled"),
        ("failed", "failed"),
        ("bogus", "failed"),
    ] {
        assert_eq!(a2a_task_state(hub), a2a);
    }
}

fn send_result(store: &HubStore) -> Value {
    handle_rpc(
        store,
        "message/send",
        &json!({
            "message": {"role": "user", "parts": [{"type": "text", "text": "hello remote"}]},
            "metadata": {"ca_agent": "worker"},
        }),
        &json!(1),
    )
}

#[test]
fn message_send_creates_and_advances_a_task() {
    let (_dir, store) = temp_store();
    let response = send_result(&store);
    assert!(response.get("error").is_none(), "unexpected: {response}");
    let result = &response["result"];
    assert_eq!(result["status"]["state"], "working");
    assert_eq!(result["contextId"], result["id"]);
    let fetched = handle_rpc(&store, "tasks/get", &json!({"id": result["id"]}), &json!(2));
    assert_eq!(fetched["result"]["status"]["state"], "working");
    let cancelled = handle_rpc(
        &store,
        "tasks/cancel",
        &json!({"id": result["id"]}),
        &json!(3),
    );
    assert_eq!(cancelled["result"]["status"]["state"], "canceled");
}

#[test]
fn message_send_rejects_missing_agent_and_text() {
    let (_dir, store) = temp_store();
    let no_agent = handle_rpc(
        &store,
        "message/send",
        &json!({"message": {"role": "user", "parts": [{"type": "text", "text": "hi"}]}}),
        &json!(1),
    );
    assert_eq!(no_agent["error"]["code"], -32602);
    let no_text = handle_rpc(
        &store,
        "message/send",
        &json!({"message": {"role": "user", "parts": []}, "metadata": {"ca_agent": "w"}}),
        &json!(1),
    );
    assert_eq!(no_text["error"]["code"], -32602);
    let unknown_task = handle_rpc(&store, "tasks/get", &json!({"id": "nope"}), &json!(1));
    assert_eq!(unknown_task["error"]["code"], -32001);
    let unknown_method = handle_rpc(&store, "tasks/boo", &json!({}), &json!(1));
    assert_eq!(unknown_method["error"]["code"], -32601);
}

#[test]
fn http_request_parses_method_path_headers_body() {
    let request = parse_http_request(
        "POST / HTTP/1.1\r\nContent-Type: application/json\r\nAuthorization: Bearer s3cret\r\nContent-Length: 2",
        "{}",
    )
    .unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/");
    assert_eq!(request.headers["authorization"], "Bearer s3cret");
    assert_eq!(request.body, "{}");
    assert!(parse_http_request("GARBAGE", "").is_err());
}

#[test]
fn routes_reject_unauthenticated_rpc_but_serve_the_card() {
    let (_dir, store) = temp_store();
    let card = parse_http_request(&format!("GET {CARD_PATH} HTTP/1.1\r\nHost: x"), "").unwrap();
    let reply = route_request(&store, &card, "http://127.0.0.1:8766", Some("secret"));
    assert!(reply.starts_with("HTTP/1.1 200 OK"));
    assert!(reply.contains("\"skills\""));
    let rpc = parse_http_request("POST / HTTP/1.1\r\nContent-Length: 2", "{}").unwrap();
    let denied = route_request(&store, &rpc, "http://127.0.0.1:8766", Some("secret"));
    assert!(denied.starts_with("HTTP/1.1 401 Unauthorized"));
}

#[test]
fn loopback_serves_the_card_over_a_live_socket() {
    use crate::commands::commands::tests::CA_HOME_ENV_LOCK;
    let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    std::env::set_var("CA_HOME", dir.path());
    let mut server = A2aServer::start(DEFAULT_BIND, 0).unwrap();
    let addr = server.address().to_string();
    let mut stream = TcpStream::connect(&addr).unwrap();
    let request = format!("GET {CARD_PATH} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    server.stop();
    std::env::remove_var("CA_HOME");
    assert!(raw.starts_with("HTTP/1.1 200 OK"), "unexpected: {raw}");
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("");
    let card: Value = serde_json::from_str(body).unwrap();
    assert_eq!(card["name"], "Coding Assistants");
    assert!(card["skills"].is_array());
}
