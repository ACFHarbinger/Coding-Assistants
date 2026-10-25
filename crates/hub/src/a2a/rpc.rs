//! JSON-RPC handlers for the A2A push-notification-config methods
//! (issue #364). Pure logic against a `HubStore` — no sockets — so every
//! method is unit-testable against a temp store, exactly like P11b's
//! `handle_rpc`.
//!
//! Canonical names are the v0.3 `tasks/pushNotificationConfig/*` verbs the
//! rest of our wire already speaks; the A2A v1.0 PascalCase names are
//! accepted aliases (see the `a2a` module docs). Params are parsed
//! leniently from both shapes: v0.3 `{taskId, pushNotificationConfig}` /
//! `{id, pushNotificationConfigId}` and v1.0 `{parent: "tasks/<id>",
//! config}` / `{name: "tasks/<t>/pushNotificationConfigs/<c>"}`.

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use super::push::{
    is_valid_id, update_store, validate_callback_url, vault_key_for, PushConfigEntry,
    PushConfigStore, MAX_CONFIGS_PER_TASK, MAX_CONFIGS_TOTAL,
};
use super::{a2a_task_state, HubStore};
use crate::secret;

pub const METHOD_SET: &str = "tasks/pushNotificationConfig/set";
pub const METHOD_GET: &str = "tasks/pushNotificationConfig/get";
pub const METHOD_LIST: &str = "tasks/pushNotificationConfig/list";
pub const METHOD_DELETE: &str = "tasks/pushNotificationConfig/delete";

/// Normalize v0.3 canonical and v1.0 PascalCase alias to one verb.
fn canonical(method: &str) -> Option<&'static str> {
    Some(match method {
        METHOD_SET | "CreateTaskPushNotificationConfig" => METHOD_SET,
        METHOD_GET | "GetTaskPushNotificationConfig" => METHOD_GET,
        METHOD_LIST | "ListTaskPushNotificationConfigs" => METHOD_LIST,
        METHOD_DELETE | "DeleteTaskPushNotificationConfig" => METHOD_DELETE,
        _ => return None,
    })
}

/// Whether `method` is one of the four push-config verbs (either naming).
pub fn is_push_method(method: &str) -> bool {
    canonical(method).is_some()
}

/// Dispatch one push-config call. `None` means "not ours" so the caller
/// falls through to its own method table.
pub fn handle_push_rpc(
    store: &HubStore,
    method: &str,
    params: &Value,
    id: &Value,
) -> Option<Value> {
    Some(match canonical(method)? {
        METHOD_SET => set_config(store, params, id),
        METHOD_GET => get_config(store, params, id, false),
        METHOD_LIST => list_configs(store, params, id),
        METHOD_DELETE => get_config(store, params, id, true),
        _ => unreachable!(),
    })
}

fn rpc_error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn rpc_ok(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Task id from any of the accepted param shapes.
fn param_task_id(params: &Value) -> Option<String> {
    for key in ["taskId", "id"] {
        if let Some(value) = params.get(key).and_then(|v| v.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    for key in ["parent", "name"] {
        if let Some(value) = params.get(key).and_then(|v| v.as_str()) {
            // v1.0 resource names: "tasks/<taskId>[/pushNotificationConfigs/<configId>]".
            let mut segments = value.split('/');
            if segments.next() == Some("tasks") {
                if let Some(task) = segments.next().map(str::trim).filter(|t| !t.is_empty()) {
                    return Some(task.to_string());
                }
            }
        }
    }
    None
}

/// Config id from explicit fields or a v1.0 `name` suffix.
fn param_config_id(params: &Value, config: Option<&Value>) -> Option<String> {
    for key in ["pushNotificationConfigId", "configId"] {
        if let Some(value) = params.get(key).and_then(|v| v.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    if let Some(value) = config
        .and_then(|c| c.get("id"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return Some(value.to_string());
    }
    params
        .get("name")
        .and_then(|v| v.as_str())
        // Only "tasks/<t>/pushNotificationConfigs/<c>" carries a config id.
        .and_then(|name| name.split("/pushNotificationConfigs/").nth(1))
        .map(str::trim)
        .filter(|tail| !tail.is_empty() && is_valid_id(tail))
        .map(str::to_string)
}

/// Wire view of one stored config. `token` round-trips (it is the caller's
/// own echo value); `authentication.credentials` never does — only scheme
/// names. Vault material is resolved only at delivery time.
fn config_json(entry: &PushConfigEntry) -> Value {
    let mut config = json!({"id": entry.config_id, "url": entry.url});
    if let Some(token) = &entry.token {
        config["token"] = json!(token);
    }
    if !entry.auth_schemes.is_empty() || entry.has_credentials {
        config["authentication"] = json!({"schemes": entry.auth_schemes});
    }
    json!({"taskId": entry.task_id, "pushNotificationConfig": config})
}

fn load_store(store: &HubStore, id: &Value) -> Result<PushConfigStore, Value> {
    PushConfigStore::open(store.data_dir())
        .map_err(|error| rpc_error(id, -32000, &error.to_string()))
}

fn set_config(store: &HubStore, params: &Value, id: &Value) -> Value {
    let Some(task_id) = param_task_id(params) else {
        return rpc_error(id, -32602, "pushNotificationConfig/set needs a task id");
    };
    if !is_valid_id(&task_id) {
        return rpc_error(id, -32602, "invalid task id");
    }
    let task = match store.get_task(&task_id) {
        Ok(Some(task)) => task,
        Ok(None) => return rpc_error(id, -32001, "task not found"),
        Err(error) => return rpc_error(id, -32000, &error.to_string()),
    };
    let config = params
        .get("pushNotificationConfig")
        .or_else(|| params.get("config"));
    let url = config
        .and_then(|c| c.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if let Err(reason) = validate_callback_url(url) {
        return rpc_error(id, -32602, &reason);
    }
    let config_id = match param_config_id(params, config) {
        Some(value) if is_valid_id(&value) => value,
        Some(_) => return rpc_error(id, -32602, "invalid config id"),
        None => Uuid::new_v4().simple().to_string(),
    };
    let token = config
        .and_then(|c| c.get("token"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|t| !t.trim().is_empty());
    let auth = config.and_then(|c| c.get("authentication"));
    let schemes: Vec<String> = auth
        .and_then(|a| a.get("schemes"))
        .and_then(|s| s.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let credentials = auth
        .and_then(|a| a.get("credentials"))
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .filter(|c| !c.trim().is_empty());
    let enabled = config
        .and_then(|c| c.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let vault_key = vault_key_for(&task_id, &config_id);

    let entry = PushConfigEntry {
        task_id: task_id.clone(),
        config_id,
        url: url.trim().to_string(),
        token,
        auth_schemes: schemes,
        has_credentials: credentials.is_some(),
        enabled,
        created_at: Utc::now().to_rfc3339(),
        // Baseline: transitions before registration are not notifications.
        last_notified_state: Some(a2a_task_state(&task.status).to_string()),
    };
    let result = config_json(&entry);
    let write = update_store(store.data_dir(), |configs| {
        if configs.find(&task_id, &entry.config_id).is_none() {
            if configs.count_for_task(&task_id) >= MAX_CONFIGS_PER_TASK {
                return Err(crate::store::HubError::Invalid(format!(
                    "per-task push config limit ({MAX_CONFIGS_PER_TASK}) reached"
                )));
            }
            if configs.configs().len() >= MAX_CONFIGS_TOTAL {
                return Err(crate::store::HubError::Invalid(format!(
                    "global push config limit ({MAX_CONFIGS_TOTAL}) reached"
                )));
            }
        }
        // Check capacity before changing credentials, and serialize credential
        // changes with notifier snapshots and registry mutations.
        if let Some(secret_value) = &credentials {
            secret::set_secret(&vault_key, secret_value).map_err(|error| {
                crate::store::HubError::Invalid(format!("vault store failed: {error}"))
            })?;
        } else if configs
            .find(&task_id, &entry.config_id)
            .is_some_and(|old| old.has_credentials)
        {
            secret::clear_secret(&vault_key).map_err(|error| {
                crate::store::HubError::Invalid(format!("vault clear failed: {error}"))
            })?;
        }
        configs.upsert(entry);
        Ok(())
    });
    match write {
        Ok(()) => rpc_ok(id, result),
        Err(error) => rpc_error(id, -32000, &error.to_string()),
    }
}

/// Shared reader for `get` and `delete` (delete is `get` + removal;
/// A2A v1.0 delete is idempotent, so a missing config deletes cleanly).
fn get_config(store: &HubStore, params: &Value, id: &Value, delete: bool) -> Value {
    let Some(task_id) = param_task_id(params) else {
        return rpc_error(
            id,
            -32602,
            "pushNotificationConfig get/delete needs a task id",
        );
    };
    let registry = match load_store(store, id) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    let candidates = registry.list_for_task(&task_id);
    if candidates.is_empty() {
        if delete {
            return rpc_ok(id, Value::Null);
        }
        return rpc_error(id, -32001, "push notification config not found");
    }
    let entry = match param_config_id(params, None) {
        Some(config_id) => match candidates.iter().find(|e| e.config_id == config_id) {
            Some(entry) => entry.clone(),
            None => {
                if delete {
                    return rpc_ok(id, Value::Null);
                }
                return rpc_error(id, -32001, "push notification config not found");
            }
        },
        None if candidates.len() == 1 => candidates[0].clone(),
        None => {
            return rpc_error(
                id,
                -32602,
                "multiple configs registered; pushNotificationConfigId required",
            )
        }
    };
    if delete {
        let config_id = entry.config_id.clone();
        let removed = update_store(store.data_dir(), |configs| {
            if configs
                .find(&task_id, &config_id)
                .is_some_and(|current| current.has_credentials)
            {
                secret::clear_secret(&vault_key_for(&task_id, &config_id)).map_err(|error| {
                    crate::store::HubError::Invalid(format!("vault clear failed: {error}"))
                })?;
            }
            configs.remove(&task_id, &config_id);
            Ok(())
        });
        if let Err(error) = removed {
            return rpc_error(id, -32000, &error.to_string());
        }
        return rpc_ok(id, Value::Null);
    }
    rpc_ok(id, config_json(&entry))
}

fn list_configs(store: &HubStore, params: &Value, id: &Value) -> Value {
    let Some(task_id) = param_task_id(params) else {
        return rpc_error(id, -32602, "pushNotificationConfig/list needs a task id");
    };
    match store.get_task(&task_id) {
        Ok(Some(_)) => {}
        Ok(None) => return rpc_error(id, -32001, "task not found"),
        Err(error) => return rpc_error(id, -32000, &error.to_string()),
    }
    let registry = match load_store(store, id) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    let configs: Vec<Value> = registry
        .list_for_task(&task_id)
        .iter()
        .map(config_json)
        .collect();
    rpc_ok(id, Value::Array(configs))
}
