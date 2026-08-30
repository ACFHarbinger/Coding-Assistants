# OpenToonz — viability spike (Tier 4)

**Finding: OpenToonz does not fit the "one MCP server per running app"
model.** There is no plugin to install here — hence no `plugins/opentoonz/`
code, only this note.

## Why

| Surface | Reality |
|---|---|
| Scripting API (Python/Lua/JS) | **None.** OpenToonz has no embedded interpreter and no script console. |
| Plugin IPC | **None usable.** The `toonz_plugin` C++ SDK targets raster *image effects* — a plugin gets tiles in and writes tiles out (`TNZ_PLUGIN_MAIN`). It cannot open a socket, enumerate the scene, or drive the app. |
| Headless / CLI render | **Not in mainline.** Old Toonz Harlequin had a `render` command; mainline OpenToonz and Tahoma2D expose only the GUI plus `tcleanup` (scan cleanup). Some distro builds add flags — none documented or portable. |
| Remote control | None. |

Every other bridge in this program works by a native plugin opening a
localhost socket. OpenToonz gives us nothing to attach that plugin to.

## What we ship instead — `crates/mcp-opentoonz`

A deliberately tiny MCP server that does **not** talk to a running
instance:

| Tool | How |
|---|---|
| `scene_info` | Parses the `.tnz` file directly (it's XML): frame count, camera resolution, referenced levels. Reliable, needs no OpenToonz install. |
| `render` | *(gated, `--allow-render`)* Runs `<opentoonz-bin> <argv>` and returns exit status + output. **Best-effort** — if your build has no headless render, a nonzero exit is the answer. |

```
coding-assistants-mcp-opentoonz [--opentoonz <path>] [--allow-render]
```

## Revisit if…

A future OpenToonz or Tahoma2D adds a scripting console or a remote-control
API. Then replace `crates/mcp-opentoonz` with a socket bridge shaped like
`crates/mcp-blender` and add a real `plugins/opentoonz/`.
