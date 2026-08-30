# Coding-Assistants Unity bridge

**Tier 3 — high risk.** No Python in Unity, so the plugin half is a C#
editor script. Target: **Unity 2021.3+**.

```
agent ──stdio MCP──► coding-assistants-mcp-unity ──TCP line-JSON──► CodingAssistantsBridge.cs ──► UnityEditor.*
```

`[InitializeOnLoad]` starts a localhost line-JSON TCP server (port
**9769**); `EditorApplication.update` pumps queued jobs on the editor main
thread. JSON is a compact embedded MiniJSON — no Newtonsoft dependency.

## Install

**As a local package (recommended):**
Window → Package Manager → **+** → *Add package from disk…* → pick
`plugins/unity/package.json`.

**Or drop-in:** copy `plugins/unity/Editor/` into your project's
`Assets/CodingAssistantsBridge/Editor/`.

The Console shows `Unity bridge listening on 127.0.0.1:9769` once the
editor finishes compiling. The bridge stops and restarts around domain
reloads (script recompiles, entering play mode).

## Register the MCP server

`coding-assistants-mcp-unity [--port N] [--allow-menu-exec]`

Claude `.mcp.json`:

```json
{ "mcpServers": { "coding-assistants-mcp-unity": {
    "command": "/path/to/coding-assistants-mcp-unity",
    "args": ["--port", "9769"]
} } }
```

`--allow-menu-exec` adds `execute_menu_item` (runs any editor menu command
by path). **Off by default** — menu commands can be destructive.

## Tools

| Tool | Effect |
|---|---|
| `get_editor_summary` | Unity version, active scene, root count, play-mode state |
| `list_gameobjects` | scene hierarchy `[{ name, path, active, components }]` |
| `create_gameobject` | empty GO, or a `primitive` (Cube/Sphere/…); optional `parent`, `position` |
| `delete_gameobject` | destroy by path |
| `set_transform` | local position / rotation (euler) / scale by path |
| `add_component` | add a component type by name (`Rigidbody`, `Light`, …) |
| `list_assets` | `AssetDatabase.FindAssets` under a folder, optional `t:` filter |
| `save_scene` | save the active scene |
| `execute_menu_item` | *(gated)* run an editor menu command by path |

## Caveats

- No CI: Unity is not installable on the runners. The C# is written for
  2021.3+ but **not compiler-verified** — check the Console on first load.
- `GameObject.Find` resolves paths from active roots; inactive objects
  won't be found. Use `list_gameobjects` to get exact paths.
- Domain reload tears the socket down briefly; a call made mid-reload gets
  a connection error — retry.
