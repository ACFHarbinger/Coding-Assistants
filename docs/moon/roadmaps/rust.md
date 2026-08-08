# Backend (Rust/Tauri) Roadmap

> Tracks planned work for `src-tauri/`. Sourced from
> [`docs/moon/research/Multi-Agent AI App Architecture.md`](../research/Multi-Agent%20AI%20App%20Architecture.md)
> and [`docs/moon/reports/AI Coding Tools Feature Report.md`](../reports/AI%20Coding%20Tools%20Feature%20Report.md).
> See [ADR 0002](../../adr/0002-polyglot-module-layout.md) for the current
> (pre-daemon) layout decision — the items below supersede parts of it and
> should get their own ADR once the daemon rearchitecture begins.

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending

## Current State

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| R1 | Scaffold `Cargo.toml`, `src/`, backend commands (`agents.rs`, `llm_client.rs`, `file_tools.rs`, `tcp_server.rs`) | S | ✅ Done |

## Track: Core Orchestration Daemon (Tokio + Actor Model)

Target: extract the current in-process Tauri-command backend into a headless
Core Orchestration Daemon built on `tokio`, so the same daemon can eventually
serve both the Tauri GUI and a future TUI over one API instead of being
wired directly into `invoke()` handlers.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RD1 | Spike: evaluate splitting `src-tauri/` into a `tokio`-based daemon crate + a thin Tauri IPC shim that talks to it locally | M | 📋 Pending |
| RD2 | Adopt an actor framework (`kameo` or `ractor`) so each LLM provider, tool-execution context, and local binary runs as an isolated actor communicating by message passing | L | 📋 Pending |
| RD3 | Route all blocking I/O (file access, crypto) through `tokio::task::spawn_blocking`; audit for accidental blocking calls on async worker threads | S | 📋 Pending |
| RD4 | Cancellation-safety audit for `tokio::select!` usage — ensure a dropped future (e.g. a timed-out provider call) never leaves agent state corrupted | M | 📋 Pending |
| RD5 | PTY integration via a `portable-pty`-equivalent crate so agent-invoked CLI tools (build scripts, test runners) emit real-time ANSI/OSC output instead of buffered plain text | M | 📋 Pending |
| RD6 | Headless SDK streaming: for providers with a structured streaming mode (e.g. `--output-format stream-json`), parse one JSON event per line into typed `serde` structs instead of scraping terminal output | M | 📋 Pending |

## Track: API Layer (GraphQL over WebSockets)

Target: replace/augment direct `invoke()` calls with a typed, subscribable
API so multiple clients (GUI, future TUI) can stay in sync off one source of
truth.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RA1 | Stand up `async-graphql` + `axum` as the daemon's API surface (mutations for command invocation, queries for history/metrics) | L | 📋 Pending |
| RA2 | GraphQL subscriptions over WebSockets (`tokio-tungstenite`) for live telemetry: token usage, tool-call events, agent status, per the `agentActivity(taskId)` pattern | M | 📋 Pending |
| RA3 | Internal `tokio::sync::broadcast` channel from agent/actor state changes to the GraphQL subscription resolvers | M | 📋 Pending |
| RA4 | Keep a documented IPC fallback: local file access continues to go through Tauri IPC directly rather than round-tripping the API layer | S | 📋 Pending |

## Track: Multi-Agent Protocols (MCP + A2A)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RM1 | MCP client support: connect to external MCP servers to consume Resources/Tools/Prompts (stdio and remote/Streamable-HTTP transports) | L | 📋 Pending |
| RM2 | MCP host support: expose this app's own Tools/Resources as an MCP server so other MCP-aware clients can drive it | L | 📋 Pending |
| RM3 | Code-execution-first tool design: for large intermediate payloads (logs, ASTs), expose a sandboxed script-execution tool instead of passing raw data into agent context, to avoid context-window exhaustion | M | 📋 Pending |
| RM4 | A2A protocol support: publish an Agent Card (`/.well-known/agent.json`) and support horizontal task delegation/discovery between locally-orchestrated agents | XL | 📋 Pending |
| RM5 | Collaboration topologies on top of MCP/A2A: handoffs (sequential delegation), chaining (pipelined output→input), and graph orchestration (parallel dispatch with shared state) | XL | 📋 Pending |

## Track: Resource Management (Rate Limiting + Budgets)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RB1 | Token-bucket rate limiting for outbound provider API calls (e.g. via the `governor` crate) to avoid provider-side throttling | S | 📋 Pending |
| RB2 | Lock-free, high-throughput rate limiting for internal hot paths (streaming token ingestion, IPC message rate) if `governor` proves to be a bottleneck under load | M | 📋 Pending |
| RB3 | Per-session spend cap: a `Budget` type that tracks cumulative cost against a user-set maximum and refuses further calls once exhausted | M | 📋 Pending |
| RB4 | Affine-typed budget delegation: make `Budget` non-`Clone`/non-`Copy` so a sub-budget can only be handed to one delegated agent at a time — a double-spend attempt becomes a compile-time "use of moved value" error, not a runtime bug | M | 📋 Pending |

## Track: Persistent Memory

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RP1 | Tier 1 declarative briefing: a compact (~150 line budget), auto-injected summary of high-confidence project facts at session start | M | 📋 Pending |
| RP2 | Tier 2 deep store: a local JSON store (e.g. `.memory/state.json`) of every captured decision/observation, queryable via `memory_search`/`memory_ask`-style tools mid-conversation | L | 📋 Pending |
| RP3 | Confidence-scored decay: temporal task-progress memories decay over ~7 days, broader context over ~30 days, architectural decisions persist indefinitely | M | 📋 Pending |
| RP4 | Deduplication pass during session compaction (e.g. Jaccard similarity) so repeated observations don't bloat the deep store | S | 📋 Pending |

## Track: Security & Human-in-the-Loop

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RS1 | Human-in-the-loop approval gate (`PreToolUse`-style hook) before any destructive command (`rm -rf`, force-push, etc.) executes — surface the exact command/args to the user for explicit approve/deny/modify | M | 📋 Pending |
| RS2 | Never pass client auth tokens through to downstream APIs unvalidated (token-passthrough mitigation) — scope and re-issue credentials at the daemon boundary instead | M | 📋 Pending |
| RS3 | MCP confused-deputy mitigation: user-bound permission scopes and explicit consent for any MCP proxy/tool call, no shared static service identity | M | 📋 Pending |
| RS4 | Tool-poisoning mitigation: sanitize/scan external tool descriptions and payloads (e.g. scraped web content, issue bodies) before they enter agent context | M | 📋 Pending |
| RS5 | Reconfirm no raw shell execution anywhere in the tool layer (already required by `AGENTS.md`'s Security Notes) as MCP/A2A tool surfaces expand — non-shell OS APIs only | S | 📋 Pending |
