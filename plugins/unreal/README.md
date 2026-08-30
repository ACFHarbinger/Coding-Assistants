# Coding-Assistants Unreal Engine bridge

**Tier 3 — high risk.** Prerequisites you must satisfy yourself:

- **Unreal Engine 5.x** (5.1+ for the subsystem APIs used here).
- **Python Editor Script Plugin** enabled: Edit → Plugins → search
  "Python" → tick *Python Editor Script Plugin*, restart.
- The startup script installed in your project (below).

```
agent ──stdio MCP──► coding-assistants-mcp-unreal ──TCP line-JSON──► init_unreal.py ──► unreal.*
```

Rather than speak Unreal's UDP-multicast *remote execution* protocol, the
startup script opens a plain localhost line-JSON TCP server (port **9768**)
— the same shape as the Blender/Krita/Godot bridges — and marshals
`unreal.*` calls to the game thread with a slate post-tick callback.

## Install the startup script

Copy `init_unreal.py` to:

```
<YourProject>/Content/Python/init_unreal.py
```

Create `Content/Python/` if it doesn't exist. Unreal runs any
`init_unreal.py` on that path automatically at editor startup (with the
Python plugin enabled). The Output Log shows
`Unreal bridge listening on 127.0.0.1:9768`.

If you already have an `init_unreal.py`, append this file's `_start()`
call and ops to it instead of overwriting.

## Register the MCP server

`coding-assistants-mcp-unreal [--port N] [--allow-run-python]`

Claude `.mcp.json`:

```json
{ "mcpServers": { "coding-assistants-mcp-unreal": {
    "command": "/path/to/coding-assistants-mcp-unreal",
    "args": ["--port", "9768"]
} } }
```

`--allow-run-python` adds `run_python` (arbitrary `unreal` Python). **Off
by default.**

## Tools

| Tool | Effect |
|---|---|
| `get_editor_summary` | project dir, engine version, current level, actor count |
| `list_actors` | `[{ label, class, location }]` in the current level |
| `spawn_actor` | spawn from a built-in class name or `/Game/...` blueprint class path |
| `destroy_actor` | by label |
| `set_actor_transform` | location / rotation (pitch,yaw,roll) / scale by label |
| `list_assets` | asset paths under a content dir |
| `save_level` | save the current level |
| `run_python` | *(gated)* `unreal` Python snippet; stdout + `repr(result)` |

## Caveats

- API names shift between 5.x minor versions (`EditorActorSubsystem`,
  `LevelEditorSubsystem` are 5.0+; older 4.x used `EditorLevelLibrary`).
  This targets 5.1+. If an op fails with an `AttributeError`, your engine
  version differs — the `run_python` tool (gated) is the escape hatch.
- No CI coverage: Unreal is not installable on the runners. `py_compile`
  only. Verify against a real editor.
