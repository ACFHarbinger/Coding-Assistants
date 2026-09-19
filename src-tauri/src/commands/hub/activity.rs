//! Activity commands for the Shared Hub Dashboard (Roadmap D4 / #324).
use crate::commands::commands::store::open_store;
use hub::{ActivityFilter, ActivityItem};

#[tauri::command]
pub fn hub_get_activity_view(filter: Option<ActivityFilter>) -> Result<Vec<ActivityItem>, String> {
    let filter = filter.unwrap_or_default();
    open_store()?
        .get_activity_view(&filter)
        .map_err(|e| e.to_string())
}
