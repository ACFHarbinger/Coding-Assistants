# Testing Guide

[![Vitest](https://img.shields.io/badge/Vitest-Frontend-6E9F18?logo=vitest&logoColor=white)](https://vitest.dev)
[![Cargo Test](https://img.shields.io/badge/Cargo-Test-DEA584?logo=rust&logoColor=black)](https://doc.rust-lang.org/cargo/commands/cargo-test.html)

Testing strategy, setup instructions, and guidelines for Coding Assistants.

---

## Current State

No test harness is configured by default. This document outlines the recommended testing approach and provides setup instructions for when tests are needed.

---

## Testing Strategy

### Test Pyramid

```
         ┌─────────┐
         │  E2E    │   Tauri WebDriver (few, critical paths)
        ─┼─────────┼─
        │Integration│   Multi-module Rust tests, IPC round-trips
       ─┼───────────┼─
       │   Unit      │   Component tests (React), Function tests (Rust)
      ─┼─────────────┼─
```

| Layer       | Frontend Tool                  | Backend Tool   | Coverage Target |
| ----------- | ------------------------------ | -------------- | --------------- |
| Unit        | Vitest + React Testing Library | `cargo test`   | High            |
| Integration | Vitest (mocked IPC)            | `cargo test`   | Medium          |
| E2E         | Tauri WebDriver                | --             | Low (key flows) |

---

## Frontend Testing

### Setup (Vitest + React Testing Library)

```bash
# Install test dependencies
npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom
```

Add to `vite.config.ts`:

```typescript
export default defineConfig(async () => ({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
  },
  // ...existing config
}));
```

Create `src/test/setup.ts`:

```typescript
import '@testing-library/jest-dom';
```

Add test script to `package.json`:

```json
{
  "scripts": {
    "test": "vitest",
    "test:ui": "vitest --ui",
    "test:coverage": "vitest run --coverage"
  }
}
```

### What to Test (Frontend)

| Area                  | Test Type   | Example                                          |
| --------------------- | ----------- | ------------------------------------------------ |
| Role configuration    | Unit        | Adding/removing roles updates state correctly     |
| Provider dropdown     | Unit        | Available providers render, selection works       |
| Event rendering       | Unit        | AgentEvent objects render with correct badges     |
| Task launch           | Integration | Launch button invokes `run_agent_task`            |
| Modal dialogs         | Unit        | Auth/Input modals show/hide correctly             |
| Error display         | Unit        | Error states render user-friendly messages        |

### Mocking Tauri IPC

Mock `@tauri-apps/api/core` for tests that don't need the backend:

```typescript
// src/test/mocks/tauri.ts
import { vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
```

### Running Frontend Tests

```bash
# Run once
npx vitest run

# Watch mode
npx vitest

# With UI
npx vitest --ui

# Coverage report
npx vitest run --coverage
```

---

## Backend Testing

### Running Rust Tests

```bash
cd src-tauri
cargo test
```

### What to Test (Backend)

| Module          | Test Type   | Example                                              |
| --------------- | ----------- | ---------------------------------------------------- |
| `agents.rs`     | Unit        | Prompt construction with/without custom files         |
| `agents.rs`     | Unit        | `[[ASK_USER]]` marker parsing                        |
| `agents.rs`     | Unit        | `[[ASK_AGENT:X]]` marker parsing and routing         |
| `llm_client.rs` | Unit        | Model list parsing from CLI output                    |
| `llm_client.rs` | Integration | Process spawning with mock responses                  |
| `tcp_server.rs` | Unit        | JSON request/response serialization                   |
| `tcp_server.rs` | Integration | TCP client connection and protocol exchange            |
| `file_tools.rs` | Unit        | Read/write relative to workspace                      |
| `file_tools.rs` | Unit        | Path traversal prevention                             |
| `lib.rs`        | Unit        | Resource path validation                              |

### Example Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_event_serialization() {
        let event = AgentEvent {
            source: "Planner".to_string(),
            event_type: "thought".to_string(),
            content: "Analyzing the task...".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source, "Planner");
    }

    #[test]
    fn test_file_tools_read_within_workspace() {
        let tools = FileTools {
            work_dir: PathBuf::from("/tmp/test_workspace"),
        };
        // Create test file, verify read works
    }

    #[test]
    fn test_resource_path_validation() {
        // Verify paths starting with .agent are allowed
        // Verify paths outside .agent are rejected
    }
}
```

### Async Tests

For async functions (most Tauri commands use `async`), use `#[tokio::test]`:

```rust
#[tokio::test]
async fn test_get_available_models() {
    // Test model discovery
}
```

---

## End-to-End Testing

Tauri supports E2E testing via WebDriver. This requires additional setup.

### Setup

```bash
# Install WebDriver dependencies
cargo install tauri-driver
npm install -D @wdio/cli @wdio/local-runner @wdio/mocha-framework @wdio/spec-reporter
```

### Key E2E Scenarios

1. **Launch and configure** -- App starts, user can add roles, select providers
2. **Execute task** -- Full task execution with mocked LLM output
3. **Cancel task** -- Task cancellation stops agent execution
4. **Remote control** -- Start/stop TCP server, verify connectivity
5. **Resource browsing** -- Load and preview agent resources from `.agent/`

---

## Test Data

### Mock Agent Events

```json
[
  {"source": "Planner", "event_type": "thought", "content": "Analyzing requirements..."},
  {"source": "Planner", "event_type": "response", "content": "Plan: 1. Parse input 2. Generate output"},
  {"source": "Developer", "event_type": "thought", "content": "Implementing based on plan..."},
  {"source": "Developer", "event_type": "response", "content": "```rust\nfn main() {}\n```"}
]
```

### Mock TCP Protocol Messages

```json
{"type": "GetModels"}
{"type": "StartTask", "config": {"roles": [], "work_dir": "/tmp", "mcp_config": "{}"}, "task": "test"}
{"type": "CancelTask"}
{"type": "SubmitInput", "input": "yes"}
{"type": "GetStatus"}
```

---

## CI Integration

When CI/CD is set up, tests should run on every pull request:

```yaml
# .github/workflows/test.yml
name: Test
on: [pull_request]
jobs:
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm ci
      - run: npx vitest run

  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cd src-tauri && cargo test
```

---

## Coverage Goals

| Module            | Target Coverage |
| ----------------- | --------------- |
| `agents.rs`       | 70%+            |
| `llm_client.rs`   | 60%+            |
| `tcp_server.rs`   | 70%+            |
| `file_tools.rs`   | 90%+            |
| `lib.rs`          | 60%+            |
| Frontend (React)  | 60%+            |
