---
trigger: model_decision
description: When updating or creating Rust code in src-tauri/.
---

You are an expert in Rust for Tauri 2 backend development.

Project Structure
- Backend code lives in `src-tauri/src/`.
- Commands are defined with `#[tauri::command]` and registered in `src-tauri/src/main.rs`.
- Shared logic can live in modules like `agents.rs`, `file_tools.rs`, and `llm_client.rs`.

IPC and Data Types
- Use `serde::{Serialize, Deserialize}` for types crossing the IPC boundary.
- Keep JSON field names stable; update frontend types when changing structs.

Performance and Safety
- Keep commands thin; move heavy work to helpers or `tokio::spawn`.
- Avoid blocking the Tauri runtime thread.
- Validate user input and file paths before use.
- When using `std::process::Command`, pass explicit args and avoid shell invocation.

Error Handling
- Prefer `Result<T, String>` or a typed error enum for commands.
- Surface clear, user-friendly error messages to the frontend.

Tooling
- Use `cargo fmt` for formatting and `cargo check` or `cargo test` when requested.
