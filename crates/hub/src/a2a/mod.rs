//! A2A wire helpers shared between the desktop listener (P11b / #335,
//! `src-tauri/src/server/a2a.rs`) and the push-notification extension
//! (issue #364): per-task webhook configs plus a background notifier.
//!
//! ## Spec naming choice
//!
//! A2A v1.0 renamed the JSON-RPC push-config methods to PascalCase
//! (`CreateTaskPushNotificationConfig`, `GetTaskPushNotificationConfig`,
//! `ListTaskPushNotificationConfigs`, `DeleteTaskPushNotificationConfig`).
//! P11b's wire already speaks the legacy v0.3 names (`message/send`,
//! `tasks/get`, `tasks/cancel`), so the canonical names here are the v0.3
//! `tasks/pushNotificationConfig/{set,get,list,delete}`; the v1.0
//! PascalCase names are accepted as aliases and normalized internally.
//!
//! ## Threat model / SSRF posture (owner-accepted)
//!
//! The A2A listener is default-off and loopback-default; callers present
//! the P6 pairing token, so a registered callback URL is already an
//! owner-trusted instruction. Still, registration validates the URL:
//! `http`/`https` only, a host is required, userinfo (`user:pass@`) is
//! rejected (credentials never ride in URLs or logs), and length is
//! bounded. Delivery never follows redirects (only 2xx counts as success),
//! uses a 5s total timeout, and logs only scheme+authority — never paths,
//! tokens, or credential values. DNS-rebinding / internal-network targeting
//! is accepted residual risk under the loopback-default posture, same as
//! the P11b listener itself.
//!
//! ## Storage and secrets
//!
//! Configs persist in `a2a_push_configs.json` next to `hub.db` (the
//! `telegram_binding.json` precedent): task id, config id, callback URL,
//! optional echo `token` (the caller's own value, echoed back on delivery
//! via the `X-A2A-Notification-Token` header — never logged), auth scheme
//! names, and `last_notified_state`. The webhook auth credential itself
//! (`authentication.credentials`) is written to the P12 vault under
//! `a2a.push.<taskId>.<configId>` ([`crate::secret`]); only
//! `has_credentials: true` is persisted. Vault value materializes only as
//! an outbound `Authorization: Bearer` header.
//!
//! ## Event source
//!
//! The notifier observes the same transitions P11a's delegation client
//! emits internally: every local/remote step completion (`advance_task`,
//! `complete_parallel_member`, remote `TaskComplete` via
//! `dispatch_remote_step`), `retry_task`, and `cancel_task` persists
//! through the tasks table. A polling loop diffs each registered task's
//! stored status against `last_notified_state`, so no new hook inside
//! `HubStore` is needed and restarts cannot lose a queued notification.

mod notifier;
mod push;
mod rpc;

pub use notifier::{status_event, PushNotifier, DEFAULT_NOTIFY_POLL};
pub use push::{
    validate_callback_url, vault_key_for, PushConfigEntry, PushConfigStore, PUSH_CONFIG_FILE,
};
pub use rpc::{handle_push_rpc, is_push_method};

use crate::{HubStore, TaskRecord};
use serde_json::{json, Value};

/// Map Hub task states onto A2A task states. Unknown persists as `failed`
/// rather than claiming progress. (Moved verbatim from P11b's a2a.rs so
/// the notifier and the listener share one mapping.)
pub fn a2a_task_state(status: &str) -> &'static str {
    match status {
        "pending" => "submitted",
        "running" => "working",
        "done" => "completed",
        "cancelled" => "canceled",
        "failed" => "failed",
        "input_required" | "input-required" => "input-required",
        "auth_required" | "auth-required" => "auth-required",
        _ => "failed",
    }
}

/// A2A `Task` JSON for one Hub task — the exact shape `tasks/get` returns.
/// Shared by the listener and the notifier so a pushed status event
/// carries the same payload a poll would have returned.
pub fn task_result(store: &HubStore, task: &TaskRecord, context_id: &str) -> Value {
    let state = a2a_task_state(&task.status);
    let message = task
        .last_message_id
        .as_deref()
        .and_then(|id| store.get_message(id).ok())
        .flatten()
        .map(|msg| {
            json!({
                "role": "agent",
                "messageId": msg.id,
                "parts": [{"type": "text", "text": msg.body}],
            })
        });
    let mut status = json!({"state": state});
    if let Some(message) = message {
        status["message"] = message;
    }
    json!({
        "id": task.id,
        "contextId": context_id,
        "status": status,
        "artifacts": [],
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod a2a_push_tests;

#[cfg(test)]
mod review_tests;
