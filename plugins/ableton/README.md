# Coding-Assistants Ableton Live bridge

**Viability:** Ableton Live has **no Python console**. Automation is the
Live Object Model (LOM), reached through a **MIDI Remote Script** (Live's
embedded CPython) or Max for Live. This plugin is a Remote Script that
opens a localhost line-JSON TCP server — the same socket-bridge model as
Blender/Krita, not a file-parse surface.

**Not compiler-verified against Ableton.** Live is not installed on this
machine or CI. The script targets Live 11/12 (`_Framework.ControlSurface`,
Python 3). `python3 -m py_compile plugins/ableton/__init__.py` is the
offline check.

```
agent ──stdio MCP──► coding-assistants-mcp ableton ──TCP line-JSON──► Remote Script ──► Live LOM
```

The Remote Script starts a localhost line-JSON TCP server (port **9770**);
`update_display` (Live calls this ~10 Hz) pumps queued jobs on the Live
thread. LOM calls off that thread are unsafe.

## Install the Remote Script

Copy this folder so Ableton sees a control surface named
**CodingAssistants**:

```
~/Music/Ableton/User Library/Remote Scripts/CodingAssistants/__init__.py
```

Windows is typically
`Documents\Ableton\User Library\Remote Scripts\CodingAssistants\`.

Restart Live (or reload MIDI scripts). Preferences → Link/Tempo/MIDI →
**Control Surface**: pick an empty slot and choose **CodingAssistants**.
Input/Output can stay **None** — this script talks TCP, not a hardware
controller. The Log.txt / status bar shows
`Ableton bridge listening on 127.0.0.1:9770`.

The port is fixed at 9770 in this version; change `DEFAULT_PORT` in the
script and pass a matching `--port` if you need a different one.

## Register the MCP server

`coding-assistants-mcp ableton [--port N] [--allow-run-lom]`

`hub::mcp` renders this into each client's config. A Claude `.mcp.json`:

```json
{ "mcpServers": { "coding-assistants-mcp-ableton": {
    "command": "/path/to/coding-assistants-mcp",
    "args": ["ableton", "--port", "9770"]
} } }
```

`--allow-run-lom` adds a `run_lom` tool (arbitrary LOM Python).
**Off by default.**

## Tools

| Tool | Effect |
|---|---|
| `get_song_summary` | name, file path, tempo, time sig, playing, track/scene counts |
| `list_tracks` | session tracks: `{ index, name, kind, mute, solo, arm }` |
| `list_scenes` | `{ index, name, clip_count }` |
| `fire_clip` | launch clip slot `(track_index, scene_index)` |
| `fire_scene` | launch a scene by index |
| `set_tempo` | BPM 20–999 |
| `create_midi_track` | new MIDI track at the end; optional `name` |
| `set_track_name` | rename by `track_index` |
| `run_lom` | *(gated)* Python against `song` / `Live`; stdout + `repr(result)` |

## Smoke check

Ableton has no headless Remote Script runner, so there is no unattended
Live smoke. Offline: `python3 plugins/ableton/smoke.py` (compiles the
script and drives every op against a dummy LOM). With Live running and
the control surface selected, from another terminal:
`printf '{"op":"get_song_summary","args":{}}\n' | nc 127.0.0.1 9770`.
