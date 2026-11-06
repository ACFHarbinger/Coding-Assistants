//! Saved named workspaces (U24 / #317).
use super::store::open_store;
use crate::core::workspace::expand_tilde;
use hub::WorkspaceRecord;

#[tauri::command]
pub fn hub_list_workspaces() -> Result<Vec<WorkspaceRecord>, String> {
    open_store()?.list_workspaces().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_save_workspace(
    id: Option<String>,
    name: String,
    path: String,
    link_session_id: Option<String>,
) -> Result<WorkspaceRecord, String> {
    let path = expand_tilde(&path).to_string_lossy().into_owned();
    open_store()?
        .save_workspace(id.as_deref(), &name, &path, link_session_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_delete_workspace(id: String) -> Result<(), String> {
    open_store()?
        .delete_workspace(&id)
        .map_err(|e| e.to_string())
}
