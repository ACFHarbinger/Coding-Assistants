//! Settings entitlement for the Linux systemd user trigger (S5a / #356).
//!
//! Only two actions are exposed: install/update the fixed `ca-schedule`
//! unit pair, or remove it. There is no generic "run this action on a
//! timer" entry point and no privileged unit path.

use hub::schedule_trigger::{self, TriggerConfig, TriggerStatus};

#[tauri::command]
pub async fn settings_schedule_trigger_status() -> Result<TriggerStatus, String> {
    tauri::async_runtime::spawn_blocking(schedule_trigger::status)
        .await
        .map_err(|error| error.to_string())
}

/// Install or refresh the user timer. `workspace` is the Settings window's
/// current workspace; it becomes the service's `WorkingDirectory` and is
/// kept separate from the Hub home that `--home`/`CA_HOME` pin.
#[tauri::command]
pub async fn settings_schedule_trigger_install(
    ca_bin: Option<String>,
    interval_minutes: u32,
    workspace: String,
) -> Result<TriggerStatus, String> {
    let workspace = workspace.trim();
    if workspace.is_empty() {
        return Err("Select an absolute workspace before installing the trigger".to_string());
    }
    let config = TriggerConfig {
        ca_bin: schedule_trigger::resolve_ca_binary(ca_bin.as_deref())?,
        ca_home: hub::default_hub_home(),
        workspace: std::path::PathBuf::from(workspace),
        interval_minutes,
    };
    tauri::async_runtime::spawn_blocking(move || schedule_trigger::install(&config))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn settings_schedule_trigger_remove() -> Result<TriggerStatus, String> {
    tauri::async_runtime::spawn_blocking(schedule_trigger::remove)
        .await
        .map_err(|error| error.to_string())?
}
