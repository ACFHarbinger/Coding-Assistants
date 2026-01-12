# Refactoring Safety Prompt

**Intent:** Safely refactor core logic without breaking IPC contracts.

## The Prompt

I need to modify the following component: `[INSERT COMPONENT/FILE]`.

**Current Goal:** [Brief description].

**Constraints:**
1. **Safety**: Avoid behavior changes; document any necessary changes clearly.
2. **Compatibility**: If a `#[tauri::command]` signature changes, update the matching `invoke` call.
3. **Scope**: Keep refactors small and localized.
4. **Tests**: List any relevant `cargo test` or frontend checks to run.

Provide the modified code snippet and a verification plan.
