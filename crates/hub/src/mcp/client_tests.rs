use super::*;
use serde_json::json;
use std::io::Cursor;
use std::path::PathBuf;

fn live_tool_list() -> Value {
    json!({
        "tools": [
            {
                "name": "perplexity_ask",
                "description": "Answer a question",
                "inputSchema": {
                    "type": "object",
                    "properties": { "query": { "type": "string" } },
                    "required": ["query"]
                }
            },
            { "name": "no_schema" }
        ]
    })
}

#[test]
fn parse_tools_keeps_name_description_and_schema() {
    let tools = parse_tools(&live_tool_list()).unwrap();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "perplexity_ask");
    assert_eq!(tools[0].description.as_deref(), Some("Answer a question"));
    assert_eq!(
        tools[0].input_schema["properties"]["query"]["type"],
        "string"
    );
    assert_eq!(tools[1].name, "no_schema");
    assert_eq!(tools[1].input_schema["type"], "object");
}

#[test]
fn call_result_joins_text_parts_and_flags_tool_errors() {
    let ok = call_result_from(&json!({
        "content": [
            { "type": "text", "text": "hello" },
            { "type": "image", "data": "xxxx" },
            { "type": "text", "text": "world" }
        ]
    }));
    assert!(!ok.is_error);
    assert_eq!(ok.text, "hello\nworld");

    let err = call_result_from(&json!({
        "isError": true,
        "content": [{ "type": "text", "text": "missing query" }]
    }));
    assert!(err.is_error);
    assert_eq!(err.text, "missing query");
}

#[test]
fn read_message_accepts_ndjson_and_content_length() {
    let ndjson = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}\n";
    let value = read_message(&mut Cursor::new(&ndjson[..])).unwrap();
    assert_eq!(value["id"], 1);

    let framed = b"Content-Length: 24\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":2}\n";
    let value = read_message(&mut Cursor::new(&framed[..])).unwrap();
    assert_eq!(value["id"], 2);
}

fn write_fake_server(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("fake_mcp.py");
    std::fs::write(
        &path,
        r#"#!/usr/bin/env python3
import json, sys
buf = sys.stdin.buffer

def read_msg():
    headers = {}
    while True:
        line = buf.readline()
        if not line:
            return None
        if line in (b"\n", b"\r\n"):
            if "len" not in headers:
                continue
            break
        lower = line.lower()
        if lower.startswith(b"content-length:"):
            headers["len"] = int(line.split(b":", 1)[1])
        stripped = line.strip()
        if stripped.startswith(b"{"):
            return json.loads(stripped)
    body = buf.read(headers["len"])
    return json.loads(body)

def reply(msg):
    body = json.dumps(msg).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(body)}\r\n\r\n".encode())
    sys.stdout.buffer.write(body)
    sys.stdout.buffer.write(b"\n")
    sys.stdout.buffer.flush()

while True:
    msg = read_msg()
    if msg is None:
        break
    method = msg.get("method")
    ident = msg.get("id")
    if method == "initialize":
        reply({"jsonrpc": "2.0", "id": ident, "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "fake", "version": "0"}
        }})
    elif method == "tools/list":
        reply({"jsonrpc": "2.0", "id": ident, "result": {
            "tools": [{"name": "echo", "description": "Echo text",
                       "inputSchema": {"type": "object",
                                       "properties": {"text": {"type": "string"}},
                                       "required": ["text"]}}]
        }})
    elif method == "tools/call":
        args = (msg.get("params") or {}).get("arguments") or {}
        reply({"jsonrpc": "2.0", "id": ident, "result": {
            "content": [{"type": "text", "text": args.get("text", "")}]
        }})
"#,
    )
    .unwrap();
    path
}

fn fake_entry(script: &std::path::Path) -> McpServerEntry {
    McpServerEntry::new("fake", "python3", &[script.to_str().unwrap()])
}

#[test]
fn spawn_lists_and_calls_a_stdio_server() {
    let dir = tempfile::tempdir().unwrap();
    let script = write_fake_server(dir.path());
    let entry = fake_entry(&script);
    let tools = list_tools(&entry, &[]).expect("list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
    let result = call_tool(&entry, &[], "echo", &json!({ "text": "pong" })).expect("call");
    assert!(!result.is_error);
    assert_eq!(result.text, "pong");
}

#[test]
fn spawn_failure_names_the_command() {
    let entry = McpServerEntry::new("missing", "ca-definitely-not-an-mcp-binary", &[]);
    let err = list_tools(&entry, &[]).unwrap_err();
    assert!(err.contains("ca-definitely-not-an-mcp-binary"), "{err}");
}
