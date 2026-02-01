# Coding Assistants

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![Vite](https://img.shields.io/badge/Vite-7-646CFF?logo=vite&logoColor=white)](https://vite.dev)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=black)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Tokio](https://img.shields.io/badge/Tokio-async-463B3B?logo=rust&logoColor=white)](https://tokio.rs)
[![Kotlin](https://img.shields.io/badge/Kotlin-Android-7F52FF?logo=kotlin&logoColor=white)](https://kotlinlang.org)
[![Jetpack Compose](https://img.shields.io/badge/Jetpack_Compose-Material3-4285F4?logo=jetpackcompose&logoColor=white)](https://developer.android.com/compose)
[![License](https://img.shields.io/badge/License-AGPL--3.0-blue)](LICENSE)

A multi-agent orchestration desktop application that coordinates multiple LLM-powered agents to collaboratively solve software engineering tasks. Configure agent roles (Planner, Developer, Reviewer, etc.), assign different LLM providers to each, and watch them work together with inter-agent communication, user interaction, and real-time streaming output.

---

## Features

- **Multi-Agent Orchestration** -- Define multiple agent roles with independent configurations and watch them execute tasks sequentially, passing context between phases.
- **Multi-Provider LLM Support** -- Connect to OpenCode Zen, Google Gemini, Anthropic Claude, OpenAI, GitHub Copilot, Ollama, and LM Studio from a single interface.
- **Real-Time Streaming** -- Agent thoughts and responses stream to the UI line-by-line as they are generated.
- **Inter-Agent Communication** -- Agents can request input from other agents via `[[ASK_AGENT:RoleName]]` markers, with user authorization.
- **User-in-the-Loop** -- Agents can pause and ask the user questions via `[[ASK_USER]]` markers, displayed as modal dialogs.
- **Workspace-Aware** -- Select a project directory and agents operate within it, reading/writing files and loading custom prompts, rules, and workflows from `.agent/`.
- **Remote Control** -- Start a TCP server on port 5555 and control the desktop app from an Android companion app over your local network.
- **MCP Integration** -- Configure Model Context Protocol servers (sequential-thinking, filesystem, memory) for enhanced agent capabilities.
- **Glass-Morphism UI** -- Dark theme with frosted glass effects, gradient buttons, and colored agent badges.

## Screenshots

*Coming soon*

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org) (LTS recommended)
- [Rust](https://rustup.rs) (stable toolchain)
- [Tauri CLI prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform
- At least one LLM provider:
  - [Ollama](https://ollama.com) for local models
  - API keys for cloud providers (OpenAI, Anthropic, Google, etc.)

### Install & Run

```bash
# Clone the repository
git clone https://github.com/your-username/Coding-Assistants.git
cd Coding-Assistants

# Install frontend dependencies
npm install

# Run in development mode (launches both frontend and Tauri backend)
npm run tauri dev
```

### Configure API Keys

Copy the environment template and add your keys:

```bash
cp env/vars.env env/.env
# Edit env/.env with your API keys
```

### Build for Production

```bash
npm run tauri build
```

The bundled application will be output to `src-tauri/target/release/bundle/`.

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│                  React Frontend                  │
│  ┌───────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Role Mgmt │ │ Task Exec│ │ Event Viewer   │  │
│  └─────┬─────┘ └────┬─────┘ └───────┬────────┘  │
│        │            │               │            │
│        └────────────┼───────────────┘            │
│                     │ invoke / listen             │
├─────────────────────┼───────────────────────────-┤
│                     │ Tauri IPC                   │
├─────────────────────┼────────────────────────────┤
│                  Rust Backend                     │
│  ┌──────────┐ ┌─────┴─────┐ ┌──────────────┐    │
│  │ AgentSys │ │ LLMClient │ │  TCP Server   │    │
│  └──────────┘ └───────────┘ └──────────────-┘    │
│        │            │               │            │
│        ▼            ▼               ▼            │
│   File Tools    LLM APIs     Android App         │
└─────────────────────────────────────────────────-┘
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for a detailed breakdown.

## Supported LLM Providers

| Provider         | Type   | Models                        |
| ---------------- | ------ | ----------------------------- |
| OpenCode Zen     | Cloud  | Auto-detected via CLI         |
| Google Gemini    | Cloud  | Gemini Pro, Flash, etc.       |
| Anthropic        | Cloud  | Claude 3/3.5/4 family         |
| OpenAI           | Cloud  | GPT-4o, GPT-4, etc.          |
| GitHub Copilot   | Cloud  | Copilot models                |
| Ollama           | Local  | Any pulled model              |
| LM Studio        | Local  | Any loaded model              |

## Android Companion App

The `android/` directory contains a Kotlin/Jetpack Compose app for remote controlling the desktop application over TCP/IP. See the [Android README](android/README.md) for setup instructions.

## Documentation

| Document                                    | Description                              |
| ------------------------------------------- | ---------------------------------------- |
| [AGENTS.md](AGENTS.md)                      | Governance and workflow for AI assistants |
| [ARCHITECTURE.md](ARCHITECTURE.md)          | System design and data flow              |
| [DEPENDENCIES.md](DEPENDENCIES.md)          | Dependency inventory and rationale       |
| [DEVELOPMENT.md](DEVELOPMENT.md)            | Developer setup and workflow guide       |
| [TESTING.md](TESTING.md)                    | Testing strategy and instructions        |
| [SECURITY.md](SECURITY.md)                  | Security policy and guidelines           |
| [CONTRIBUTING.md](CONTRIBUTING.md)          | Contribution workflow                    |
| [TUTORIAL.md](TUTORIAL.md)                  | Step-by-step usage tutorial              |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md)    | Common issues and solutions              |
| [ROADMAP.md](ROADMAP.md)                    | Project roadmap and planned features     |

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri Extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE).
