# Coding-Assistants Aseprite bridge

Aseprite is the odd one out — **no live session, no socket**. Its only
automation surface is batch-mode Lua. So this bridge shells out to
`aseprite -b --script dispatch.lua` once per tool call, and every tool
operates on a **sprite file path**.

```
agent ──stdio MCP──► coding-assistants-mcp-aseprite ──spawns──► aseprite -b --script dispatch.lua
                                                     ◄──stdout JSON line──┘
```

- **`dispatch.lua`** — the script Aseprite runs. Reads flat
  `--script-param` scalars from `app.params`, does one op, prints one
  `{"ok":..., "result"|"error":...}` line (hand-rolled JSON — Aseprite
  bundles no JSON lib).
- **`crates/mcp-aseprite`** (`coding-assistants-mcp-aseprite`) — the MCP
  server.

## Setup

No plugin install. Just have Aseprite available:

```
coding-assistants-mcp-aseprite [--aseprite <path>] [--script <dispatch.lua>] [--allow-apply-script]
```

`--aseprite` defaults to `aseprite` on PATH. `--script` defaults to a
`dispatch.lua` next to the binary, then `plugins/aseprite/dispatch.lua`
relative to the working directory — so ship `dispatch.lua` alongside the
binary when packaging.

`hub::mcp` renders the server into each client's config. Claude `.mcp.json`:

```json
{ "mcpServers": { "coding-assistants-mcp-aseprite": {
    "command": "/path/to/coding-assistants-mcp-aseprite",
    "args": ["--script", "/path/to/dispatch.lua"]
} } }
```

`--allow-apply-script` adds `apply_script` (arbitrary Aseprite Lua against
a sprite). **Off by default.**

## Tools

| Tool | Effect |
|---|---|
| `sprite_info` | dimensions, color mode, frame/layer count, palette size |
| `list_layers` | layer names bottom→top, visibility, group flag |
| `export` | save to another path/format (`out`), optional integer `scale` |
| `resize` | resize to `width`×`height`, save to `out` (or overwrite) |
| `export_spritesheet` | pack frames into a sheet `.png` + `.json` metadata |
| `get_palette` | palette as `#RRGGBBAA` strings |
| `apply_script` | *(gated)* run Lua with the sprite open as `spr`; set `result`; saves unless `no_save = true` |

Every op **opens and closes** the file per call — an operation never sees
another operation's state.

## Notes

- Aseprite 1.3+ Lua API (`app.open`, `Sprite:saveCopyAs`, `Sprite:resize`,
  `app.command.ExportSpriteSheet`).
- CI does not exercise this (no Aseprite on the runners, and it's a paid
  app). Verify locally with a real `.aseprite` file.
- "Any pixel art editor" from the original ask is narrowed to Aseprite —
  the others (Pixelorama, GraphicsGale, Pyxel Edit, LibreSprite) have no
  comparable scripting/CLI surface.
