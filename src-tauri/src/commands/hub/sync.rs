//! Cloud-sync desktop commands (S4 / #94).

use super::store::open_store;
use hub::sync::{
    self, CleanupCandidate, ConflictDecision, ConflictItem, Diagnostics, HistoryEntry, LockFile,
    PurgeReport, SyncLimits, SyncPlan, SyncSession, Tombstone, TrustEntry, TrustList,
};

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

#[tauri::command]
pub fn hub_sync_conflicts() -> Result<Vec<ConflictItem>, String> {
    let store = open_store()?;
    sync::list_conflicts(store.data_dir()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_resolve(slug: String, choice: String) -> Result<ConflictDecision, String> {
    let store = open_store()?;
    let parsed = sync::ConflictChoice::parse(&choice).map_err(|error| error.to_string())?;
    sync::apply_choice(&store, &slug, parsed).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_tombstones() -> Result<Vec<Tombstone>, String> {
    let store = open_store()?;
    sync::list_tombstones(store.data_dir()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_expired() -> Result<Vec<CleanupCandidate>, String> {
    let store = open_store()?;
    sync::expired_cleanup_candidates(store.data_dir(), chrono::Utc::now())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_purge_expired(confirm: bool) -> Result<PurgeReport, String> {
    if !confirm {
        return Err("purge-expired requires confirm".into());
    }
    let store = open_store()?;
    sync::purge_expired(&store, &[], true).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_devices() -> Result<TrustList, String> {
    let store = open_store()?;
    sync::list_trust(store.data_dir()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_register(id: String) -> Result<TrustEntry, String> {
    let store = open_store()?;
    let parsed = hub::sync::DeviceId::parse(&id).map_err(|error| error.to_string())?;
    sync::register(store.data_dir(), parsed).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_revoke(id: String) -> Result<TrustEntry, String> {
    let store = open_store()?;
    let parsed = hub::sync::DeviceId::parse(&id).map_err(|error| error.to_string())?;
    sync::revoke(store.data_dir(), parsed).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_trust(id: String) -> Result<TrustEntry, String> {
    let store = open_store()?;
    let parsed = hub::sync::DeviceId::parse(&id).map_err(|error| error.to_string())?;
    sync::retrust(store.data_dir(), parsed).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_retry() -> Result<SyncSession, String> {
    let store = open_store()?;
    let schema = store
        .hub_schema_version()
        .map_err(|error| error.to_string())?;
    sync::start_persisted(store.data_dir(), schema, "retry").map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_history() -> Result<Vec<HistoryEntry>, String> {
    let store = open_store()?;
    sync::list_history(store.data_dir()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_diagnostics() -> Result<Diagnostics, String> {
    let store = open_store()?;
    sync::export_diagnostics(store.data_dir()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_sync_limits() -> Result<SyncLimits, String> {
    let store = open_store()?;
    sync::ensure_limits(store.data_dir()).map_err(|error| error.to_string())
}

#[derive(serde::Serialize)]
pub struct SyncStatus {
    pub plan: SyncPlan,
    pub lock: Option<LockFile>,
}
