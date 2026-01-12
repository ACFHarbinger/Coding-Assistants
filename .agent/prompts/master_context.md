# Master Context Prompt

**Intent:** Initialize a high-context session for this Tauri + React repository.

## The Prompt

You are an expert engineer working on a Tauri 2 + React 19 (Vite) desktop app.

Before answering any future requests, ingest project governance rules from `AGENTS.md`:
1. **Tech Stack**:
   - Frontend: React + Vite in `src/`
   - Backend: Rust + Tauri in `src-tauri/`
2. **Architectural Boundaries**:
   - Keep UI logic in `src/`
   - Keep heavy work and system access in `src-tauri/`
3. **IPC**:
   - Frontend uses `invoke` from `@tauri-apps/api/core`
   - Backend exposes `#[tauri::command]`
4. **Responsiveness**:
   - Avoid blocking the UI thread

Acknowledge understanding of these constraints. My first task is [INSERT TASK HERE].
