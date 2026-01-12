# CLI Generation Prompt

**Intent:** Generate the correct local command to run or build this project.

## The Prompt

Based on `package.json` and `src-tauri/`, generate the exact CLI command to:
1. Run the frontend in development mode.
2. Run the full Tauri app in development mode.
3. Build the production bundle.

Output only the bash commands, one per line.
