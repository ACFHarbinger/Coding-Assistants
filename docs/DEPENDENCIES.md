# Dependencies

[![npm](https://img.shields.io/badge/npm-packages-CB3837?logo=npm&logoColor=white)](https://www.npmjs.com)
[![Cargo](https://img.shields.io/badge/Cargo-crates-DEA584?logo=rust&logoColor=black)](https://crates.io)

Complete inventory of all project dependencies, their versions, purposes, and license information.

---

## Frontend Dependencies (npm)

### Production

| Package                       | Version | Purpose                                           | License   |
| ----------------------------- | ------- | ------------------------------------------------- | --------- |
| `@tauri-apps/api`             | ^2      | Tauri IPC bridge (`invoke`, `listen`, `emit`)     | MIT/Apache |
| `@tauri-apps/plugin-dialog`   | ^2.6.0  | Native OS dialogs (file picker, message boxes)    | MIT/Apache |
| `@tauri-apps/plugin-opener`   | ^2      | Open URLs and files with system default handler   | MIT/Apache |
| `react`                       | ^19.1.0 | UI component library                              | MIT       |
| `react-dom`                   | ^19.1.0 | React DOM renderer                                | MIT       |

### Development

| Package                  | Version  | Purpose                               | License   |
| ------------------------ | -------- | ------------------------------------- | --------- |
| `@tauri-apps/cli`        | ^2       | Tauri CLI for dev/build commands       | MIT/Apache |
| `@types/react`           | ^19.1.8  | TypeScript type definitions for React  | MIT       |
| `@types/react-dom`       | ^19.1.6  | TypeScript type definitions for ReactDOM | MIT     |
| `@vitejs/plugin-react`   | ^4.6.0   | Vite plugin for React (JSX, HMR)      | MIT       |
| `typescript`             | ~5.8.3   | TypeScript compiler                    | Apache-2.0 |
| `vite`                   | ^7.0.4   | Frontend build tool and dev server     | MIT       |

---

## Backend Dependencies (Cargo)

### Production

| Crate                    | Version | Features     | Purpose                                    | License       |
| ------------------------ | ------- | ------------ | ------------------------------------------ | ------------- |
| `tauri`                  | 2       | `tray-icon`  | Desktop app framework with webview          | MIT/Apache    |
| `tauri-plugin-opener`    | 2       | --           | Open URLs and files from Rust               | MIT/Apache    |
| `tauri-plugin-dialog`    | 2.5.0   | --           | Native dialog boxes from Rust               | MIT/Apache    |
| `serde`                  | 1       | `derive`     | Serialization/deserialization framework      | MIT/Apache    |
| `serde_json`             | 1       | --           | JSON parsing and generation                  | MIT/Apache    |
| `tokio`                  | 1       | `full`       | Async runtime for all async operations       | MIT           |
| `reqwest`                | 0.12    | `json,stream`| HTTP client for API calls                    | MIT/Apache    |
| `async-openai`           | 0.26    | --           | OpenAI-compatible API client                 | MIT           |
| `dotenv`                 | 0.15    | --           | Load environment variables from `.env` files | MIT           |
| `walkdir`                | 2       | --           | Recursive directory traversal                | Unlicense/MIT |
| `toml_edit`              | 0.22    | --           | Comment-preserving `settings.toml` store     | MIT/Apache    |

### Build Dependencies

| Crate          | Version | Purpose                              | License    |
| -------------- | ------- | ------------------------------------ | ---------- |
| `tauri-build`  | 2       | Tauri build script for code generation | MIT/Apache |

---

## Android Dependencies (Gradle)

The Android companion app (`android/`) uses these key dependencies:

| Dependency                | Purpose                                |
| ------------------------- | -------------------------------------- |
| Jetpack Compose           | Declarative UI framework               |
| Material 3                | Material Design components             |
| Kotlin Coroutines         | Async programming                      |
| Kotlin Serialization      | JSON serialization                     |
| Compose Navigation        | Screen navigation                      |

---

## System Requirements

These external tools must be available on `$PATH` for full functionality:

| Tool       | Purpose                                    | Required |
| ---------- | ------------------------------------------ | -------- |
| `node`     | Frontend build toolchain                   | Yes      |
| `npm`      | Package manager                            | Yes      |
| `rustc`    | Rust compiler                              | Yes      |
| `cargo`    | Rust package manager and build tool        | Yes      |
| `ollama`   | Local LLM inference (Ollama provider)      | Optional |
| `opencode` | OpenCode CLI (OpenCode / DeepSeek)         | Optional |
| `vibe`     | Mistral Vibe CLI (Mistral provider)        | Optional |

---

## Dependency Update Policy

- **Tauri ecosystem** (`tauri`, `@tauri-apps/*`): Stay within major version 2. Update minor/patch versions regularly.
- **React**: Stay on version 19.x. Major upgrades require careful migration.
- **Tokio/Reqwest**: Keep current for security patches. These are foundational async crates.
- **serde/serde_json**: Stable APIs. Update freely.
- **async-openai**: Monitor for breaking changes as OpenAI APIs evolve.

### Checking for Updates

```bash
# Frontend
npm outdated

# Backend
cd src-tauri && cargo outdated
```

---

## License Compatibility

All dependencies use permissive licenses (MIT, Apache-2.0, or Unlicense) that are compatible with the project's AGPL-3.0 license. No copyleft dependency conflicts exist.
