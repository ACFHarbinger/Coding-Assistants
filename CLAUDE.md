# AGENTS.md

This repository is a Tauri 2 + React 19 (Vite) desktop app. Use this file as the shared governance and workflow reference.

## Tech Stack
- Frontend: React + TypeScript in `src/`
- Backend: Rust + Tauri in `src-tauri/`
- Build tooling: Vite (frontend) and Cargo (backend)
- IPC: `invoke` from `@tauri-apps/api/core` to `#[tauri::command]`

## Repository Layout
- `src/`: React components, UI logic, and styles
- `src/main.tsx`: frontend entry point
- `src/App.tsx`: root component
- `src/index.css`: global styles
- `src-tauri/src/`: Rust backend code
- `src-tauri/src/main.rs`: Tauri app entry
- `src-tauri/src/lib.rs`: shared modules and commands

## Architectural Boundaries
- UI and presentation logic live in `src/`.
- System access, file I/O, and heavy work live in `src-tauri/`.
- Keep IPC payloads JSON-serializable and aligned with Rust `serde` structs.

## IPC Contract Rules
- Every frontend `invoke("command_name", payload)` must map to a `#[tauri::command]` named `command_name`.
- If a Rust command signature changes, update all corresponding `invoke` calls.
- Derive `Serialize`/`Deserialize` for types crossing the boundary.

## Performance and Responsiveness
- Avoid blocking the UI thread.
- Move heavy CPU or file operations into Rust commands or async helpers.

## Error Handling
- Prefer `Result<T, String>` or a typed error enum for Tauri commands.
- Surface clear, user-friendly errors to the frontend.

## Development Commands
- Frontend dev server: `npm run dev`
- Full app dev mode: `npm run tauri dev`
- Frontend build: `npm run build`
- Tauri bundle: `npm run tauri build`

## Testing
- No test harness is configured by default.
- If tests are requested:
  - Frontend: prefer Vitest + React Testing Library.
  - Backend: use `cargo test` in `src-tauri/`.

## Security Notes
- Do not invoke shells; pass explicit args to `std::process::Command`.
- Validate file paths and user input before use.
