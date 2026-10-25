use super::a2a_push_tests::{running_task, set_rpc, temp_store};
use super::notifier::{build_agent, deliver, status_event, tick};
use super::push::{update_store, vault_key_for, PushConfigStore, MAX_CONFIGS_PER_TASK};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;

#[test]
fn interrupted_tasks_are_nonterminal_on_both_wire_surfaces() {
    let (_dir, store) = temp_store();
    let mut task = running_task(&store);
    for (stored, wire) in [
        ("input_required", "input-required"),
        ("auth_required", "auth-required"),
    ] {
        task.status = stored.into();
        let event = status_event(&store, &task);
        assert_eq!(event["status"]["state"], wire);
        assert_eq!(event["final"], false);
        assert_eq!(
            super::task_result(&store, &task, &task.id)["status"]["state"],
            wire
        );
    }
}

#[test]
fn missing_credential_never_sends_an_unauthenticated_notification() {
    let (dir, store) = temp_store();
    let task = running_task(&store);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/hook", listener.local_addr().unwrap());
    let set = set_rpc(&store, &task.id, &url);
    let id = set["result"]["pushNotificationConfig"]["id"]
        .as_str()
        .unwrap();
    update_store(dir.path(), |configs| {
        configs.find_mut(&task.id, id).unwrap().has_credentials = true;
        Ok(())
    })
    .unwrap();
    store.cancel_task(&task.id).unwrap();
    tick(&store, &build_agent());
    assert!(listener.accept().is_err());
    let registry = PushConfigStore::open(dir.path()).unwrap();
    assert_eq!(
        registry
            .find(&task.id, id)
            .unwrap()
            .last_notified_state
            .as_deref(),
        Some("working")
    );
}

#[test]
fn rejected_registration_does_not_write_a_vault_secret() {
    let _guard = crate::CA_HOME_LOCK.lock().unwrap();
    let (dir, store) = temp_store();
    std::env::set_var("CA_HOME", dir.path());
    std::env::set_var("CA_SECRET_BACKEND", "file");
    let task = running_task(&store);
    for _ in 0..MAX_CONFIGS_PER_TASK {
        let result = set_rpc(&store, &task.id, "http://127.0.0.1:1/hook");
        assert!(result.get("error").is_none());
    }
    let response = super::handle_push_rpc(
        &store,
        "tasks/pushNotificationConfig/set",
        &json!({
            "taskId": task.id,
            "pushNotificationConfig": {"id": "overflow", "url": "http://127.0.0.1:1/hook",
                "authentication": {"schemes": ["Bearer"], "credentials": "rejected-secret"}},
        }),
        &json!(1),
    )
    .unwrap();
    assert!(response.get("error").is_some());
    assert!(crate::secret::resolve(&vault_key_for(&task.id, "overflow")).is_none());
    std::env::remove_var("CA_SECRET_BACKEND");
    std::env::remove_var("CA_HOME");
}

#[test]
fn delivery_does_not_acknowledge_a_replaced_configuration() {
    let (dir, store) = temp_store();
    let task = running_task(&store);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/hook", listener.local_addr().unwrap());
    let set = set_rpc(&store, &task.id, &url);
    let id = set["result"]["pushNotificationConfig"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let task_id = task.id.clone();
    let config_id = id.clone();
    let home = dir.path().to_path_buf();
    let callback = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut buf = [0; 4096];
        stream.read(&mut buf).unwrap();
        update_store(&home, |configs| {
            let entry = configs.find_mut(&task_id, &config_id).unwrap();
            entry.created_at = "replacement-generation".into();
            entry.last_notified_state = Some("new-baseline".into());
            Ok(())
        })
        .unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
    });
    store.cancel_task(&task.id).unwrap();
    tick(&store, &build_agent());
    callback.join().unwrap();
    let registry = PushConfigStore::open(dir.path()).unwrap();
    assert_eq!(
        registry
            .find(&task.id, &id)
            .unwrap()
            .last_notified_state
            .as_deref(),
        Some("new-baseline")
    );
}

#[test]
fn transport_errors_do_not_expose_callback_path_or_query() {
    let error = deliver(
        &build_agent(),
        "http://127.0.0.1:1/private?token=sensitive",
        None,
        None,
        &json!({}),
    )
    .unwrap_err();
    assert!(!error.contains("private"));
    assert!(!error.contains("sensitive"));
}
