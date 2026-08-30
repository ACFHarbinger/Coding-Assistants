# Coding-Assistants Godot bridge

Two halves (mirrors `plugins/blender/`, `plugins/krita/`):

- **`addons/coding_assistants_bridge/`** — a Godot 4 editor plugin
  (`EditorPlugin`, GDScript). Opens a localhost line-JSON TCP server
  (port **9767**).
- **`crates/mcp-godot`** (`coding-assistants-mcp-godot`) — the MCP server
  an agent's config points at. Connects to the plugin per tool call.

```
agent ──stdio MCP──► coding-assistants-mcp-godot ──TCP line-JSON──► Godot editor plugin ──► EditorInterface
```

Godot has no in-editor Python, so this half is GDScript. Everything runs on
the editor's main thread, polled from `_process` — no thread marshalling.

## Install the plugin

1. Copy `addons/coding_assistants_bridge/` into your project's
   `res://addons/` folder.
2. Project → Project Settings → **Plugins** → enable **Coding-Assistants
   Bridge**.
3. The Output panel shows `Godot bridge listening on 127.0.0.1:9767`.

Godot 4.x only (uses `TCPServer`, `ClassDB.instantiate`, typed GDScript).
The port is fixed at 9767 in this version — change `PORT` in `plugin.gd`
and pass a matching `--port`.

## Register the MCP server

`coding-assistants-mcp-godot [--port N] [--allow-run-script]`

`hub::mcp` renders this into each client's config. Claude `.mcp.json`:

```json
{ "mcpServers": { "coding-assistants-mcp-godot": {
    "command": "/path/to/coding-assistants-mcp-godot",
    "args": ["--port", "9767"]
} } }
```

`--allow-run-script` adds a `run_gdscript` tool (arbitrary GDScript in the
editor). **Off by default.**

## Tools

| Tool | Effect |
|---|---|
| `get_scene_summary` | edited scene path, root node name/type, node count |
| `list_nodes` | scene tree `[{ name, type, path }]` (path is relative to the root) |
| `add_node` | instance a class under a parent path; owner set so it saves |
| `delete_node` | remove a node (and children) by path |
| `set_node_property` | set one property; `[x,y]` / `[r,g,b,a]` map to Vector/Color |
| `save_scene` | save the edited scene |
| `open_scene` | open a `res://` scene, make it active |
| `list_project_scenes` | all `.tscn` / `.scn` in the project |
| `run_gdscript` | *(gated)* wrap a snippet in a `@tool` `_run()`; returns `str(result)` |

## Smoke check

No headless editor scripting mode is convenient here. Verify manually:
enable the plugin with a scene open, then
`printf '{"op":"get_scene_summary","args":{}}\n' | nc 127.0.0.1 9767`.
