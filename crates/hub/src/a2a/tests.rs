//! A2A push-notification tests (issue #364): URL validation, config store
//! CRUD, the four JSON-RPC verbs in both naming schemes, vault hygiene for
//! webhook credentials, and a live loopback end-to-end (register a config,
//! transition the task, assert the POST hits a local test listener).

use super::notifier::{deliver, status_event, tick, PushNotifier};
use super::push::{update_store, validate_callback_url, vault_key_for, PushConfigStore};
use super::{a2a_task_state, handle_push_rpc, is_push_method};
use crate::{HubStore, WorkflowStep};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;
use tempfile::{tempdir, TempDir};

pub(super) fn temp_store() -> (TempDir, HubStore) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    (dir, store)
}

pub(super) fn running_task(store: &HubStore) -> crate::TaskRecord {
    store
        .set_wake_policy(&crate::WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: true,
        })
        .unwrap();
    let steps = vec![WorkflowStep {
        agent: "worker".into(),
        role: None,
        instruction: "do the thing".into(),
        max_retries: 0,
        parallel_group: None,
        peer: None,
        branch: None,
        sub_task_id: None,
        handoff_metadata: None,
        context_filter: None,
    }];
    let task = store
        .create_task_with_parallel("push test", None, &steps, 4, false)
        .unwrap();
    store.advance_task(&task.id, Some("worker"), None).unwrap()
}

fn rpc(store: &HubStore, method: &str, params: Value, id: i64) -> Value {
    handle_push_rpc(store, method, &params, &json!(id)).expect("push method handled")
}

pub(super) fn set_rpc(store: &HubStore, task_id: &str, url: &str) -> Value {
    rpc(
        store,
        "tasks/pushNotificationConfig/set",
        json!({
            "taskId": task_id,
            "pushNotificationConfig": {"url": url, "token": "echo-1"},
        }),
        1,
    )
}

/// One-shot loopback webhook: captures `expected` requests (head + body)
/// into the channel, replying `200 OK` to each.
fn webhook_sink(expected: usize) -> (String, Receiver<(String, String)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/hook", listener.local_addr().unwrap());
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for _ in 0..expected {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut raw = Vec::new();
            let mut byte = [0u8; 1];
            loop {
                if stream.read(&mut byte).unwrap_or(0) == 0 {
                    return;
                }
                raw.push(byte[0]);
                if raw.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let head = String::from_utf8_lossy(&raw).to_string();
            let length = head
                .split("\r\n")
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            let mut body = vec![0u8; length];
            if stream.read_exact(&mut body).is_err() {
                return;
            }
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            let _ = tx.send((head, String::from_utf8_lossy(&body).to_string()));
        }
    });
    (url, rx)
}

#[test]
fn callback_url_validation() {
    assert!(validate_callback_url("http://127.0.0.1:9000/hook").is_ok());
    assert!(validate_callback_url("https://agent.example.com/a2a").is_ok());
    for bad in [
        "",
        "file:///etc/passwd",
        "gopher://x/",
        "http://",
        "http://user:pass@127.0.0.1:9/",
        "not a uri",
    ] {
        assert!(validate_callback_url(bad).is_err(), "accepted {bad:?}");
    }
    let too_long = format!("http://h/{}", "x".repeat(3000));
    assert!(validate_callback_url(&too_long).is_err());
}

#[test]
fn config_store_crud_round_trip() {
    let (dir, store) = temp_store();
    let task = running_task(&store);
    update_store(dir.path(), |configs| {
        assert_eq!(configs.count_for_task(&task.id), 0);
        configs.upsert(super::push::PushConfigEntry {
            task_id: task.id.clone(),
            config_id: "c1".into(),
            url: "http://127.0.0.1:1/".into(),
            token: None,
            auth_schemes: vec![],
            has_credentials: false,
            enabled: true,
            created_at: "now".into(),
            last_notified_state: None,
        });
        Ok(())
    })
    .unwrap();
    let reopened = PushConfigStore::open(dir.path()).unwrap();
    assert_eq!(reopened.count_for_task(&task.id), 1);
    assert_eq!(
        reopened.find(&task.id, "c1").unwrap().url,
        "http://127.0.0.1:1/"
    );
    update_store(dir.path(), |configs| {
        assert!(configs.remove(&task.id, "c1").is_some());
        assert!(configs.remove(&task.id, "c1").is_none());
        Ok(())
    })
    .unwrap();
    assert!(PushConfigStore::open(dir.path())
        .unwrap()
        .configs()
        .is_empty());
}

#[test]
fn set_get_list_delete_round_trip() {
    let (_dir, store) = temp_store();
    let task = running_task(&store);
    let set = set_rpc(&store, &task.id, "http://127.0.0.1:9100/hook");
    assert!(set.get("error").is_none(), "unexpected: {set}");
    let config = &set["result"]["pushNotificationConfig"];
    assert_eq!(config["url"], "http://127.0.0.1:9100/hook");
    assert_eq!(config["token"], "echo-1");
    let config_id = config["id"].as_str().unwrap().to_string();
    // Baseline pinned to the state at registration.
    let registry = PushConfigStore::open(store.data_dir()).unwrap();
    assert_eq!(
        registry
            .find(&task.id, &config_id)
            .unwrap()
            .last_notified_state
            .as_deref(),
        Some("working")
    );

    let get = rpc(
        &store,
        "tasks/pushNotificationConfig/get",
        json!({"id": task.id}),
        2,
    );
    assert_eq!(
        get["result"]["pushNotificationConfig"]["id"],
        json!(config_id)
    );

    let list = rpc(
        &store,
        "tasks/pushNotificationConfig/list",
        json!({"taskId": task.id}),
        3,
    );
    assert_eq!(list["result"].as_array().unwrap().len(), 1);

    let delete = rpc(
        &store,
        "tasks/pushNotificationConfig/delete",
        json!({"id": task.id, "pushNotificationConfigId": config_id}),
        4,
    );
    assert_eq!(delete["result"], Value::Null);
    // v1.0: delete is idempotent.
    let again = rpc(
        &store,
        "tasks/pushNotificationConfig/delete",
        json!({"id": task.id, "pushNotificationConfigId": "gone"}),
        5,
    );
    assert_eq!(again["result"], Value::Null);
    let missing = rpc(
        &store,
        "tasks/pushNotificationConfig/get",
        json!({"id": task.id}),
        6,
    );
    assert_eq!(missing["error"]["code"], -32001);
}

#[test]
fn set_validates_task_url_and_params() {
    let (_dir, store) = temp_store();
    let task = running_task(&store);
    let no_task = set_rpc(&store, "no-such-task", "http://127.0.0.1:1/");
    assert_eq!(no_task["error"]["code"], -32001);
    let bad_url = set_rpc(&store, &task.id, "file:///etc/passwd");
    assert_eq!(bad_url["error"]["code"], -32602);
    let no_url = rpc(
        &store,
        "tasks/pushNotificationConfig/set",
        json!({"taskId": task.id, "pushNotificationConfig": {}}),
        1,
    );
    assert_eq!(no_url["error"]["code"], -32602);
    let no_task_id = rpc(
        &store,
        "tasks/pushNotificationConfig/set",
        json!({"pushNotificationConfig": {"url": "http://127.0.0.1:1/"}}),
        1,
    );
    assert_eq!(no_task_id["error"]["code"], -32602);
    let list_unknown = rpc(
        &store,
        "tasks/pushNotificationConfig/list",
        json!({"taskId": "nope"}),
        1,
    );
    assert_eq!(list_unknown["error"]["code"], -32001);
    assert!(is_push_method("tasks/pushNotificationConfig/set"));
    assert!(is_push_method("DeleteTaskPushNotificationConfig"));
    assert!(!is_push_method("tasks/get"));
}

#[test]
fn v1_pascal_case_alias_and_resource_params() {
    let (_dir, store) = temp_store();
    let task = running_task(&store);
    let set = rpc(
        &store,
        "CreateTaskPushNotificationConfig",
        json!({
            "parent": format!("tasks/{}", task.id),
            "config": {"url": "http://127.0.0.1:9101/hook"},
        }),
        1,
    );
    assert!(set.get("error").is_none(), "unexpected: {set}");
    let config_id = set["result"]["pushNotificationConfig"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let get = rpc(
        &store,
        "GetTaskPushNotificationConfig",
        json!({"name": format!("tasks/{}/pushNotificationConfigs/{}", task.id, config_id)}),
        2,
    );
    assert_eq!(
        get["result"]["pushNotificationConfig"]["id"],
        json!(config_id)
    );
}

#[test]
fn credentials_live_in_the_vault_never_in_files_or_replies() {
    let _guard = crate::CA_HOME_LOCK.lock().unwrap();
    let (dir, store) = temp_store();
    std::env::set_var("CA_HOME", dir.path());
    std::env::set_var("CA_SECRET_BACKEND", "file");
    let task = running_task(&store);
    let set = rpc(
        &store,
        "tasks/pushNotificationConfig/set",
        json!({
            "taskId": task.id,
            "pushNotificationConfig": {
                "url": "http://127.0.0.1:9102/hook",
                "authentication": {"schemes": ["Bearer"], "credentials": "whsec-ultra"},
            },
        }),
        1,
    );
    assert!(set.get("error").is_none(), "unexpected: {set}");
    let config_id = set["result"]["pushNotificationConfig"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // The credential is resolvable from the P12 vault under the derived key.
    let resolved = crate::secret::resolve(&vault_key_for(&task.id, &config_id)).unwrap();
    assert_eq!(resolved.expose(), "whsec-ultra");
    // ...and appears nowhere else: not in replies, not in the JSON file.
    let raw_file = std::fs::read_to_string(dir.path().join("a2a_push_configs.json")).unwrap();
    assert!(!raw_file.contains("whsec-ultra"));
    for method in [
        "tasks/pushNotificationConfig/get",
        "tasks/pushNotificationConfig/list",
    ] {
        let reply = rpc(&store, method, json!({"id": task.id}), 2);
        assert!(
            !reply.to_string().contains("whsec-ultra"),
            "{method} leaked"
        );
    }
    let get = rpc(
        &store,
        "tasks/pushNotificationConfig/get",
        json!({"id": task.id}),
        3,
    );
    assert_eq!(
        get["result"]["pushNotificationConfig"]["authentication"]["schemes"],
        json!(["Bearer"])
    );
    // Deleting the config revokes the vault entry.
    let delete = rpc(
        &store,
        "tasks/pushNotificationConfig/delete",
        json!({"id": task.id}),
        4,
    );
    assert_eq!(delete["result"], Value::Null);
    assert!(crate::secret::resolve(&vault_key_for(&task.id, &config_id)).is_none());
    std::env::remove_var("CA_SECRET_BACKEND");
    std::env::remove_var("CA_HOME");
}

#[test]
fn tick_delivers_state_changes_once_and_skips_baseline() {
    let (_dir, store) = temp_store();
    let task = running_task(&store);
    let (url, rx) = webhook_sink(1);
    let set = set_rpc(&store, &task.id, &url);
    assert!(set.get("error").is_none(), "unexpected: {set}");
    let agent = super::notifier::build_agent();
    tick(&store, &agent);
    assert!(
        rx.recv_timeout(Duration::from_millis(300)).is_err(),
        "baseline must not notify"
    );
    store.advance_task(&task.id, Some("worker"), None).unwrap();
    tick(&store, &agent);
    let (head, body) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(head.starts_with("POST /hook HTTP/1.1"), "head: {head}");
    assert!(head
        .to_lowercase()
        .contains("x-a2a-notification-token: echo-1"));
    let event: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(event["kind"], "status-update");
    assert_eq!(event["taskId"], json!(task.id));
    assert_eq!(event["status"]["state"], "completed");
    assert_eq!(event["final"], true);
    // Delivered once: a second tick sees no diff.
    tick(&store, &agent);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn notifier_thread_posts_to_a_live_loopback_webhook() {
    let (dir, store) = temp_store();
    let task = running_task(&store);
    let (url, rx) = webhook_sink(1);
    let set = set_rpc(&store, &task.id, &url);
    assert!(set.get("error").is_none(), "unexpected: {set}");
    let mut notifier = PushNotifier::start(dir.path().to_path_buf(), Duration::from_millis(50));
    // Let the notifier baseline, then fire the transition.
    std::thread::sleep(Duration::from_millis(200));
    store.cancel_task(&task.id).unwrap();
    let (head, body) = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("webhook received the push");
    notifier.stop();
    assert!(head.starts_with("POST /hook HTTP/1.1"), "head: {head}");
    let event: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(event["status"]["state"], "canceled");
    assert_eq!(event["final"], true);
}

#[test]
fn deliver_reports_non_2xx_and_transport_failures() {
    let agent = super::notifier::build_agent();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/gone", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"HTTP/1.1 500 Boom\r\nContent-Length: 0\r\n\r\n");
        }
    });
    let payload = json!({"k": "v"});
    let err = deliver(&agent, &url, None, None, &payload).unwrap_err();
    assert!(err.contains("500"), "unexpected: {err}");
    let refused = deliver(&agent, "http://127.0.0.1:1/", None, None, &payload);
    assert!(refused.is_err());
}

#[test]
fn status_event_mirrors_tasks_get_shape() {
    let (_dir, store) = temp_store();
    let task = running_task(&store);
    let event = status_event(&store, &task);
    assert_eq!(event["status"]["state"], a2a_task_state("running"));
    assert_eq!(event["final"], false);
    assert_eq!(event["contextId"], json!(task.id));
    assert!(event["status"]["message"]["parts"][0]["text"].is_string());
}

#[test]
fn deliver_refuses_redirects_and_never_follows_location() {
    let agent = super::notifier::build_agent();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let dest_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let dest_url = format!("http://{}/destination", dest_listener.local_addr().unwrap());
    let url = format!("http://{}/redirect", listener.local_addr().unwrap());
    let dest_hit = Arc::new(AtomicBool::new(false));
    let dest_hit_clone = dest_hit.clone();

    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let response =
                format!("HTTP/1.1 302 Found\r\nLocation: {dest_url}\r\nContent-Length: 0\r\n\r\n");
            let _ = stream.write_all(response.as_bytes());
        }
    });

    std::thread::spawn(move || {
        dest_listener.set_nonblocking(true).ok();
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(500) {
            if dest_listener.accept().is_ok() {
                dest_hit_clone.store(true, Ordering::SeqCst);
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });

    let payload = json!({"status": "completed"});
    let err = deliver(
        &agent,
        &url,
        Some("secret-token"),
        Some("Bearer secret-vault-token"),
        &payload,
    )
    .expect_err("redirect must be treated as failure, never followed");

    assert!(err.contains("302"), "error should report 302: {err}");
    assert!(
        !dest_hit.load(Ordering::SeqCst),
        "redirect destination was contacted — token leak!"
    );
}
