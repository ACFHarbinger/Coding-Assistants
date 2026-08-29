//! The smallest possible `mcp-core` server: one `echo` tool. It exists to
//! (1) prove the transport end-to-end against a real MCP client and (2) be
//! the thing `crates/mcp-<tool>` is copied from. A real tool bridge adds a
//! socket client to the running application in `call`; this one just echoes.

use mcp_core::{McpServer, ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};
use std::sync::Arc;

struct Echo;

impl ToolProvider for Echo {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-echo".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        vec![json!({
            "name": "echo",
            "description": "Return the given text unchanged. A connectivity check.",
            "inputSchema": {
                "type": "object",
                "properties": { "text": { "type": "string", "description": "Text to echo back." } },
                "required": ["text"],
            },
        })]
    }

    fn call(&self, name: &str, arguments: &Value) -> ToolResult {
        match name {
            "echo" => match arguments.get("text").and_then(Value::as_str) {
                Some(text) => ToolResult::Ok(text.to_string()),
                None => ToolResult::Err("echo requires a 'text' string argument".into()),
            },
            other => ToolResult::Err(format!("unknown tool {other}")),
        }
    }
}

fn main() {
    McpServer::new(Arc::new(Echo)).run();
}
