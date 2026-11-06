//! U19 / #313 Git branches tab commands.

use crate::commands::commands::store::open_store;
use hub::{list_workspace_branches, BranchList};
use std::path::PathBuf;

fn require_absolute(workspace: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(workspace);
    if !path.is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    Ok(path)
}

#[tauri::command]
pub async fn hub_list_git_branches(workspace: String) -> Result<BranchList, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = require_absolute(&workspace)?;
        let store = open_store()?;
        let overrides = store
            .list_branch_issue_links(&workspace)
            .map_err(|e| e.to_string())?;
        list_workspace_branches(&path, &overrides).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("hub_list_git_branches worker panic: {e}"))?
}

#[tauri::command]
pub async fn hub_set_branch_issue(
    workspace: String,
    branch: String,
    issue_number: Option<i64>,
) -> Result<BranchList, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = require_absolute(&workspace)?;
        let store = open_store()?;
        store
            .set_branch_issue_link(&workspace, &branch, issue_number)
            .map_err(|e| e.to_string())?;
        let overrides = store
            .list_branch_issue_links(&workspace)
            .map_err(|e| e.to_string())?;
        list_workspace_branches(&path, &overrides).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("hub_set_branch_issue worker panic: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_absolute_rejects_relative() {
        assert!(require_absolute("rel/x").is_err());
        assert!(require_absolute("/abs/x").is_ok());
    }
}
