# Architecture

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=black)](https://www.rust-lang.org)

This document describes the system architecture, component responsibilities, data flow, and design decisions of the Coding Assistants application.

---

## High-Level Overview

Coding Assistants is a desktop application built on Tauri 2, combining a React 19 frontend with a Rust backend. The application orchestrates multiple LLM-powered agents that collaborate to solve software engineering tasks.

```
┌─────────────────────────────────────────────────────────────┐
│                     Desktop Application                      │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                   React 19 Frontend                     │  │
│  │  ┌────────────┐ ┌───────────┐ ┌─────────────────────┐  │  │
│  │  │Configuration│ │Task Exec  │ │   Event Viewer      │  │  │
│  │  │  Panel      │ │  Panel    │ │   & Output Display  │  │  │
│  │  └──────┬─────┘ └─────┬─────┘ └──────────┬──────────┘  │  │
│  │         │             │                   │             │  │
│  │         └─────────────┼───────────────────┘             │  │
│  │                       │                                 │  │
│  │              invoke() │ listen()                        │  │
│  └───────────────────────┼─────────────────────────────────┘  │
│                          │                                    │
│  ┌───────────────────────┼─────────────────────────────────┐  │
│  │                 Tauri IPC Bridge                         │  │
│  └───────────────────────┼─────────────────────────────────┘  │
│                          │                                    │
│  ┌───────────────────────┼─────────────────────────────────┐  │
│  │                  Rust Backend                            │  │
│  │                       │                                  │  │
│  │  ┌────────────────────┴───────────────────────────────┐  │  │
│  │  │                  AppState                           │  │  │
│  │  │  agents: Mutex<Option<AgentSystem>>                 │  │  │
│  │  │  cancellation_token: Mutex<Option<Arc<AtomicBool>>> │  │  │
│  │  │  user_input_tx: Mutex<Option<mpsc::Sender<String>>> │  │  │
│  │  │  tcp_server: Mutex<Option<TcpServer>>               │  │  │
│  │  └────────────────────┬───────────────────────────────┘  │  │
│  │                       │                                  │  │
│  │  ┌──────────┐ ┌───────┴──────┐ ┌────────────────────┐   │  │
│  │  │AgentSystem│ │  LLMClient   │ │   TCP Server       │   │  │
│  │  │(agents.rs)│ │(llm_client.rs│ │  (tcp_server.rs)   │   │  │
│  │  └─────┬────┘ └──────┬───────┘ └─────────┬──────────┘   │  │
│  │        │             │                    │              │  │
│  │  ┌─────┴────┐        │                    │              │  │
│  │  │FileTools │        │                    │              │  │
│  │  └──────────┘        │                    │              │  │
│  └──────────────────────┼────────────────────┼──────────────┘  │
│                         │                    │                 │
└─────────────────────────┼────────────────────┼─────────────────┘
                          │                    │
                          ▼                    ▼
                    ┌───────────┐       ┌──────────────┐
                    │ LLM APIs  │       │ Android App  │
                    │ (External)│       │ (TCP Client) │
                    └───────────┘       └──────────────┘
```

---

## Layer Breakdown

### 1. Frontend Layer (`src/`)

The frontend is a single-page React 19 application bundled with Vite 7.

#### Component Structure

```
App.tsx (root orchestrator)
├── Header
│   └── Title + connection status
├── Configuration Card
│   ├── ModelSelect (per role)
│   │   ├── Provider dropdown
│   │   ├── Model dropdown
│   │   └── Resource file pickers (prompt, rule, workflow)
│   ├── Add/Remove role buttons
│   ├── Workspace directory picker
│   └── MCP configuration editor
├── Execute Task Card
│   ├── Task description textarea
│   └── Launch / Cancel button
├── Remote Control Card
│   ├── Server status + IP display
│   ├── Start / Stop server button
│   └── Remote connection log
├── Agent Activity Card
│   └── Scrollable event log with colored badges
├── Final Output Card
│   └── Formatted result display
└── Modals
    ├── Resource Preview Modal
    ├── Authorization Request Modal
    └── User Input Modal
```

#### State Management

All state is managed via React `useState` hooks in `App.tsx`. There is no external state library. Key state includes:

- `roles: RoleConfig[]` -- Array of agent role configurations
- `events: AgentEvent[]` -- Stream of agent activity events
- `isRunning: boolean` -- Task execution status
- `finalOutput: string` -- Accumulated final result
- `showAuthModal / showInputModal` -- Modal visibility flags

#### IPC Pattern

The frontend communicates with the backend exclusively through:

- **`invoke()`** -- Request/response calls to Tauri commands
- **`listen()`** -- Event subscriptions for streaming data

```typescript
// Request/response
const result = await invoke("run_agent_task", { config, task });

// Event streaming
const unlisten = await listen("agent-event", (event) => {
  setEvents(prev => [...prev, event.payload]);
});
```

### 2. IPC Bridge (Tauri)

Tauri provides the IPC bridge between the webview (frontend) and the native Rust process (backend). All communication is JSON-serialized.

**Commands** (frontend -> backend):
- Synchronous request/response via `invoke`
- Each maps to a `#[tauri::command]` function

**Events** (backend -> frontend):
- Async push via `app_handle.emit("event-name", payload)`
- Frontend subscribes with `listen("event-name", callback)`

### 3. Backend Layer (`src-tauri/src/`)

The backend is a Rust application using Tauri 2 with Tokio for async operations.

#### Module Responsibilities

| Module          | File             | Responsibility                                    |
| --------------- | ---------------- | ------------------------------------------------- |
| Entry Point     | `main.rs`        | Windows subsystem config + calls `run()`          |
| App Setup       | `lib.rs`         | Tauri app builder, command registration, AppState |
| Agent System    | `agents.rs`      | Multi-role orchestration, prompt construction      |
| LLM Client      | `llm_client.rs`  | Process spawning for LLM providers, streaming      |
| TCP Server      | `tcp_server.rs`  | Remote control protocol over TCP/IP                |
| File Tools      | `file_tools.rs`  | Sandboxed workspace file read/write                |

#### AppState

Shared mutable state managed through `Mutex` wrappers:

```rust
struct AppState {
    agents: Mutex<Option<AgentSystem>>,
    cancellation_token: Mutex<Option<Arc<AtomicBool>>>,
    user_input_tx: Mutex<Option<mpsc::Sender<String>>>,
    tcp_server: Mutex<Option<TcpServer>>,
}
```

- `agents` -- The active agent system instance
- `cancellation_token` -- Shared atomic flag for task cancellation
- `user_input_tx` -- Channel sender for passing user input to waiting agents
- `tcp_server` -- Active TCP server instance for remote control

---

## Data Flow

### Task Execution Flow

```
1. User clicks "Launch"
   │
2. Frontend invokes run_agent_task(config, task)
   │
3. Backend creates AgentSystem with config
   │
4. AgentSystem.execute_phases() iterates through roles:
   │
   ├─── Role 1 (e.g., "Planner")
   │    ├── construct_prompt() - loads custom or default prompt
   │    ├── LLMClient.chat_completion() - spawns process
   │    │   └── Streams stdout line-by-line via emit("agent-event")
   │    ├── interactive_completion() - parses response for markers
   │    │   ├── [[ASK_USER]] → emit question, await user_input_rx
   │    │   └── [[ASK_AGENT:X]] → emit auth request, call role X
   │    └── Save output for next role's context
   │
   ├─── Role 2 (e.g., "Developer")
   │    └── (same flow, receives Role 1 output as context)
   │
   └─── Role N...
        └── Generate project memory file

5. Backend returns final accumulated result
   │
6. Frontend displays in Final Output card
```

### Remote Control Flow

```
1. User clicks "Start Server" on desktop
   │
2. Backend starts TCP listener on 0.0.0.0:5555
   │
3. Android app connects via TCP socket
   │
4. Android sends JSON request (newline-delimited):
   │  {"type": "StartTask", "config": {...}, "task": "..."}
   │
5. Backend emits "android-task-request" Tauri event
   │
6. Frontend picks up event, begins task execution
   │
7. Agent events are broadcast to TCP clients via broadcast channel
   │
8. Android displays real-time progress
```

### User Input Flow

```
1. Agent response contains [[ASK_USER]] marker
   │
2. Backend emits "agent-question" event with question text
   │
3. Frontend shows User Input Modal
   │
4. User types answer and submits
   │
5. Frontend invokes submit_user_input(input)
   │
6. Backend sends input through mpsc channel
   │
7. Agent receives input and continues generation
```

---

## Key Type Definitions

### Frontend (TypeScript)

```typescript
interface AgentEvent {
  source: string;           // Role name: "Planner", "Developer"
  event_type: string;       // "thought", "response", "question", etc.
  content: string;          // Event content
}

interface ModelConfig {
  provider: string;         // "openai", "anthropic", "ollama", etc.
  model: string;            // Specific model name
  prompt_file?: string;     // Custom prompt from .agent/prompts/
  rule_file?: string;       // Custom rules from .agent/rules/
  workflow_file?: string;   // Custom workflow from .agent/workflows/
}

interface RoleConfig {
  name: string;             // Display name for the role
  config: ModelConfig;      // LLM configuration for this role
}

interface AgentConfig {
  roles: RoleConfig[];      // Ordered list of roles to execute
  work_dir: string;         // Workspace directory path
  mcp_config: string;       // MCP server config (JSON string)
}
```

### Backend (Rust)

```rust
pub struct AgentEvent {
    pub source: String,
    pub event_type: String,
    pub content: String,
}

pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub prompt_file: Option<String>,
    pub rule_file: Option<String>,
    pub workflow_file: Option<String>,
}

pub struct AgentConfig {
    pub roles: Vec<RoleConfig>,
    pub work_dir: String,
    pub mcp_config: String,
}
```

---

## Design Decisions

### Why Tauri over Electron?

- **Smaller binary size** -- Tauri apps are typically 5-10 MB vs 100+ MB for Electron.
- **Lower memory footprint** -- Uses the OS webview instead of bundling Chromium.
- **Rust backend** -- Enables safe, performant system operations and async I/O.
- **Security** -- Tauri's capability system provides fine-grained permission control.

### Why External Processes for LLM Calls?

LLM providers are invoked via `std::process::Command` (spawning `ollama run`, `opencode run`, etc.) rather than direct HTTP API calls because:

- **CLI tools handle authentication** -- No need to manage API keys in the app for CLI-based providers.
- **Streaming is simple** -- Read stdout line-by-line for real-time output.
- **Provider agnostic** -- Adding a new CLI-based provider requires minimal code changes.

### Why Single-File Frontend?

`App.tsx` contains the entire UI in a single file. This is a pragmatic choice for a tool-oriented application where:

- The component tree is relatively flat.
- State is tightly coupled across sections.
- Extracting components would add indirection without reducing complexity.

### Why Mutex over RwLock?

`AppState` uses `Mutex` rather than `RwLock` because:

- Write operations are frequent (agent state changes on every event).
- The critical sections are short (acquire, update, release).
- Simplicity is preferred over marginal read concurrency gains.

---

## Security Architecture

```
┌─────────────────────────────────────┐
│           Tauri Capabilities         │
│  ┌───────────────────────────────┐  │
│  │  core:default                  │  │
│  │  opener:default                │  │
│  │  dialog:default                │  │
│  └───────────────────────────────┘  │
├─────────────────────────────────────┤
│         Application Layer            │
│  - File path validation              │
│  - .agent/ directory restriction     │
│  - No shell invocation               │
│  - Explicit process arguments        │
├─────────────────────────────────────┤
│         Network Layer                │
│  - TCP server on local network only  │
│  - JSON protocol validation          │
│  - No authentication (LAN trusted)   │
└─────────────────────────────────────┘
```

See [SECURITY.md](SECURITY.md) for the complete security policy.

---

## Future Architecture Considerations

- **Component extraction** -- As the UI grows, extract `ModelSelect`, `EventViewer`, and modal components into separate files.
- **Typed error enum** -- Replace `Result<T, String>` with a dedicated error enum for better error categorization.
- **Plugin system** -- Abstract LLM providers behind a trait for cleaner extensibility.
- **State management** -- Consider `useReducer` or a lightweight state library if state complexity increases.

See [ROADMAP.md](moon/ROADMAP.md) for planned features.
