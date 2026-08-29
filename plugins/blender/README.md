# Coding-Assistants Blender bridge

Two halves:

- **`coding_assistants_bridge.py`** — a Blender add-on. Opens a localhost
  line-JSON TCP server (default port **9765**).
- **`crates/mcp-blender`** (`coding-assistants-mcp-blender`) — the MCP
  server an agent's config points at. Connects to the add-on per tool call.

```
agent ──stdio MCP──► coding-assistants-mcp-blender ──TCP line-JSON──► Blender add-on ──► bpy
```

## Install the add-on

1. Blender → Edit → Preferences → Add-ons → **Install…**
2. Pick `plugins/blender/coding_assistants_bridge.py`, then tick it enabled.
3. (Optional) expand its entry to change the **Port** — must match the
   bridge's `--port`. Re-enable the add-on after changing it.

The add-on starts listening as soon as it's enabled and on every Blender
launch. Console shows `Blender bridge listening on 127.0.0.1:9765`.

## Register the MCP server

`coding-assistants-mcp-blender [--port N] [--allow-run-python]`

`hub::mcp` renders this into each client's config (see
`crates/mcp-core/CONTRACT.md`). A Claude `.mcp.json` entry:

```json
{ "mcpServers": { "coding-assistants-mcp-blender": {
    "command": "/path/to/coding-assistants-mcp-blender",
    "args": ["--port", "9765"]
} } }
```

`--allow-run-python` adds a `run_python` tool that executes arbitrary `bpy`
code inside Blender. **Off by default.** Only pass it for a workspace you
trust to drive Blender freely.

## Tools

| Tool | Effect |
|---|---|
| `get_scene_summary` | object count, active object, frame range, engine, units |
| `list_objects` | `[{ name, type, location }]` |
| `create_primitive` | add cube/uv_sphere/ico_sphere/cylinder/cone/plane/torus; returns the new object name |
| `delete_object` | remove by name |
| `export_scene` | `.glb/.gltf/.obj/.fbx/.stl`, optional selection-only |
| `render_still` | render current frame to an image path |
| `run_python` | *(gated)* run a `bpy` snippet; returns stdout + `repr(result)` |

## Smoke check

With Blender on `PATH`:

```bash
blender --background --python plugins/blender/smoke.py
```

It starts the add-on, drives a few ops through a local socket, and prints
`SMOKE OK` / `SMOKE FAILED`. CI does not run this (no Blender on the
runners); run it locally when changing the add-on.
