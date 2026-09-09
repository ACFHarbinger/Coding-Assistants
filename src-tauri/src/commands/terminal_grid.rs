//! Named terminal grid layout persistence commands (#300, U15).
//!
//! Layouts are stored as atomic JSON files under:
//! `<CA_HOME or ~/.coding-assistants>/terminal-grids/<name>.json`
//!
//! File format:
//! `{ "version": 1, "name": "...", "savedAt": "...", "canvas": { "width": ..., "height": ... }, "layout": { ... } }`

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn layouts_dir() -> PathBuf {
    hub::default_hub_home().join("terminal-grids")
}

/// Validates layout name to prevent path traversal and restrict to safe filenames.
pub fn validate_layout_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Layout name cannot be empty".to_string());
    }
    if trimmed.len() > 64 {
        return Err("Layout name cannot exceed 64 characters".to_string());
    }
    if trimmed.starts_with('.') {
        return Err("Layout name cannot start with a dot".to_string());
    }
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains('\0')
    {
        return Err("Layout name contains invalid path characters".to_string());
    }
    if !trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
    {
        return Err(
            "Layout name must contain only letters, numbers, spaces, hyphens, and underscores"
                .to_string(),
        );
    }
    Ok(trimmed.to_string())
}

fn layout_file_path(name: &str) -> Result<PathBuf, String> {
    let valid_name = validate_layout_name(name)?;
    Ok(layouts_dir().join(format!("{valid_name}.json")))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalGridCanvas {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalGridLayoutFile {
    pub version: u32,
    pub name: String,
    pub saved_at: String,
    pub canvas: Option<TerminalGridCanvas>,
    pub layout: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalGridLayoutSummary {
    pub name: String,
    pub saved_at: String,
    pub version: u32,
    pub canvas: Option<TerminalGridCanvas>,
}

pub fn hub_save_terminal_grid_layout_blocking(
    name: String,
    layout: serde_json::Value,
    canvas: Option<TerminalGridCanvas>,
) -> Result<TerminalGridLayoutFile, String> {
    let valid_name = validate_layout_name(&name)?;
    let dir = layouts_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create terminal-grids directory: {e}"))?;

    let target_path = dir.join(format!("{valid_name}.json"));
    let temp_name = format!(
        ".{valid_name}.tmp.{}.{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    let temp_path = dir.join(temp_name);

    let file_data = TerminalGridLayoutFile {
        version: 1,
        name: valid_name,
        saved_at: chrono::Utc::now().to_rfc3339(),
        canvas,
        layout,
    };

    let serialized = serde_json::to_string_pretty(&file_data)
        .map_err(|e| format!("failed to serialize layout JSON: {e}"))?;

    let write_res = (|| -> Result<(), std::io::Error> {
        use std::io::Write;
        let mut f = std::fs::File::create(&temp_path)?;
        f.write_all(serialized.as_bytes())?;
        f.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_res {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("failed to write temporary layout file: {e}"));
    }

    if let Err(e) = std::fs::rename(&temp_path, &target_path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("failed to commit layout file: {e}"));
    }

    Ok(file_data)
}

pub fn hub_load_terminal_grid_layout_blocking(
    name: String,
) -> Result<TerminalGridLayoutFile, String> {
    let target_path = layout_file_path(&name)?;
    if !target_path.exists() {
        return Err(format!("Layout '{}' not found", name.trim()));
    }
    let content = std::fs::read_to_string(&target_path)
        .map_err(|e| format!("failed to read layout file: {e}"))?;
    serde_json::from_str::<TerminalGridLayoutFile>(&content)
        .map_err(|e| format!("failed to parse layout JSON: {e}"))
}

pub fn hub_list_terminal_grid_layouts_blocking() -> Result<Vec<TerminalGridLayoutSummary>, String> {
    let dir = layouts_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("failed to read terminal-grids directory: {e}"))?;

    let mut summaries = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let file_name = entry.file_name();
        let file_str = file_name.to_string_lossy();
        if file_str.starts_with('.') || !file_str.ends_with(".json") {
            continue;
        }
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Ok(data) = serde_json::from_str::<TerminalGridLayoutFile>(&content) {
            summaries.push(TerminalGridLayoutSummary {
                name: data.name,
                saved_at: data.saved_at,
                version: data.version,
                canvas: data.canvas,
            });
        }
    }
    summaries.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
    Ok(summaries)
}

pub fn hub_delete_terminal_grid_layout_blocking(name: String) -> Result<(), String> {
    let target_path = layout_file_path(&name)?;
    if !target_path.exists() {
        return Err(format!("Layout '{}' not found", name.trim()));
    }
    std::fs::remove_file(&target_path).map_err(|e| format!("failed to delete layout file: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn hub_save_terminal_grid_layout(
    name: String,
    layout: serde_json::Value,
    canvas: Option<TerminalGridCanvas>,
) -> Result<TerminalGridLayoutFile, String> {
    tauri::async_runtime::spawn_blocking(move || {
        hub_save_terminal_grid_layout_blocking(name, layout, canvas)
    })
    .await
    .map_err(|e| format!("hub_save_terminal_grid_layout worker panic: {e}"))?
}

#[tauri::command]
pub async fn hub_load_terminal_grid_layout(name: String) -> Result<TerminalGridLayoutFile, String> {
    tauri::async_runtime::spawn_blocking(move || hub_load_terminal_grid_layout_blocking(name))
        .await
        .map_err(|e| format!("hub_load_terminal_grid_layout worker panic: {e}"))?
}

#[tauri::command]
pub async fn hub_list_terminal_grid_layouts() -> Result<Vec<TerminalGridLayoutSummary>, String> {
    tauri::async_runtime::spawn_blocking(hub_list_terminal_grid_layouts_blocking)
        .await
        .map_err(|e| format!("hub_list_terminal_grid_layouts worker panic: {e}"))?
}

#[tauri::command]
pub async fn hub_delete_terminal_grid_layout(name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || hub_delete_terminal_grid_layout_blocking(name))
        .await
        .map_err(|e| format!("hub_delete_terminal_grid_layout worker panic: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_ca_home<T>(prefix: &str, run: impl FnOnce() -> T) -> T {
        let _guard = crate::commands::commands::tests::CA_HOME_ENV_LOCK
            .lock()
            .unwrap();
        let dir = std::env::temp_dir().join(format!(
            "hub-terminal-grid-{prefix}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("CA_HOME", &dir);
        let out = run();
        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
        out
    }

    #[test]
    fn test_validate_layout_name() {
        assert_eq!(validate_layout_name("quad-split").unwrap(), "quad-split");
        assert_eq!(validate_layout_name(" my layout ").unwrap(), "my layout");
        assert_eq!(validate_layout_name("A_B-1 2").unwrap(), "A_B-1 2");

        assert!(validate_layout_name("").is_err());
        assert!(validate_layout_name("   ").is_err());
        assert!(validate_layout_name(".hidden").is_err());
        assert!(validate_layout_name("a/b").is_err());
        assert!(validate_layout_name("a\\b").is_err());
        assert!(validate_layout_name("../test").is_err());
        assert!(validate_layout_name("test\0bad").is_err());
        assert!(validate_layout_name(&"x".repeat(65)).is_err());
    }

    #[test]
    fn test_save_load_list_delete_round_trip() {
        with_ca_home("roundtrip", || {
            let layout_json = serde_json::json!({
                "type": "split",
                "id": "split-1",
                "direction": "row",
                "ratio": 0.5,
                "first": { "type": "leaf", "id": "leaf-1", "harness": "claude" },
                "second": { "type": "leaf", "id": "leaf-2", "harness": "gemini" }
            });
            let canvas = Some(TerminalGridCanvas {
                width: 1280.0,
                height: 720.0,
            });

            // 1. Initially empty
            let listed = hub_list_terminal_grid_layouts_blocking().unwrap();
            assert_eq!(listed.len(), 0);

            // 2. Save
            let saved = hub_save_terminal_grid_layout_blocking(
                "my-pair".to_string(),
                layout_json.clone(),
                canvas.clone(),
            )
            .unwrap();
            assert_eq!(saved.name, "my-pair");
            assert_eq!(saved.version, 1);
            assert_eq!(saved.canvas, canvas);
            assert_eq!(saved.layout, layout_json);

            // 3. Load
            let loaded = hub_load_terminal_grid_layout_blocking("my-pair".to_string()).unwrap();
            assert_eq!(loaded, saved);

            // 4. List
            let listed = hub_list_terminal_grid_layouts_blocking().unwrap();
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].name, "my-pair");
            assert_eq!(listed[0].canvas, canvas);

            // 5. Delete
            hub_delete_terminal_grid_layout_blocking("my-pair".to_string()).unwrap();
            let listed_after = hub_list_terminal_grid_layouts_blocking().unwrap();
            assert_eq!(listed_after.len(), 0);

            // 6. Loading deleted layout returns error
            let err = hub_load_terminal_grid_layout_blocking("my-pair".to_string()).unwrap_err();
            assert!(err.contains("not found"));
        });
    }

    #[test]
    fn test_atomic_write_leaves_no_temp_files() {
        with_ca_home("atomic", || {
            let layout_json = serde_json::json!({ "type": "leaf", "id": "1", "harness": "muse" });
            hub_save_terminal_grid_layout_blocking("atomic-test".to_string(), layout_json, None)
                .unwrap();

            let dir = layouts_dir();
            let mut file_names: Vec<String> = std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                .collect();
            file_names.sort();

            assert_eq!(file_names, vec!["atomic-test.json"]);
        });
    }
}
