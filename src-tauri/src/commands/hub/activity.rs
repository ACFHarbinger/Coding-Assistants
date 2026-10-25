//! Activity commands for the Shared Hub Dashboard (Roadmap D1 / #87 and D4 / #324).
use crate::commands::commands::store::open_store;
use hub::{ActivityFilter, ActivityItem, TaskTimelineView};

#[tauri::command]
pub fn hub_get_activity_view(filter: Option<ActivityFilter>) -> Result<Vec<ActivityItem>, String> {
    let filter = filter.unwrap_or_default();
    open_store()?
        .get_activity_view(&filter)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_get_task_timeline(task_id: String) -> Result<TaskTimelineView, String> {
    open_store()?
        .get_task_timeline(&task_id)
        .map_err(|e| e.to_string())
}
