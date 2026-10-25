//! Tool schemas for the 33-name CLIP STUDIO PAINT catalog.

use serde_json::{json, Value};

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required },
    })
}

fn none(name: &str, description: &str) -> Value {
    tool(name, description, json!({}), &[])
}

fn input(name: &str, description: &str, mut properties: Value, mut required: Vec<&str>) -> Value {
    let obj = properties.as_object_mut().expect("properties");
    obj.insert("captureId".into(), json!({ "type": "string" }));
    obj.insert("confirmToken".into(), json!({ "type": "string" }));
    required.insert(0, "confirmToken");
    required.insert(0, "captureId");
    tool(name, description, properties, &required)
}

pub fn schemas() -> Vec<Value> {
    let xy = json!({
        "x": { "type": "number", "minimum": 0, "maximum": 1 },
        "y": { "type": "number", "minimum": 0, "maximum": 1 },
    });
    vec![
        none("csp_health", "Server profile, and whether a CSP session is attested. No window input."),
        none("csp_capabilities", "Each tool's availability and the reason it is closed."),
        none("csp_get_activity", "User activity. Closed without an attested CSP session."),
        none("csp_list_windows", "CSP main windows. Closed without an attested session."),
        none("csp_get_state", "Foreground, window, and dialog identity. Closed without an attested session."),
        none("csp_capture", "Screenshot of the CSP window. Closed without an attested session."),
        tool("csp_compare_captures", "Pixel delta of two captures. Closed without an attested session.", json!({ "before": { "type": "string" }, "after": { "type": "string" } }), &["before", "after"]),
        tool("csp_sample_pixel", "RGBA at a normalized capture point. Closed without an attested session.", json!({ "captureId": { "type": "string" }, "x": { "type": "number" }, "y": { "type": "number" } }), &["captureId", "x", "y"]),
        none("csp_inspect_ui", "Native dialog inspection. Closed without an attested session."),
        none("csp_list_commands", "Command ids from the local config. Empty when unset."),
        none("csp_list_auto_actions", "Auto Action ids from the local config. Empty when unset."),
        none("csp_launch", "Does not start an unattested CLIPStudioPaint.exe."),
        none("csp_focus", "Does not focus a window without an attested session."),
        tool("csp_window_action", "Restore, maximize, or minimize. Closed without an attested session.", json!({ "action": { "type": "string", "enum": ["restore", "maximize", "minimize"] } }), &["action"]),
        tool("csp_wait_visual_change", "Wait for a screenshot delta. Closed without an attested session.", json!({ "threshold": { "type": "number" } }), &[]),
        none("csp_cancel", "Would send one Escape to the attested CSP window. Closed here."),
        input("csp_move_pointer", "Normalized pointer move. Requires a fresh preflight token, then refuses without a session.", xy, vec!["x", "y"]),
        input("csp_click", "Click or double-click. Requires a preflight token, then refuses without a session.", json!({ "x": { "type": "number" }, "y": { "type": "number" }, "button": { "type": "string", "enum": ["left", "right", "middle"] }, "clicks": { "type": "integer", "enum": [1, 2] } }), vec!["x", "y", "button"]),
        input("csp_drag", "Drag between two normalized points. Requires a preflight token, then refuses without a session.", json!({ "x1": { "type": "number" }, "y1": { "type": "number" }, "x2": { "type": "number" }, "y2": { "type": "number" } }), vec!["x1", "y1", "x2", "y2"]),
        input("csp_scroll", "Bounded scroll. Requires a preflight token, then refuses without a session.", json!({ "steps": { "type": "integer", "minimum": -5, "maximum": 5 } }), vec!["steps"]),
        input("csp_type_text", "Unicode text without the clipboard. Requires a preflight token, then refuses without a session.", json!({ "text": { "type": "string" } }), vec!["text"]),
        input("csp_navigation_key", "One navigation key. Requires a preflight token, then refuses without a session.", json!({ "key": { "type": "string", "enum": ["Up", "Down", "Left", "Right", "Home", "End", "PageUp", "PageDown", "Tab"] } }), vec!["key"]),
        input("csp_run_command", "Registered shortcut command. Requires a preflight token, then refuses without a session.", json!({ "commandId": { "type": "string" } }), vec!["commandId"]),
        input("csp_run_auto_action", "Registered Auto Action. Requires a preflight token, then refuses without a session.", json!({ "actionId": { "type": "string" } }), vec!["actionId"]),
        input("csp_run_command_sequence", "Registered non-destructive command ids. Requires a preflight token, then refuses without a session.", json!({ "commandIds": { "type": "array", "items": { "type": "string" } } }), vec!["commandIds"]),
        tool("csp_preflight", "Issue a 45-second single-use token for one atomic input tool and its exact parameters. Does not focus CSP.", json!({ "operation": { "type": "string" }, "identifier": { "type": "string" }, "parameters_json": { "type": "string" } }), &["operation", "identifier", "parameters_json"]),
        tool("csp_validate_path", "Check one absolute path against the three asset roots.", json!({ "path": { "type": "string" }, "intent": { "type": "string", "enum": ["read", "write", "overwrite"] } }), &["path", "intent"]),
        tool("csp_list_assets", "List files under one configured root. Symlinks are skipped.", json!({ "root": { "type": "string", "enum": ["input", "working", "exports"] } }), &["root"]),
        tool("csp_file_info", "Size, mtime, and SHA-256 for a file inside a root.", json!({ "path": { "type": "string" } }), &["path"]),
        tool("csp_scan_exports", "Files in the exports root modified at or after since_unix.", json!({ "since_unix": { "type": "integer", "minimum": 0 } }), &[]),
        tool("csp_wait_for_file", "Wait until a working or exports file is non-empty and unchanged. timeout_ms is capped at 15000.", json!({ "path": { "type": "string" }, "timeout_ms": { "type": "integer", "minimum": 0, "maximum": 15000 } }), &["path"]),
        tool("csp_open_document", "Validate an input-root file. Does not launch CSP.", json!({ "path": { "type": "string" } }), &["path"]),
        input("csp_native_file_dialog", "Fill a verified native file dialog. Requires a preflight token, then refuses without a session.", json!({ "path": { "type": "string" }, "intent": { "type": "string" }, "submit": { "type": "boolean" }, "dialogHwnd": { "type": "string" } }), vec!["path", "intent", "submit", "dialogHwnd"]),
    ]
}
