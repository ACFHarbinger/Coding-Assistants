//! Background A2A push notifier (issue #364): watches tasks with a
//! registered webhook config and POSTs status events as they progress, so
//! remote callers stop polling `tasks/get`.
//!
//! Event source: the tasks table every internal transition already flows
//! through — P11a's delegation client (`advance_task` /
//! `dispatch_remote_step` on a peer `TaskComplete`), `retry_task`,
//! `cancel_task`, and P11b's own `message/send`. Polling diffs against the
//! persisted `last_notified_state`, so the notifier needs no hook inside
//! `HubStore` and a restart re-baselines instead of replaying history.
//!
//! Delivery policy: best-effort with bounded retries (3 attempts, 200ms
//! and 1s backoff), 5s total timeout, redirects never followed (only 2xx
//! counts), response body discarded. Failure is logged with a sanitized
//! scheme://authority — never the full URL, echo token, or credential.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::JoinHandle;
use std::time::Duration;

use super::push::{read_store, update_store, vault_key_for, PushConfigEntry};
use super::{a2a_task_state, task_result, HubStore, TaskRecord};

/// Default poll interval for the production notifier.
pub const DEFAULT_NOTIFY_POLL: Duration = Duration::from_secs(1);
const DELIVERY_TIMEOUT: Duration = Duration::from_secs(5);
/// Backoffs before attempt 2 and 3 (attempt 1 fires immediately).
const RETRY_BACKOFFS: [Duration; 2] = [Duration::from_millis(200), Duration::from_secs(1)];

/// A2A `TaskStatusUpdateEvent` for one Hub task. `status` is byte-identical
/// to what `tasks/get` returns for the same task; `final` marks terminal
/// states so the webhook receiver can close its side out.
pub fn status_event(store: &HubStore, task: &TaskRecord) -> Value {
    let snapshot = task_result(store, task, &task.id);
    let state = a2a_task_state(&task.status);
    let terminal = matches!(state, "completed" | "canceled" | "failed");
    json!({
        "taskId": task.id,
        "contextId": snapshot["contextId"],
        "kind": "status-update",
        "status": snapshot["status"],
        "final": terminal,
    })
}

/// scheme://authority for logs — path/query (which could carry secrets)
/// are dropped. Never log a full callback URL.
fn sanitized_target(url: &str) -> String {
    url.parse::<ureq::http::Uri>()
        .map(|uri| {
            format!(
                "{}://{}",
                uri.scheme_str().unwrap_or("?"),
                uri.authority().map(|a| a.as_str()).unwrap_or("?")
            )
        })
        .unwrap_or_else(|_| "<unparseable>".to_string())
}

pub(crate) fn build_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(DELIVERY_TIMEOUT))
        .max_redirects(0)
        // 3xx/4xx/5xx must surface as statuses, not errors: redirects are
        // never followed and only 2xx acknowledges a notification.
        .http_status_as_error(false)
        .build()
        .into()
}

/// One POST of `payload` to the callback. `token` rides as
/// `X-A2A-Notification-Token` (A2A echo convention); `auth_header` is the
/// already-resolved `Bearer <vault secret>` value or `None`.
pub fn deliver(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
    auth_header: Option<&str>,
    payload: &Value,
) -> Result<(), String> {
    let mut request = agent.post(url);
    if let Some(token) = token {
        request = request.header("X-A2A-Notification-Token", token);
    }
    if let Some(auth) = auth_header {
        request = request.header("Authorization", auth);
    }
    let response = request
        .send_json(payload)
        .map_err(|_| "webhook POST transport failed".to_string())?;
    let status = response.status().as_u16();
    // Body intentionally discarded unread; only the status acknowledges.
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(format!("webhook answered HTTP {status}"))
    }
}

fn deliver_with_retries(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
    auth_header: Option<&str>,
    payload: &Value,
) -> Result<(), String> {
    let mut last_error = deliver(agent, url, token, auth_header, payload);
    for backoff in RETRY_BACKOFFS {
        if last_error.is_ok() {
            return last_error;
        }
        std::thread::sleep(backoff);
        last_error = deliver(agent, url, token, auth_header, payload);
    }
    last_error
}

/// Outbound `Authorization` value for one config, or `None`. The credential
/// materializes here only, straight from the P12 vault into the header.
fn auth_header_for(entry: &PushConfigEntry) -> Result<Option<String>, &'static str> {
    if !entry.has_credentials {
        return Ok(None);
    }
    crate::secret::resolve(&vault_key_for(&entry.task_id, &entry.config_id))
        .map(|secret| Some(format!("Bearer {}", secret.expose())))
        .ok_or("configured webhook credential is unavailable")
}

/// One poll pass: deliver every state change since `last_notified_state`.
/// Public so tests can drive a single tick without the thread.
pub fn tick(store: &HubStore, agent: &ureq::Agent) {
    let entries = match read_store(store.data_dir(), |registry| {
        registry
            .configs()
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| (entry.clone(), auth_header_for(entry)))
            .collect::<Vec<_>>()
    }) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("a2a push notifier: cannot read config registry: {error}");
            return;
        }
    };
    for (entry, auth_header) in &entries {
        let task = match store.get_task(&entry.task_id) {
            Ok(Some(task)) => task,
            // Missing task: nothing to report; the config stays for `get`.
            _ => continue,
        };
        let state = a2a_task_state(&task.status);
        if entry.last_notified_state.as_deref() == Some(state) {
            continue;
        }
        let payload = status_event(store, &task);
        let auth_header = match auth_header {
            Ok(header) => header,
            Err(error) => {
                eprintln!("a2a push notifier: {error}");
                continue;
            }
        };
        if let Err(error) = deliver_with_retries(
            agent,
            &entry.url,
            entry.token.as_deref(),
            auth_header.as_deref(),
            &payload,
        ) {
            eprintln!(
                "a2a push notifier: delivery to {} failed: {error}",
                sanitized_target(&entry.url)
            );
        }
        // Best-effort: record the attempt either way so a dead webhook does
        // not wedge the loop into an infinite retry across ticks.
        let _ = update_store(store.data_dir(), |configs| {
            if let Some(stored) = configs.find_mut(&entry.task_id, &entry.config_id) {
                // A replacement config has its own baseline; this delivery
                // belongs only to the snapshot used above.
                if stored.created_at == entry.created_at {
                    stored.last_notified_state = Some(state.to_string());
                }
            }
            Ok(())
        });
    }
}

fn run(hub_home: PathBuf, poll: Duration, running: Arc<AtomicBool>) {
    let store = match HubStore::open(&hub_home) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("a2a push notifier: cannot open hub store: {error}");
            return;
        }
    };
    let agent = build_agent();
    while running.load(Ordering::SeqCst) {
        tick(&store, &agent);
        // Sleep in small slices so `stop` joins promptly.
        let mut waited = Duration::ZERO;
        while waited < poll && running.load(Ordering::SeqCst) {
            let slice = Duration::from_millis(50).min(poll - waited);
            std::thread::sleep(slice);
            waited += slice;
        }
    }
}

/// Background notifier started alongside the A2A listener. Owns one
/// thread; `stop` (or drop) joins it.
pub struct PushNotifier {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl PushNotifier {
    pub fn start(hub_home: PathBuf, poll: Duration) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let flag = running.clone();
        let handle = std::thread::spawn(move || run(hub_home, poll, flag));
        Self {
            running,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for PushNotifier {
    fn drop(&mut self) {
        self.stop();
    }
}
