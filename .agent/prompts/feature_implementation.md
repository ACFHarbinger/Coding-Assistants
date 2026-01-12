# Feature Implementation Prompt

**Intent:** Implement a feature while preserving the Tauri + React architecture.

## The Prompt

I need to implement a new feature: `[INSERT FEATURE NAME]`.
**Goal:** [Brief description].
**Dependencies:** [Relevant React components, Rust modules, or commands].

**Constraints:**
1. **Separation**: UI in `src/`, backend logic in `src-tauri/`.
2. **IPC**: Use `invoke` + `#[tauri::command]` with JSON-serializable payloads.
3. **Performance**: No heavy work in React; use Rust async helpers if needed.

Provide a plan and any key code snippets to implement the feature.
