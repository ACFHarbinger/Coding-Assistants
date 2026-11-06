use super::{ServerInfo, ToolProvider, ToolResult};
use hub::{default_hub_home, HubStore, MemoryScope, MemoryTier};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Shared `remember` / `recall` capability for a creative-tool MCP server.
/// A workspace is explicit: omitting it intentionally writes and searches the
/// global scope, rather than guessing from the server process' working dir.
pub struct MemoryTools {
    tool: &'static str,
    workspace: Option<String>,
    hub_home: PathBuf,
}

impl MemoryTools {
    pub fn new(tool: &'static str, workspace: Option<String>) -> Self {
        Self::with_hub_home(tool, workspace, default_hub_home())
    }

    pub fn with_hub_home(tool: &'static str, workspace: Option<String>, hub_home: PathBuf) -> Self {
        Self {
            tool,
            workspace: workspace.and_then(|path| {
                if path.trim().is_empty() {
                    None
                } else if PathBuf::from(&path).is_absolute() {
                    Some(path)
                } else {
                    eprintln!("ignoring non-absolute MCP workspace path {path:?}");
                    None
                }
            }),
            hub_home,
        }
    }

    pub fn tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "remember",
                "description": "Save a short-term memory scoped to this creative tool and workspace.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "body": { "type": "string", "description": "Memory text to retain." },
                        "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional searchable tags." }
                    },
                    "required": ["body"]
                }
            }),
            json!({
                "name": "recall",
                "description": "Search memories previously saved by this creative-tool server.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 20, "default": 5 }
                    },
                    "required": ["query"]
                }
            }),
        ]
    }

    pub fn call(&self, name: &str, arguments: &Value) -> Option<ToolResult> {
        match name {
            "remember" => Some(self.remember(arguments)),
            "recall" => Some(self.recall(arguments)),
            _ => None,
        }
    }

    fn open(&self) -> Result<HubStore, String> {
        HubStore::open(&self.hub_home).map_err(|error| format!("memory store unavailable: {error}"))
    }

    fn remember(&self, arguments: &Value) -> ToolResult {
        let Some(body) = arguments.get("body").and_then(Value::as_str) else {
            return ToolResult::Err("remember requires a string body".into());
        };
        let tags = match tags(arguments) {
            Ok(tags) => tags,
            Err(error) => return ToolResult::Err(error),
        };
        let scope = if self.workspace.is_some() {
            MemoryScope::Workspace
        } else {
            MemoryScope::Global
        };
        match self.open().and_then(|store| {
            store
                .write_memory_with_tool(
                    MemoryTier::ShortTerm,
                    scope,
                    Some(&format!("mcp-{}", self.tool)),
                    self.workspace.as_deref(),
                    None,
                    body,
                    &tags,
                    Some(self.tool),
                )
                .map_err(|error| error.to_string())
        }) {
            Ok(memory) => ToolResult::Ok(format!("Remembered {} ({})", memory.id, scope.as_str())),
            Err(error) => ToolResult::Err(format!("could not remember: {error}")),
        }
    }

    fn recall(&self, arguments: &Value) -> ToolResult {
        let Some(query) = arguments.get("query").and_then(Value::as_str) else {
            return ToolResult::Err("recall requires a string query".into());
        };
        let limit = match limit(arguments) {
            Ok(limit) => limit,
            Err(error) => return ToolResult::Err(error),
        };
        match self.open().and_then(|store| {
            store
                .search_memories_hybrid_with_tool(
                    query,
                    limit,
                    None,
                    None,
                    self.workspace.as_deref(),
                    Some(self.tool),
                )
                .map_err(|error| error.to_string())
        }) {
            Ok(memories) if memories.is_empty() => ToolResult::Ok("No matching memories.".into()),
            Ok(memories) => ToolResult::Ok(
                memories
                    .into_iter()
                    .map(render_memory)
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            ),
            Err(error) => ToolResult::Err(format!("could not recall: {error}")),
        }
    }
}

/// Adds shared memory tools to an otherwise application-specific provider.
pub struct MemoryProvider<P> {
    provider: P,
    memory: MemoryTools,
}

impl<P> MemoryProvider<P> {
    pub fn new(provider: P, memory: MemoryTools) -> Self {
        Self { provider, memory }
    }
}

impl<P: ToolProvider> ToolProvider for MemoryProvider<P> {
    fn server_info(&self) -> ServerInfo {
        self.provider.server_info()
    }

    fn tools(&self) -> Vec<Value> {
        let mut tools = self.provider.tools();
        tools.extend(self.memory.tools());
        tools
    }

    fn call(&self, name: &str, arguments: &Value) -> ToolResult {
        self.memory
            .call(name, arguments)
            .unwrap_or_else(|| self.provider.call(name, arguments))
    }

    fn extra_capabilities(&self) -> Value {
        self.provider.extra_capabilities()
    }

    fn on_notification(&self, method: &str, params: &Value, emitter: &super::Emitter) {
        self.provider.on_notification(method, params, emitter);
    }
}

fn tags(arguments: &Value) -> Result<Vec<String>, String> {
    arguments.get("tags").map_or(Ok(Vec::new()), |tags| {
        tags.as_array()
            .ok_or_else(|| "remember tags must be an array of strings".to_string())?
            .iter()
            .map(|tag| {
                tag.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| "remember tags must be an array of strings".to_string())
            })
            .collect()
    })
}

fn limit(arguments: &Value) -> Result<usize, String> {
    match arguments.get("limit").and_then(Value::as_u64) {
        None => Ok(5),
        Some(value @ 1..=20) => Ok(value as usize),
        Some(_) => Err("recall limit must be an integer between 1 and 20".into()),
    }
}

fn render_memory((memory, score): (hub::MemoryRecord, f32)) -> String {
    let title = memory.title.unwrap_or_else(|| "(untitled)".into());
    format!("{} | {title} | {:.3}\n{}", memory.id, score, memory.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remember_and_recall_are_isolated_by_tool_and_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let blender =
            MemoryTools::with_hub_home("blender", Some("/project".into()), dir.path().into());
        let krita = MemoryTools::with_hub_home("krita", Some("/project".into()), dir.path().into());

        assert!(matches!(
            blender.call(
                "remember",
                &json!({"body": "Use Eevee for previews", "tags": ["render"]})
            ),
            Some(ToolResult::Ok(_))
        ));
        assert!(
            matches!(blender.call("recall", &json!({"query": "preview rendering"})), Some(ToolResult::Ok(text)) if text.contains("Eevee"))
        );
        assert!(
            matches!(krita.call("recall", &json!({"query": "preview rendering"})), Some(ToolResult::Ok(text)) if text == "No matching memories.")
        );
    }
}
