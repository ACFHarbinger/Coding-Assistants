# Development Guide

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![Node.js](https://img.shields.io/badge/Node.js-LTS-5FA04E?logo=nodedotjs&logoColor=white)](https://nodejs.org)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=black)](https://www.rust-lang.org)

Everything you need to set up your development environment and start working on Coding Assistants.

---

## Prerequisites

### Required

| Tool    | Version         | Installation                                                |
| ------- | --------------- | ----------------------------------------------------------- |
| Node.js | LTS (20+)      | [nodejs.org](https://nodejs.org)                            |
| npm     | 10+             | Included with Node.js                                       |
| Rust    | stable (1.75+)  | [rustup.rs](https://rustup.rs)                              |
| Cargo   | (with Rust)     | Included with Rust                                          |

### Platform-Specific

Tauri 2 requires system dependencies that vary by platform. Follow the official guide:

- **Linux**: `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`
- **macOS**: Xcode Command Line Tools (`xcode-select --install`)
- **Windows**: Microsoft Visual Studio C++ Build Tools, WebView2

Full details: [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)

### Optional

| Tool       | Purpose                          | Installation                        |
| ---------- | -------------------------------- | ----------------------------------- |
| Ollama     | Run local LLM models            | [ollama.com](https://ollama.com)    |
| OpenCode   | OpenCode CLI provider            | Project-specific                    |
| Android Studio | Build the companion app      | [developer.android.com](https://developer.android.com/studio) |

---

## Initial Setup

```bash
# 1. Clone the repository
git clone https://github.com/ACFHarbinger/Coding-Assistants.git
cd Coding-Assistants

# 2. Install frontend dependencies
npm install

# 3. (Optional) Set up environment variables
cp env/vars.env env/.env
# Edit env/.env with your API keys

# 4. Verify Rust toolchain
rustc --version
cargo --version
```

---

## Development Workflow

### Running the App

```bash
# Full development mode (frontend + backend with hot reload)
npm run tauri dev
```

This command:
1. Starts the Vite dev server on port `1420`
2. Compiles the Rust backend
3. Launches the Tauri window pointing to the dev server
4. Watches for frontend changes (HMR) and backend changes (recompile)

### Frontend Only

```bash
# Run just the React frontend (useful for UI work)
npm run dev
```

The frontend dev server runs on port 1420 with strict port mode. Note that `invoke()` calls will fail without the Tauri backend.

### Building

```bash
# Frontend production build only
npm run build

# Full application bundle (platform-specific installer)
npm run tauri build
```

Build output locations:
- Frontend: `dist/`
- Application bundle: `src-tauri/target/release/bundle/`

---

## Project Structure Guide

### Where to Make Changes

| Change Type                    | Location                      |
| ------------------------------ | ----------------------------- |
| UI layout, styling             | `src/App.tsx`, `src/index.css` |
| New Tauri command              | `src-tauri/src/lib.rs`        |
| Agent orchestration logic      | `src-tauri/src/agent/orchestrator.rs` |
| LLM provider integration      | `src-tauri/src/client/llm.rs` |
| Remote control protocol       | `src-tauri/src/server/tcp_server.rs` |
| File system operations         | `src-tauri/src/core/file_tools.rs` |
| Tauri permissions/capabilities | `src-tauri/capabilities/`     |
| App metadata/config            | `src-tauri/tauri.conf.json`   |

### Adding a New Tauri Command

1. Define the command in `src-tauri/src/lib.rs`:

```rust
#[tauri::command]
async fn my_command(state: State<'_, AppState>, arg: String) -> Result<String, String> {
    Ok(format!("Hello, {}", arg))
}
```

2. Register it in the Tauri builder (also in `lib.rs`):

```rust
.invoke_handler(tauri::generate_handler![
    // ...existing commands
    my_command,
])
```

3. Call it from the frontend:

```typescript
import { invoke } from "@tauri-apps/api/core";

const result = await invoke<string>("my_command", { arg: "world" });
```

### Adding a New LLM Provider

1. Add the provider logic in `src-tauri/src/client/llm.rs`
2. Update `chat_completion()` to handle the new provider string
3. Update `list_models()` if the provider supports model discovery
4. Add the provider option to the frontend dropdown in `src/App.tsx`

---

## Environment Variables

| Variable              | Purpose                    | Required |
| --------------------- | -------------------------- | -------- |
| `OPENAI_API_KEY`      | OpenAI API access          | No       |
| `GOOGLE_GENAI_API_KEY`| Google Gemini API access   | No       |

Environment variables are loaded from `env/.env` via the `dotenv` crate at runtime.

---

## Configuration Files

| File                        | Purpose                                   |
| --------------------------- | ----------------------------------------- |
| `package.json`              | npm scripts and frontend dependencies     |
| `vite.config.ts`            | Vite build configuration (port 1420)      |
| `tsconfig.json`             | TypeScript compiler options (strict mode) |
| `tsconfig.node.json`        | TypeScript config for Vite config file    |
| `src-tauri/Cargo.toml`      | Rust dependencies and crate metadata      |
| `src-tauri/tauri.conf.json` | Tauri app config (window, bundle, CSP)    |
| `src-tauri/capabilities/default.json` | Tauri permission grants        |
| `Cargo.toml` (root)         | Cargo workspace definition                |

---

## Debugging

### Frontend

- Open browser DevTools in the Tauri window: Right-click -> Inspect (debug builds only)
- Vite HMR provides instant feedback on frontend changes
- Console logs from `console.log()` appear in DevTools

### Backend

- Rust `println!` and `eprintln!` output appears in the terminal running `npm run tauri dev`
- Use `RUST_LOG=debug` environment variable for verbose logging
- Attach a debugger via `rust-analyzer` in VS Code

### Common Debug Steps

1. **IPC not working**: Check that the command name in `invoke()` exactly matches the `#[tauri::command]` function name
2. **Serialization errors**: Ensure TypeScript types match Rust `serde` structs
3. **Permission denied**: Check `src-tauri/capabilities/default.json` for required permissions

---

## Code Style

### Rust

- Follow `rustfmt` defaults
- Run `cargo clippy` before committing
- Use `Result<T, String>` for command return types

### TypeScript

- Strict mode is enabled (`noUnusedLocals`, `noUnusedParameters`)
- Target ES2020 with React JSX transform
- No external linter is configured; rely on `tsc` for type checking

### CSS

- Use the existing CSS custom properties for colors
- Follow the glass-morphism design pattern (blur, transparency, gradients)
- Prefix utility classes appropriately

---

## Useful Commands Reference

```bash
# Development
npm run dev              # Frontend only
npm run tauri dev        # Full app (frontend + backend)

# Building
npm run build            # Frontend build
npm run tauri build      # Full app bundle

# Rust
cd src-tauri
cargo check              # Fast compile check
cargo clippy             # Linting
cargo test               # Run tests
cargo doc --open         # Generate and view docs

# Frontend
npx tsc --noEmit         # Type check without building
```
