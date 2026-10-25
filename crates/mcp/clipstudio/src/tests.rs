use super::*;
use mcp_core::ToolProvider;
use serde_json::json;
use std::fs;

fn bridge(config: Config) -> ClipStudio {
    ClipStudio::new(Some(config))
}

fn sample_config() -> (tempfile::TempDir, Config) {
    let dir = tempfile::tempdir().unwrap();
    for name in ["input", "working", "exports"] {
        fs::create_dir(dir.path().join(name)).unwrap();
    }
    let raw = json!({
        "input": dir.path().join("input"),
        "working": dir.path().join("working"),
        "exports": dir.path().join("exports"),
        "commands": ["document.save_as_dialog"],
        "auto_actions": [],
    });
    let path = dir.path().join("config.json");
    fs::write(&path, raw.to_string()).unwrap();
    (dir, load_config(&path).unwrap())
}

fn text(result: ToolResult) -> String {
    match result {
        ToolResult::Ok(text) | ToolResult::Err(text) => text,
    }
}

#[test]
fn catalog_has_the_thirty_three_upstream_names() {
    let bridge = ClipStudio::new(None);
    let names: Vec<_> = bridge
        .tools()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names.len(), 33);
    assert!(names.contains(&"csp_health".to_string()));
    assert!(names.contains(&"csp_native_file_dialog".to_string()));
    assert_eq!(
        bridge.server_info().name,
        "coding-assistants-mcp-clipstudio"
    );
}

#[test]
fn preflight_token_is_single_use_and_input_is_not_sent() {
    let bridge = ClipStudio::new(None);
    let params = json!({ "x": 0.5, "y": 0.25, "button": "left" });
    let issued = bridge.call(
        "csp_preflight",
        &json!({
            "operation": "input",
            "identifier": "csp_click",
            "parameters_json": params.to_string(),
        }),
    );
    let issued = match issued {
        ToolResult::Ok(body) => serde_json::from_str::<Value>(&body).unwrap(),
        ToolResult::Err(err) => panic!("{err}"),
    };
    let token = issued["confirmToken"].as_str().unwrap();
    let call = json!({
        "captureId": "cap-1",
        "confirmToken": token,
        "x": 0.5,
        "y": 0.25,
        "button": "left",
    });
    let first = text(bridge.call("csp_click", &call));
    assert!(first.contains("does not send pointer"));
    let second = text(bridge.call("csp_click", &call));
    assert!(second.contains("was not issued"));
}

#[test]
fn mismatched_parameters_consume_the_token() {
    let bridge = ClipStudio::new(None);
    let issued = match bridge.call(
        "csp_preflight",
        &json!({
            "operation": "input",
            "identifier": "csp_scroll",
            "parameters_json": "{\"steps\":1}",
        }),
    ) {
        ToolResult::Ok(body) => serde_json::from_str::<Value>(&body).unwrap(),
        ToolResult::Err(err) => panic!("{err}"),
    };
    let token = issued["confirmToken"].as_str().unwrap();
    let wrong = text(bridge.call(
        "csp_scroll",
        &json!({ "captureId": "cap", "confirmToken": token, "steps": 2 }),
    ));
    assert!(wrong.contains("does not match"));
    let again = text(bridge.call(
        "csp_scroll",
        &json!({ "captureId": "cap", "confirmToken": token, "steps": 1 }),
    ));
    assert!(again.contains("was not issued"));
}

#[test]
fn file_tools_stay_inside_the_roots() {
    let (_dir, config) = sample_config();
    let input = config.input.clone();
    fs::write(input.join("page.png"), b"png").unwrap();
    let bridge = bridge(config);
    let listed = match bridge.call("csp_list_assets", &json!({ "root": "input" })) {
        ToolResult::Ok(body) => body,
        ToolResult::Err(err) => panic!("{err}"),
    };
    assert!(listed.contains("page.png"));
    let outside = text(bridge.call(
        "csp_validate_path",
        &json!({ "path": "/etc/passwd", "intent": "read" }),
    ));
    assert!(outside.contains("outside"));
    let escape = text(bridge.call(
        "csp_validate_path",
        &json!({ "path": format!("{}/../secret", input.display()), "intent": "read" }),
    ));
    assert!(escape.contains("parent"));
    let write = text(bridge.call(
        "csp_validate_path",
        &json!({ "path": input.join("page.png").display().to_string(), "intent": "write" }),
    ));
    assert!(write.contains("read-only"));
}

#[test]
fn symlink_and_open_document_fail_closed() {
    let (_dir, config) = sample_config();
    let real = config.input.join("real.png");
    fs::write(&real, b"real").unwrap();
    let link = config.input.join("link.png");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let bridge = bridge(config.clone());
    #[cfg(unix)]
    {
        let rejected = text(bridge.call(
            "csp_file_info",
            &json!({ "path": link.display().to_string() }),
        ));
        assert!(rejected.contains("reparse"));
    }
    let opened = match bridge.call(
        "csp_open_document",
        &json!({ "path": real.display().to_string() }),
    ) {
        ToolResult::Ok(body) => body,
        ToolResult::Err(err) => panic!("{err}"),
    };
    assert!(opened.contains("\"dispatched\":false"));
    let info = match bridge.call(
        "csp_file_info",
        &json!({ "path": real.display().to_string() }),
    ) {
        ToolResult::Ok(body) => body,
        ToolResult::Err(err) => panic!("{err}"),
    };
    assert!(info.contains("sha256"));
}

#[test]
fn session_inspection_does_not_pretend_to_capture() {
    let bridge = ClipStudio::new(None);
    let err = text(bridge.call("csp_capture", &json!({})));
    assert!(err.contains("does not send pointer"));
    let health = match bridge.call("csp_health", &json!({})) {
        ToolResult::Ok(body) => body,
        ToolResult::Err(err) => panic!("{err}"),
    };
    assert!(health.contains("\"session_attested\":false"));
}
