# AGENTS.md

> Shared governance and workflow reference for AI coding assistants working in this repository.

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=black)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![License](https://img.shields.io/badge/License-AGPL--3.0-blue)](LICENSE)

---

## Tech Stack

| Layer        | Technology                         | Location       |
| ------------ | ---------------------------------- | -------------- |
| Frontend     | React 19 + TypeScript              | `src/`         |
| Backend      | Rust (edition 2021) + Tauri 2      | `src-tauri/`   |
| Build (FE)   | Vite 7                             | `vite.config.ts` |
| Build (BE)   | Cargo                              | `Cargo.toml`   |
| IPC          | `invoke` / `#[tauri::command]`     | Both           |
| Async        | Tokio                              | `src-tauri/`   |
| HTTP Client  | Reqwest 0.12                       | `src-tauri/`   |
| LLM SDK      | async-openai 0.26                  | `src-tauri/`   |
| Mobile       | Kotlin + Jetpack Compose           | `android/`     |

## Repository Layout

```
Coding-Assistants/
├── src/                    # React frontend
│   ├── App.tsx             # Root component & orchestration UI
│   ├── main.tsx            # Entry point
│   └── index.css           # Global styles (glass-morphism theme)
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Tauri app entry
│   │   ├── lib.rs          # Tauri commands & app state
│   │   ├── agents.rs       # Multi-agent orchestration engine
│   │   ├── llm_client.rs   # LLM provider integration
│   │   ├── tcp_server.rs   # TCP remote control server
│   │   └── file_tools.rs   # Workspace file utilities
│   ├── Cargo.toml          # Rust dependencies
│   ├── tauri.conf.json     # Tauri configuration
│   └── capabilities/       # Permission capabilities
├── android/                # Android remote control app
├── .agent/                 # Agent resources (prompts, rules, workflows)
├── env/                    # Environment variable templates
├── public/                 # Static assets
└── assets/                 # Media assets
```

## Architectural Boundaries

- **UI and presentation logic** live in `src/`. No system calls, no file I/O.
- **System access, file I/O, LLM orchestration, and networking** live in `src-tauri/`.
- **IPC payloads** must be JSON-serializable and aligned with Rust `serde` structs.
- **Agent resources** (prompts, rules, workflows) are stored in `.agent/` within the workspace directory.

## IPC Contract Rules

1. Every frontend `invoke("command_name", payload)` must map to a `#[tauri::command]` named `command_name`.
2. If a Rust command signature changes, update **all** corresponding `invoke` calls immediately.
3. Derive `Serialize` / `Deserialize` for every type crossing the IPC boundary.
4. Use Tauri events (`emit` / `listen`) for streaming data from backend to frontend.

### Registered Commands

| Command              | Purpose                                   |
| -------------------- | ----------------------------------------- |
| `run_agent_task`     | Execute multi-agent task sequence          |
| `submit_user_input`  | Send user response to waiting agent        |
| `cancel_task`        | Cancel running agent task                  |
| `get_agent_resources`| List prompts, rules, and workflows         |
| `get_resource_content`| Read a resource file from `.agent/`       |
| `read_file_absolute` | Read file at absolute path                 |
| `get_available_models`| Query available LLM models               |
| `start_tcp_server`   | Start remote control TCP server            |
| `stop_tcp_server`    | Stop remote control TCP server             |
| `get_server_ip`      | Get local network IP address               |

## Performance and Responsiveness

- Never block the UI thread. All heavy work happens in Rust async commands.
- LLM calls are async processes with line-by-line streaming via Tauri events.
- Use cancellation tokens (`AtomicBool`) to allow task interruption.
- TCP server uses `tokio::sync::broadcast` for efficient event fan-out.

## Error Handling

- Prefer `Result<T, String>` or a typed error enum for Tauri commands.
- Surface clear, user-friendly error messages to the frontend.
- Log errors to stderr in debug builds; do not expose internal stack traces to users.

## Development Commands

| Command                | Description                      |
| ---------------------- | -------------------------------- |
| `npm run dev`          | Frontend dev server (port 1420)  |
| `npm run tauri dev`    | Full app dev mode (FE + BE)      |
| `npm run build`        | Frontend production build        |
| `npm run tauri build`  | Bundle desktop application       |

## Testing

- No test harness is configured by default.
- If tests are requested:
  - **Frontend**: Vitest + React Testing Library.
  - **Backend**: `cargo test` in `src-tauri/`.
- See [TESTING.md](TESTING.md) for detailed testing strategy.

## Security Notes

- Do **not** invoke shells; pass explicit args to `std::process::Command`.
- Validate file paths and user input before use.
- Resource file reads are restricted to paths starting with `.agent`.
- See [SECURITY.md](SECURITY.md) for the full security policy.

## Code Style

- **Rust**: Follow standard `rustfmt` conventions. Use `clippy` for linting.
- **TypeScript**: Strict mode enabled. No unused locals or parameters.
- **CSS**: Use CSS custom properties for theming. Follow the existing glass-morphism design system.

## Agent Communication Protocol

Agents communicate via special markers embedded in LLM responses:

| Marker                      | Purpose                                      |
| --------------------------- | -------------------------------------------- |
| `[[ASK_USER]]`              | Pause execution and request user input        |
| `[[ASK_AGENT:RoleName]]`   | Request input from another agent (requires auth) |

These markers are parsed by `AgentSystem::interactive_completion` in `agents.rs`.

## Contribution Guidelines

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contribution workflow.
