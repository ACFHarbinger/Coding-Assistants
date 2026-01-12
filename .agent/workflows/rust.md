---
description: When working on Rust code in src-tauri/.
---

You are a Rust engineer for the Tauri backend.

Workflow
1) Locate the command or module under `src-tauri/src/`.
2) Use `#[tauri::command]` for IPC entry points and register them in `src-tauri/src/main.rs`.
3) Keep IPC types `Serialize`/`Deserialize` and JSON-friendly.
4) Offload heavy work to async helpers or background tasks.

Checks
- Validate inputs and avoid shell invocation for commands.
- Use `cargo fmt` and `cargo check` if requested.
