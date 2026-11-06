//! Workspace-scoped Chat & Memory presence. Thin Tauri wrap around
//! `hub::workspace_agent_presence` — do not re-derive this from
//! `detect_agent_processes`.
//!
//! Claude liveness shells out to `claude agents --json`; Messager polls this
//! every few seconds. Must not run as a sync command on the IPC thread (#163).

use crate::commands::commands::store::open_store;
use crate::harness::blocking::run_blocking;
use hub::{workspace_agent_presence, WorkspaceAgentPresence};
use std::path::Path;

#[tauri::command]
pub async fn hub_workspace_agent_presence(
    workspace: String,
) -> Result<WorkspaceAgentPresence, String> {
    run_blocking("hub_workspace_agent_presence", move || {
        workspace_agent_presence(&open_store()?, Path::new(&workspace))
    })
    .await
}
