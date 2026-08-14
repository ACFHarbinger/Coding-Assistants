//! Workspace-scoped Chat & Memory presence. Thin Tauri wrap around
//! `hub::workspace_agent_presence` — do not re-derive this from
//! `detect_agent_processes`.

use crate::commands::commands::store::open_store;
use hub::{workspace_agent_presence, WorkspaceAgentPresence};
use std::path::Path;

#[tauri::command]
pub fn hub_workspace_agent_presence(workspace: String) -> Result<WorkspaceAgentPresence, String> {
    workspace_agent_presence(&open_store()?, Path::new(&workspace))
}
