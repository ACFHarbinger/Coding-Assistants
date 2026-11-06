//! Terminal grid layout export/import to user-chosen files (#300, U15
//! follow-up), split out of `terminal_grid.rs` to stay under the 500-LoC cap.
//!
//! Export writes the same version-1 JSON format as a named save, but to an
//! arbitrary path outside the CA_HOME store. Import accepts that same
//! format, or an Ubuntu-terminal-style alignment file (recursive
//! `Orientation`/`Widgets` containers), converted into the CA binary split
//! tree below.

use super::{
    pane_count, validate_layout, write_layout_file_atomic, TerminalGridCanvas,
    TerminalGridLayoutFile, MAX_LAYOUT_BYTES,
};
use std::path::PathBuf;

/// Validates a user-chosen layout file path for export/import.
/// `for_export` appends a missing `.json` extension and requires the parent
/// directory to exist; otherwise the path must be an existing file.
fn validate_layout_file_path(raw: &str, for_export: bool) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Layout file path cannot be empty".into());
    }
    if trimmed.contains('\0') {
        return Err("Layout file path contains invalid characters".into());
    }
    let mut path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err("Layout file path must be absolute".into());
    }
    if for_export {
        let has_json_ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("json"))
            .unwrap_or(false);
        if !has_json_ext {
            path.set_extension("json");
        }
        let parent = path
            .parent()
            .ok_or_else(|| "Layout file path has no parent directory".to_string())?;
        if !parent.is_dir() {
            return Err(format!("Directory '{}' does not exist", parent.display()));
        }
    } else if !path.is_file() {
        return Err(format!("Layout file '{}' not found", path.display()));
    }
    Ok(path)
}

/// Default harness for panes imported from Ubuntu-terminal-style layout
/// files, which carry no harness identity of their own.
const UBUNTU_IMPORT_HARNESS: &str = "claude";

/// Ubuntu-terminal-style layout files (e.g. ~/Templates/*_terminals.json)
/// use n-ary containers instead of the CA binary split tree:
/// `{ "Orientation": "Horizontal" | "Vertical", "Widgets": [ ... ] }`
/// with leaves carrying `Columns`/`Lines`/`Command`/`SessionRestoreId`/
/// `WorkingDirectory`. Only `WorkingDirectory` maps onto the CA model, as
/// the per-pane `cwd` override; harness panes have no equivalent for
/// `Columns`/`Lines`/`Command`/`SessionRestoreId`, so those are ignored.
fn ubuntu_widget_to_layout_node(
    value: &serde_json::Value,
    id_counter: &mut u64,
) -> Result<serde_json::Value, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Ubuntu layout widget must be a JSON object".to_string())?;

    if object.contains_key("Widgets") {
        let orientation = object
            .get("Orientation")
            .and_then(|o| o.as_str())
            .ok_or_else(|| "Ubuntu layout container is missing 'Orientation'".to_string())?;
        let direction = match orientation {
            "Horizontal" => "row",
            "Vertical" => "col",
            other => {
                return Err(format!(
                    "Ubuntu layout container has unknown Orientation '{other}'"
                ))
            }
        };
        let widgets = object
            .get("Widgets")
            .and_then(|w| w.as_array())
            .ok_or_else(|| "Ubuntu layout container 'Widgets' must be an array".to_string())?;
        if widgets.is_empty() {
            return Err("Ubuntu layout container has no widgets".into());
        }
        return fold_ubuntu_widgets(widgets, direction, id_counter);
    }

    let working_directory = object
        .get("WorkingDirectory")
        .and_then(|wd| wd.as_str())
        .ok_or_else(|| {
            "Ubuntu layout widget is neither a container nor a terminal leaf (no 'WorkingDirectory')"
                .to_string()
        })?;
    *id_counter += 1;
    Ok(serde_json::json!({
        "type": "leaf",
        "id": format!("leaf-import-{id_counter}"),
        "harness": UBUNTU_IMPORT_HARNESS,
        "cwd": working_directory,
    }))
}

/// Folds an n-ary Ubuntu container into the binary CA split tree, sharing
/// space equally between children (ratio 1/n at each level).
fn fold_ubuntu_widgets(
    widgets: &[serde_json::Value],
    direction: &str,
    id_counter: &mut u64,
) -> Result<serde_json::Value, String> {
    let first = ubuntu_widget_to_layout_node(&widgets[0], id_counter)?;
    if widgets.len() == 1 {
        return Ok(first);
    }
    *id_counter += 1;
    Ok(serde_json::json!({
        "type": "split",
        "id": format!("split-import-{id_counter}"),
        "direction": direction,
        "ratio": 1.0 / widgets.len() as f64,
        "first": first,
        "second": fold_ubuntu_widgets(&widgets[1..], direction, id_counter)?,
    }))
}

/// Exports a layout to a user-chosen file (e.g. a `~/Templates/*.json`
/// alignment file), outside the CA_HOME named-layout store. Format is the
/// same version-1 JSON; the layout `name` becomes the file stem.
pub fn hub_export_terminal_grid_layout_blocking(
    path: String,
    layout: serde_json::Value,
    canvas: Option<TerminalGridCanvas>,
) -> Result<TerminalGridLayoutFile, String> {
    let target_path = validate_layout_file_path(&path, true)?;
    validate_layout(&layout, &canvas)?;
    let name = target_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Layout file path has no usable file name".to_string())?
        .to_string();

    let file_data = TerminalGridLayoutFile {
        version: 1,
        name,
        saved_at: chrono::Utc::now().to_rfc3339(),
        canvas,
        layout,
    };
    let serialized = serde_json::to_string_pretty(&file_data)
        .map_err(|e| format!("failed to serialize layout JSON: {e}"))?;
    write_layout_file_atomic(&target_path, &serialized, false)?;
    Ok(file_data)
}

/// Imports a layout from a user-chosen JSON file. Accepts both the CA
/// version-1 format and Ubuntu-terminal-style alignment files with
/// recursive `Orientation`/`Widgets` containers (see
/// `ubuntu_widget_to_layout_node`).
pub fn hub_import_terminal_grid_layout_blocking(
    path: String,
) -> Result<TerminalGridLayoutFile, String> {
    let source_path = validate_layout_file_path(&path, false)?;
    let content = std::fs::read_to_string(&source_path)
        .map_err(|e| format!("failed to read layout file: {e}"))?;
    if content.len() > MAX_LAYOUT_BYTES {
        return Err("Terminal grid layout file cannot exceed 256 KiB".into());
    }
    let name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("imported-layout")
        .to_string();

    // CA format v1: `{ version, name, savedAt, canvas, layout }`.
    if let Ok(data) = serde_json::from_str::<TerminalGridLayoutFile>(&content) {
        if pane_count(&data.layout).is_none() {
            return Err("Saved layout has an invalid tree shape".into());
        }
        return Ok(data);
    }

    // Ubuntu-terminal-style alignment: `{ "Orientation": ..., "Widgets": [...] }`.
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("failed to parse layout JSON: {e}"))?;
    if parsed.get("Widgets").is_some() {
        let mut id_counter = 0u64;
        let layout = ubuntu_widget_to_layout_node(&parsed, &mut id_counter)?;
        if pane_count(&layout).is_none() {
            return Err("Converted Ubuntu layout has an invalid tree shape".into());
        }
        return Ok(TerminalGridLayoutFile {
            version: 1,
            name,
            saved_at: chrono::Utc::now().to_rfc3339(),
            canvas: None,
            layout,
        });
    }

    Err("Unrecognized layout file format (expected a Coding Assistants grid layout or an Ubuntu terminal alignment)".into())
}

#[tauri::command]
pub async fn hub_export_terminal_grid_layout(
    path: String,
    layout: serde_json::Value,
    canvas: Option<TerminalGridCanvas>,
) -> Result<TerminalGridLayoutFile, String> {
    tauri::async_runtime::spawn_blocking(move || {
        hub_export_terminal_grid_layout_blocking(path, layout, canvas)
    })
    .await
    .map_err(|e| format!("hub_export_terminal_grid_layout worker panic: {e}"))?
}

#[tauri::command]
pub async fn hub_import_terminal_grid_layout(
    path: String,
) -> Result<TerminalGridLayoutFile, String> {
    tauri::async_runtime::spawn_blocking(move || hub_import_terminal_grid_layout_blocking(path))
        .await
        .map_err(|e| format!("hub_import_terminal_grid_layout worker panic: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_workdir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "hub-terminal-grid-io-{prefix}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn ubuntu_leaf(cwd: &str) -> serde_json::Value {
        serde_json::json!({
            "Columns": 60,
            "Command": "",
            "Lines": 10,
            "SessionRestoreId": 0,
            "WorkingDirectory": cwd,
        })
    }

    fn collect_leaf_ids(layout: &serde_json::Value, out: &mut Vec<String>) {
        if layout["type"] == "leaf" {
            out.push(layout["id"].as_str().unwrap().to_string());
        } else {
            collect_leaf_ids(&layout["first"], out);
            collect_leaf_ids(&layout["second"], out);
        }
    }

    fn collect_leaf_harness_cwd(layout: &serde_json::Value, out: &mut Vec<(String, String)>) {
        if layout["type"] == "leaf" {
            out.push((
                layout["harness"].as_str().unwrap().to_string(),
                layout["cwd"].as_str().unwrap().to_string(),
            ));
        } else {
            collect_leaf_harness_cwd(&layout["first"], out);
            collect_leaf_harness_cwd(&layout["second"], out);
        }
    }

    #[test]
    fn test_export_import_round_trip_to_user_path() {
        let dir = temp_workdir("export");
        let target = dir.join("my-alignment.json");
        let layout_json = serde_json::json!({
            "type": "split",
            "id": "split-1",
            "direction": "row",
            "ratio": 0.5,
            "first": { "type": "leaf", "id": "leaf-1", "harness": "claude", "cwd": "/tmp/proj-a" },
            "second": { "type": "leaf", "id": "leaf-2", "harness": "gemini" }
        });

        let exported = hub_export_terminal_grid_layout_blocking(
            target.to_string_lossy().into_owned(),
            layout_json.clone(),
            Some(TerminalGridCanvas {
                width: 1280.0,
                height: 720.0,
            }),
        )
        .unwrap();
        assert_eq!(exported.name, "my-alignment");
        assert!(target.is_file());

        // No temp files left behind.
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with('.'))
            .collect();
        assert!(leftovers.is_empty());

        let imported =
            hub_import_terminal_grid_layout_blocking(target.to_string_lossy().into_owned())
                .unwrap();
        assert_eq!(imported.layout, layout_json);
        assert_eq!(imported.canvas, exported.canvas);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_appends_json_extension_and_validates_parent() {
        let dir = temp_workdir("export-ext");
        let layout = serde_json::json!({ "type": "leaf", "id": "l", "harness": "claude" });

        let no_ext = dir.join("alignment");
        let saved = hub_export_terminal_grid_layout_blocking(
            no_ext.to_string_lossy().into_owned(),
            layout.clone(),
            None,
        )
        .unwrap();
        assert_eq!(saved.name, "alignment");
        assert!(dir.join("alignment.json").is_file());

        let missing_parent = dir.join("nope").join("x.json");
        let err = hub_export_terminal_grid_layout_blocking(
            missing_parent.to_string_lossy().into_owned(),
            layout,
            None,
        )
        .unwrap_err();
        assert!(err.contains("does not exist"), "unexpected error: {err}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_rejects_relative_and_missing_paths() {
        let err = hub_import_terminal_grid_layout_blocking("relative.json".into()).unwrap_err();
        assert!(err.contains("absolute"), "unexpected error: {err}");

        let err =
            hub_import_terminal_grid_layout_blocking("/no/such/file.json".into()).unwrap_err();
        assert!(err.contains("not found"), "unexpected error: {err}");
    }

    #[test]
    fn test_import_ubuntu_alignment_nested() {
        // Mirrors the real ~/Templates/project_dark_star_terminals.json shape:
        // a 2x2 band, another 2x2 band, then a vertical band of three 2x1 rows.
        let cwd = "/home/pkhunter/Repositories/Game/Project-Dark-Star";
        let row = |n: usize| {
            serde_json::json!({
                "Orientation": "Horizontal",
                "Widgets": (0..n).map(|_| ubuntu_leaf(cwd)).collect::<Vec<_>>(),
            })
        };
        let ubuntu = serde_json::json!({
            "Orientation": "Vertical",
            "Widgets": [
                row(2),
                row(2),
                { "Orientation": "Vertical", "Widgets": [row(2), row(2), row(2)] },
            ],
        });
        let dir = temp_workdir("ubuntu");
        let file = dir.join("dark-star.json");
        std::fs::write(&file, serde_json::to_string_pretty(&ubuntu).unwrap()).unwrap();

        let imported =
            hub_import_terminal_grid_layout_blocking(file.to_string_lossy().into_owned()).unwrap();
        assert_eq!(imported.name, "dark-star");
        assert_eq!(imported.canvas, None);

        let layout = &imported.layout;
        assert_eq!(pane_count(layout), Some(10));
        // Root is Vertical (stacked) with three equal shares.
        assert_eq!(layout["type"], "split");
        assert_eq!(layout["direction"], "col");
        assert!((layout["ratio"].as_f64().unwrap() - 1.0 / 3.0).abs() < 1e-9);
        // First band is Horizontal (side-by-side).
        assert_eq!(layout["first"]["direction"], "row");
        assert!((layout["first"]["ratio"].as_f64().unwrap() - 0.5).abs() < 1e-9);

        let mut ids = Vec::new();
        collect_leaf_ids(layout, &mut ids);
        assert_eq!(ids.len(), 10);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10, "leaf ids must be unique");

        let mut leaf_harness_cwd = Vec::new();
        collect_leaf_harness_cwd(layout, &mut leaf_harness_cwd);
        assert_eq!(leaf_harness_cwd.len(), 10);
        assert!(leaf_harness_cwd.iter().all(|(harness, leaf_cwd)| harness
            == UBUNTU_IMPORT_HARNESS
            && leaf_cwd.as_str() == cwd));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_ubuntu_single_child_container_collapses() {
        let ubuntu = serde_json::json!({
            "Orientation": "Horizontal",
            "Widgets": [ubuntu_leaf("/tmp/only")],
        });
        let dir = temp_workdir("ubuntu-single");
        let file = dir.join("single.json");
        std::fs::write(&file, ubuntu.to_string()).unwrap();

        let imported =
            hub_import_terminal_grid_layout_blocking(file.to_string_lossy().into_owned()).unwrap();
        assert_eq!(imported.layout["type"], "leaf");
        assert_eq!(imported.layout["cwd"], "/tmp/only");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_rejects_malformed_ubuntu_and_unknown_formats() {
        let dir = temp_workdir("ubuntu-bad");
        let cases: Vec<(serde_json::Value, &str)> = vec![
            (
                serde_json::json!({ "Orientation": "Diagonal", "Widgets": [ubuntu_leaf("/tmp")] }),
                "unknown Orientation",
            ),
            (
                serde_json::json!({ "Orientation": "Vertical", "Widgets": [] }),
                "no widgets",
            ),
            (
                serde_json::json!({ "Orientation": "Vertical", "Widgets": [ { "Columns": 1 } ] }),
                "neither a container nor a terminal leaf",
            ),
            (
                serde_json::json!({ "hello": "world" }),
                "Unrecognized layout file format",
            ),
        ];
        for (idx, (value, expected)) in cases.into_iter().enumerate() {
            let file = dir.join(format!("bad-{idx}.json"));
            std::fs::write(&file, value.to_string()).unwrap();
            let err = hub_import_terminal_grid_layout_blocking(file.to_string_lossy().into_owned())
                .unwrap_err();
            assert!(
                err.contains(expected),
                "case {idx}: expected error containing '{expected}', got '{err}'"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_rejects_oversized_file() {
        let dir = temp_workdir("oversized");
        let file = dir.join("big.json");
        std::fs::write(
            &file,
            format!("{{\"blob\": \"{}\"}}", "x".repeat(MAX_LAYOUT_BYTES)),
        )
        .unwrap();
        let err = hub_import_terminal_grid_layout_blocking(file.to_string_lossy().into_owned())
            .unwrap_err();
        assert!(err.contains("256 KiB"), "unexpected error: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_rejects_ca_file_with_invalid_tree() {
        let dir = temp_workdir("ca-bad-tree");
        let file = dir.join("broken.json");
        std::fs::write(
            &file,
            r#"{ "version": 1, "name": "broken", "savedAt": "2026-01-01T00:00:00Z", "canvas": null, "layout": { "type": "split", "id": "x" } }"#,
        )
        .unwrap();
        let err = hub_import_terminal_grid_layout_blocking(file.to_string_lossy().into_owned())
            .unwrap_err();
        assert!(
            err.contains("invalid tree shape"),
            "unexpected error: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
