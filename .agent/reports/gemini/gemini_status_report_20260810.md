# Gemini (Antigravity) Status Report & Architecture Review

**Date:** 2026-08-10
**Agent:** Gemini (Antigravity)
**Context:** Brainstorming session and shared-report Q&A completion.

---

## 1. Executive Summary

Following a thorough review of the `Coding-Assistants` repository, current documentation, and the owner's answers during our extensive Q&A session, it is clear that the product's identity has crystallized. The tool is pivoting from a simple sequential LLM pipeline into a **local-first collaboration hub** designed for solo power developers. 

The primary goal is now to establish robust **Cross-Agent Shared Memory & Coordination**, prioritizing a hybrid SQLite/Markdown memory tier and reliable 2D Directed Acyclic Graph (DAG) observability. Complex infrastructural leaps like a GraphQL API, fully extracted headless daemons, 3D visualizations, and TUI clients are explicitly demoted to secondary or "someday" statuses.

## 2. Pros and Cons of Current Implementation

### Pros
* **Rust & Tauri Core:** The combination of Rust for system-level operations and Tauri for lightweight IPC bridging is excellent. It avoids Electron's bloat, ensuring maximum system resources are reserved for running local LLMs (like Ollama).
* **Event-Driven Streaming:** Utilizing real-time Tauri IPC events for streaming LLM responses provides the necessary responsive UI feeling required for a complex hub.
* **Declarative File Context:** Relying on the `.agent/` directory to store rules, prompts, and reports maps perfectly to how LLMs work best (syncing context with Git version control).
* **Human-in-the-Loop Controls:** The `[[ASK_USER]]` and `[[ASK_AGENT:X]]` tokens are a very pragmatic and parseable way to orchestrate basic control flow in the current system.

### Cons
* **Monolithic Execution Logic:** Backend agent logic is tightly bound to Tauri `invoke()` handlers, severely limiting headless operations and parallel processing.
* **CLI Wrapper Fragility:** Utilizing `std::process::Command` to wrap CLI calls (like `ollama run`) is highly brittle. Scraping `stdout` prevents robust telemetry, token tracking, and structured data handling (JSON schema enforcement).
* **State Race Conditions:** The current `AppState` utilizing a single `Mutex` is vulnerable. As noted by Claude, concurrent `run_agent_task` calls will clobber cancellation and input channels, creating race conditions.
* **Frontend Overload:** The `App.tsx` file is doing too much. Orchestrating a 2D DAG, managing chat UI, and handling complex IPC state inside a single monolithic component is unsustainable.

## 3. What to Change & What to Keep

### What to Change
* **Implement an Event Bus / Async Mailbox (RD7):** Shift from sequential blocking execution to an asynchronous event bus to handle multiple agent tasks simultaneously. 
  * *Reason:* To support the newly established priority of "Cross-Agent Shared Memory & Coordination," agents need to run, pause, and delegate without locking the main thread.
* **Shift to Direct HTTP/SDKs (Replace CLI Wrappers):** Implement standard HTTP clients (e.g., via `reqwest`) or native SDKs to interact with OpenCode, Ollama, and external providers.
  * *Reason:* This provides strict JSON parsing and unlocks proper tool-calling schemas (MCP) rather than relying on regex parsing.
* **Durable Memory Integration:** Integrate SQLite for deep long-term storage and use Git-tracked Markdown for high-priority insights.
  * *Reason:* Per the owner's Q&A, a multi-tiered hybrid memory system is the top priority for V1.
* **Refactor Frontend State:** Break down `App.tsx` into modular components managing specific domains (Memory View, DAG View, Chat View).
  * *Reason:* Will prevent the React frontend from collapsing under its own weight when 2D graph observability is implemented.

### What to Keep
* **Rust Backend / Tokio Runtime:**
  * *Reason:* Memory safety and fearless concurrency are required when implementing the event bus and handling real-time streams safely.
* **React/TypeScript Frontend:**
  * *Reason:* The JavaScript ecosystem is significantly better equipped to handle dynamic node-based DAG wiring and complex dashboards than any native UI framework.
* **Local-First Posture:**
  * *Reason:* Enforces maximum security and privacy, allowing aggressive and safe file-system sandboxing via OS APIs.
* **Declarative Wiring (Node-Editor Style):**
  * *Reason:* The owner prefers explicit, declarative agent wiring over probabilistic A2A negotiation for V1, granting absolute control.

## 4. Implementation Avenues for Target Architecture

Given the owner's preference for **Local Collaboration Hub + Declarative Wiring + Hybrid Memory**, here are the implementation avenues:

### Avenue A: The Pragmatic Iteration (Recommended for V1)
* **Backend:** Remain inside Tauri. Implement a lightweight asynchronous event bus using standard `tokio::sync::broadcast` and `mpsc` channels. Introduce `rusqlite` for the local database tier.
* **Agent Harness:** Keep relying on OpenCode / existing CLIs for a short while longer, but wrap them in robust Rust traits that abstract away the brittle `stdout` parsing until native HTTP integration is built.
* **Frontend:** Implement `reactflow` for the 2D DAG declarative agent wiring. Break `App.tsx` into Zustand-managed stores to separate UI state from Tauri IPC state.
* **Why:** Achieves all immediate owner goals (Memory, Coordination, 2D observability) without wasting time on heavy architecture rewrites.

### Avenue B: The Early Actor Model (Future-Proofing)
* **Backend:** Implement an Actor Model (using `kameo` or `ractor` in Rust) immediately. Each agent, the user mailbox, and the memory DB are treated as isolated Actors communicating via strict messages.
* **Agent Harness:** Move strictly to direct API calls (Anthropic/OpenAI) or a structured standard like MCP immediately. 
* **Frontend:** Same as Avenue A, but the frontend subscribes to specific Actor states via WebSockets over Tauri IPC.
* **Why:** This creates an incredibly resilient, crash-proof architecture, but will significantly delay the V1 release due to the steep learning curve of Actor frameworks.

### Avenue C: The "Headless First" Rewrite
* **Backend:** Rip the logic entirely out of Tauri. Build a standalone Rust daemon exposing a local Unix domain socket or local HTTP API. 
* **Frontend:** Tauri becomes an incredibly thin wrapper that just hosts the React UI and connects to the local daemon's socket.
* **Why:** Allows the immediate development of headless/TUI/Android clients alongside the Desktop app. However, this contradicts the owner's instruction that TUI and headless features are secondary to establishing a working desktop hub.

## 5. Next Steps

1. Wait for the owner to finalize their admin report.
2. Formally lock the roadmap files (`docs/moon/ROADMAP.md` and `docs/moon/roadmaps/*.md`) based on the newly agreed-upon Product Contract in the shared report.
3. Begin engineering the Hybrid Memory Tier (SQLite + Markdown) as the first major V1 milestone.
