//! Offload long/sync harness work off Tauri's IPC dispatch thread.
//!
//! A non-`async` `#[tauri::command]` runs inline on the webview event-loop
//! thread on Linux. Subprocess or large FS work there freezes the whole
//! window (#163 / same class as `hub_get_provider_quotas` and
//! `claude_channel_is_connected`). Prefer `async` + this helper for any
//! command that may take more than a few milliseconds.

pub async fn run_blocking<T, F>(label: &'static str, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| format!("{label} task panicked: {error}"))?
}

pub async fn run_blocking_ok<T, F>(label: &'static str, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| format!("{label} task panicked: {error}"))
}
