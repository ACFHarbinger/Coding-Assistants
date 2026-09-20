//! Cloud-sync desktop commands (S4 / #94).

use super::store::open_store;
use hub::sync::{self, LockFile, SyncPlan, SyncSession};

#[tauri::command]
pub fn hub_sync_preview() -> Result<SyncPlan, String> {
    let store = open_store()?;
    let schema = store
        .hub_schema_version()
        .map_err(|error| error.to_string())?;
    sync::build_plan(store.data_dir(), schema, "preview").map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_start(action: String) -> Result<SyncSession, String> {
    let store = open_store()?;
    let schema = store
        .hub_schema_version()
        .map_err(|error| error.to_string())?;
    sync::start_persisted(store.data_dir(), schema, &action).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_status() -> Result<SyncStatus, String> {
    let store = open_store()?;
    let schema = store
        .hub_schema_version()
        .map_err(|error| error.to_string())?;
    let plan =
        sync::build_plan(store.data_dir(), schema, "status").map_err(|error| error.to_string())?;
    let lock = sync::read_lock(store.data_dir()).map_err(|error| error.to_string())?;
    Ok(SyncStatus { plan, lock })
}

#[tauri::command]
pub fn hub_sync_cancel() -> Result<(), String> {
    let store = open_store()?;
    sync::release(store.data_dir()).map_err(|error| error.to_string())
}

#[derive(serde::Serialize)]
pub struct SyncStatus {
    pub plan: SyncPlan,
    pub lock: Option<LockFile>,
}
