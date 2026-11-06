# CLIP STUDIO PAINT — profile bridge

This is the Coding Assistants stdio MCP for the unofficial
[CLIP STUDIO PAINT 5.1.2 production server](https://github.com/A2pha/clip-studio-paint-5.1.2-production-mcp).
CELSYS does not publish a document or layer API for this profile. The
bridge therefore does not invent `get_layers` or a headless render, and it
does not inject pointer, keyboard, or window input.

## What the bridge does

`coding-assistants-mcp clipstudio` exposes the 33 catalog names.

- `csp_health` and `csp_capabilities` report the pinned profile
  (Windows, CLIP STUDIO PAINT 5.1.2.0, `zh-CN`, DPI 120) and mark the
  session as not attested.
- `csp_preflight` issues a 45-second single-use token bound to one atomic
  input tool and its exact parameters. A mismatch consumes the token.
- The ten atomic input tools, plus launch, focus, capture, and the other
  session tools, then fail closed. No `CLIPStudioPaint.exe` is started.
- File tools honor three absolute roots from `CSP_MCP_CONFIG` or `--config`:

```json
{
  "input": "/abs/input",
  "working": "/abs/working",
  "exports": "/abs/exports",
  "commands": ["document.save_as_dialog"],
  "auto_actions": []
}
```

`input` is read-only. `working` and `exports` accept write and overwrite
intents. Parent segments, wildcards, UNC paths, symlinks, and file hard
links are rejected. `csp_open_document` only validates an input-root file.

Enable the bridge from Settings → Creative Tools. The config key is
`coding-assistants-mcp-clipstudio`.
