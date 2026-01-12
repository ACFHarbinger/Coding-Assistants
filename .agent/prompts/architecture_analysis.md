# Architectural Analysis Prompt

**Intent:** Analyze the frontend/backend boundary in this Tauri + React repository.

## The Prompt

I need to understand the interface between the React UI and the Rust backend.

Analyze the relationship between:
- `src/` (React + Vite components and `invoke` calls)
- `src-tauri/src/` (Tauri commands and Rust modules)
- The IPC data structures passed across the boundary

Explain potential bottlenecks (e.g., heavy work on the UI thread, large payloads, sync I/O) and suggest improvements while keeping `invoke` contracts stable.
