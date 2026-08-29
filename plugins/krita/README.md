# Coding-Assistants Krita bridge

Two halves (mirrors `plugins/blender/`):

- **`coding_assistants_bridge.py`** + **`.desktop`** — a PyKrita plugin.
  Opens a localhost line-JSON TCP server (default port **9766**).
- **`crates/mcp-krita`** (`coding-assistants-mcp-krita`) — the MCP server
  an agent's config points at. Connects to the plugin per tool call.

```
agent ──stdio MCP──► coding-assistants-mcp-krita ──TCP line-JSON──► Krita plugin ──► krita API
```

## Install the plugin

PyKrita loads plugins from your resources folder's `pykrita/` subdir
(Settings → Manage Resources → **Open Resources Folder**). Copy both files
into it so you have:

```
pykrita/coding_assistants_bridge.py
pykrita/coding_assistants_bridge.desktop
```

Then Settings → Configure Krita → **Python Plugin Manager** → tick
**Coding-Assistants Bridge**, and restart Krita. The console
(`--pykrita-log` or the Scripter output) shows
`Krita bridge listening on 127.0.0.1:9766`.

The port is fixed at 9766 in this version; change `DEFAULT_PORT` in the
plugin and pass a matching `--port` if you need a different one.

## Register the MCP server

`coding-assistants-mcp-krita [--port N] [--allow-run-python]`

`hub::mcp` renders this into each client's config. A Claude `.mcp.json`:

```json
{ "mcpServers": { "coding-assistants-mcp-krita": {
    "command": "/path/to/coding-assistants-mcp-krita",
    "args": ["--port", "9766"]
} } }
```

`--allow-run-python` adds a `run_python` tool (arbitrary `krita` scripting).
**Off by default.**

## Tools

| Tool | Effect |
|---|---|
| `get_document_summary` | name, size, color model/depth, resolution, layer count |
| `list_layers` | top-level layers: `{ name, type, visible, opacity }` |
| `create_document` | new doc (width, height, name, color_model, resolution), made active |
| `create_paint_layer` | add a paint layer above the active node |
| `set_layer_visible` | show/hide a layer by name |
| `set_layer_opacity` | 0–100 by name |
| `export_document` | `.png/.jpg/.webp/.tiff/.kra` |
| `run_python` | *(gated)* run a `krita` snippet; stdout + `repr(result)` |

## Smoke check

Krita has no headless scripting mode comparable to `blender --background`,
so there is no unattended smoke script. To verify manually: install the
plugin, open a document, then from another terminal
`printf '{"op":"get_document_summary","args":{}}\n' | nc 127.0.0.1 9766`.
