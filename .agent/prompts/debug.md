# Debugging Prompt

**Intent:** Resolve runtime issues in this Tauri + React app.

## The Prompt

I am encountering a specific error: `[INSERT ERROR HERE]`.

Context:
- **Layer**: [Frontend (React/Vite) / Backend (Tauri/Rust)]
- **Operation**: [e.g., invoke command, file dialog, LLM request]

Task:
Analyze relevant code snippets (or suggest which files to read). Identify potential causes such as:
1. **IPC Mismatch**: `invoke` name or payload shape doesn't match the Rust command.
2. **Blocking Work**: CPU or I/O work running on the UI thread.
3. **Command Errors**: `std::process::Command` failed or returned stderr.
4. **Frontend Errors**: React state/effects or Vite build issues.

Propose a fix that keeps the UI responsive and preserves IPC contracts.
