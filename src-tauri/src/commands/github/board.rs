//! U18 / #312 Kanban board Tauri commands.

use crate::commands::commands::store::open_store;
use hub::{
    empty_project_board, list_issue_branches, list_project_board, set_project_item_status,
    BoardCard, ProjectBoard,
};
use std::path::PathBuf;

async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn hub_list_project_board(workspace: Option<String>) -> Result<ProjectBoard, String> {
    blocking(move || {
        let store = open_store()?;
        let branches = workspace
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .map(|p| list_issue_branches(&p))
            .unwrap_or_default();
        let mut board = match list_project_board(&branches) {
            Ok(board) => {
                let _ = store.save_github_board_cache(&board);
                board
            }
            Err(error) => match store.load_github_board_cache() {
                Ok(Some((fetched_at, mut cached))) => {
                    cached.notice = Some(format!(
                        "GitHub project unavailable ({error}). Showing cached board from {fetched_at}."
                    ));
                    cached
                }
                _ => empty_project_board(Some(format!(
                    "GitHub project unavailable ({error}). Showing Hub-only cards."
                ))),
            },
        };
        store
            .apply_overlays(&mut board)
            .map_err(|e| e.to_string())?;
        store
            .merge_internal_cards(&mut board)
            .map_err(|e| e.to_string())?;
        Ok(board)
    })
    .await
}

#[tauri::command]
pub async fn hub_move_board_card(
    kind: String,
    id: String,
    url: Option<String>,
    status: String,
) -> Result<(), String> {
    blocking(move || {
        if status.trim().is_empty() {
            return Err("status is required".into());
        }
        if kind == "internal" {
            let store = open_store()?;
            return store
                .set_internal_board_status(&id, &status)
                .map_err(|e| e.to_string());
        }
        let url = url.ok_or_else(|| "github cards need an issue url".to_string())?;
        set_project_item_status(&url, &status).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn hub_create_internal_board_card(
    title: String,
    status: Option<String>,
    assignees: Option<Vec<String>>,
    deadline: Option<String>,
) -> Result<BoardCard, String> {
    blocking(move || {
        let store = open_store()?;
        store
            .upsert_internal_board_card(
                &title,
                status.as_deref().unwrap_or("Backlog"),
                assignees.as_deref().unwrap_or(&[]),
                deadline.as_deref(),
            )
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn hub_set_board_overlay(
    kind: Option<String>,
    id: Option<String>,
    issue_number: Option<i64>,
    assignees: Option<Vec<String>>,
    deadline: Option<String>,
) -> Result<(), String> {
    blocking(move || {
        let store = open_store()?;
        if kind.as_deref() == Some("internal") {
            let id = id.ok_or_else(|| "internal cards need an id".to_string())?;
            return store
                .set_internal_board_meta(&id, assignees.as_deref(), Some(deadline.as_deref()))
                .map_err(|e| e.to_string());
        }
        let issue_number =
            issue_number.ok_or_else(|| "github cards need an issue number".to_string())?;
        store
            .set_board_overlay(
                issue_number,
                assignees.as_deref(),
                Some(deadline.as_deref()),
            )
            .map_err(|e| e.to_string())
    })
    .await
}
