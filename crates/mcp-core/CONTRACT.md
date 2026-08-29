# `mcp-core` — the contract every Coding-Assistants MCP bridge follows

`mcp-core` is the shared stdio MCP server. A bridge crate (`crates/mcp-<tool>`,
and `crates/claude`) implements one trait and hands it to `McpServer`.

## Transport

Newline-delimited JSON on stdio. One JSON-RPC object per line, `\n`-terminated,
stdout flushed after every write. This is what an MCP client's `command`-type
server entry speaks. `mcp-core` handles:

| Inbound method | Handled by |
|---|---|
| `initialize` | `mcp-core` — replies with `MCP_PROTOCOL_VERSION`, `serverInfo` from `server_info()`, and `capabilities` = `{ "tools": {} }` merged with `extra_capabilities()` |
| `tools/list` | `mcp-core` — replies with `tools()` verbatim |
| `tools/call` | `mcp-core` validates `params.name` against `tools()`, then calls `call(name, arguments)`; wraps `ToolResult` in a `result.content` array (`Err` → `isError: true`) |
| anything else | forwarded to `on_notification(method, params, emitter)` |

An unknown tool name → JSON-RPC `error` `-32601` (protocol fault). A tool that
runs but fails → `ToolResult::Err` → a normal `result` with `isError: true`
(tool fault). Keep that distinction.

## The trait

```rust
impl ToolProvider for MyBridge {
    fn server_info(&self) -> ServerInfo { /* name + THIS crate's CARGO_PKG_VERSION */ }
    fn tools(&self) -> Vec<Value>       { /* [{ name, description, inputSchema }, ...] */ }
    fn call(&self, name: &str, args: &Value) -> ToolResult { /* touch the app, return text */ }

    // optional:
    fn extra_capabilities(&self) -> Value { json!({}) }          // merged into initialize
    fn on_notification(&self, method: &str, params: &Value, emitter: &Emitter) {}
}
```

`call` is the only method that may do I/O. It still returns a value rather than
writing the wire, so it is unit-testable directly (see `mcp-echo` and
`crates/claude`'s `server::tests`).

## Running it

```rust
let server = McpServer::new(Arc::new(MyBridge::new()));
let emitter = server.emitter();                 // clone BEFORE run() for a push thread
std::thread::spawn(move || push_loop(emitter)); // optional
server.run();                                    // blocks on stdin until EOF
```

`Emitter` is `Clone`; `Emitter::notify(method, params)` writes a JSON-RPC
notification (no `id`). Use it for proactive pushes (`crates/claude`'s
`poll_loop` pushes `notifications/claude/channel` this way).

## App-side transport (for real tool bridges)

`mcp-echo` echoes in-process. A real bridge's `call` connects to a **running
instance of the target application** over a localhost line-JSON channel that the
app's native plugin opens (TCP or Unix socket, one JSON request/response per
line). One contract, one implementation per app language:

- `plugins/blender/` — Python addon, `bpy`, opens the socket
- `plugins/krita/` — PyKrita `Extension`
- `plugins/godot/` — `EditorPlugin` (GDScript)
- `plugins/aseprite/` — Lua (request/response per `aseprite -b` invocation; no
  long-lived socket)
- `plugins/unreal/` — in-editor Python
- `plugins/unity/` — C# editor package

Each `plugins/<tool>/` carries its own README with install steps and a smoke
script. The Rust side (`crates/mcp-<tool>`) owns the tool schemas and the
socket-client half.

## Config delivery

Bridges are registered as app-managed MCP servers via `hub::mcp` (see the
program design doc `.agent/reports/chat/memory_and_creative_mcp_program_20260829.md`,
track C-1b) and rendered into each client's config
(`<workspace>/.mcp.json`, Codex `config.toml` `[mcp_servers]`, Gemini
`settings.json`, `opencode.json`). A server entry is
`{ "command": "<abs path to coding-assistants-mcp-<tool>>", "args": [...] }`.
