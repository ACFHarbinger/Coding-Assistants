# Backend (Rust/Tauri) Roadmap

> Tracks planned work for `src-tauri/`. Sourced from moon research plus owner
> Q&A 2026-08-10. See [ADR 0002](../../adr/0002-polyglot-module-layout.md) and
> [ADR 0003](../../adr/0003-daemon-extraction-spike.md).
> **Product priority:** Cross-agent memory/coordination lives in
> [`hub.md`](hub.md) and outranks the daemon track.

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending · 💤 Someday/Maybe

## Current State

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| R1 | Scaffold `Cargo.toml`, `src/`, backend commands (`agents.rs`, `llm_client.rs`, `file_tools.rs`, `tcp_server.rs`) | S | ✅ Done |

---

## Track: Reliability Backlog (near-term)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RD8 | **Per-task AppState:** replace single global `Mutex<Option<…>>` for agents/cancel/input so concurrent tasks cannot clobber each other | M | 📋 Pending |
| RD9 | **Per-task / per-workspace MCP config path** (stop racing on fixed `~/.coding-assistants/mcp.json`) | S | 📋 Pending |
| RD4 | Cancellation-safety audit for `tokio::select!` | M | 📋 Pending |
| RD10 | Typed error system — replace `Result<T, String>` with structured error enum across commands | M | 📋 Pending |
| RD11 | Backend test suite (`cargo test`) for orchestration markers, TCP protocol, file tools path rules | M | 📋 Pending |
| RD12 | Configuration persistence — save/load agent configs; workspace profiles | M | 📋 Pending |

---

## Track: Provider / Tool Integration

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RP10 | **Direct HTTP API providers** — wire existing unused deps (`async-openai`, `reqwest`, `dotenv`, …) for OpenAI/Anthropic/Google/xAI-style APIs alongside CLI providers (owner: wire, do not drop) | L | 📋 Pending |
| RP11 | Provider trait abstracting CLI vs HTTP backends | M | 📋 Pending |
| RP12 | LM Studio full support (OpenAI-compatible endpoint; remove stub error) | S | 📋 Pending |
| RP13 | Model parameter tuning (temperature, top-p, max tokens) per role/agent | S | 📋 Pending |
| RP14 | Provider health checks before task start | S | 📋 Pending |
| RP15 | Cost estimation telemetry (token usage display; soft warnings by default) | M | 📋 Pending |
| RD5 | PTY integration for real-time ANSI/OSC tool output | M | 📋 Pending |
| RD6 | Headless SDK stream-json parsing into typed events | M | 📋 Pending |
| RT1 | Tool/command execution via OS APIs (not display-only); permission **user setting** (always / destructive-only / never) | L | 📋 Pending |
| RT2 | Workspace sandbox strictness as user setting; default **relaxed** for now | M | 📋 Pending |

---

## Track: Core Orchestration Daemon (Tokio)

Target: eventually extract headless daemon. **ADR 0003:** do **not** split
crates until after RD7 and a clearer multi-client need. Actor framework later.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RD1 | Spike: daemon vs in-process | M | ✅ Done |
| RD7 | Internal `tokio::sync::broadcast::<AgentEvent>` bus; remove `AppHandle` from agents/llm_client | S | 📋 Pending · **next spine step** |
| RD3 | Blocking I/O via `spawn_blocking` / `tokio::fs` | S | ✅ Done |
| RD2 | Actor framework (`kameo`/`ractor`) | L | 💤 Someday/Maybe · later interest |
| RD20 | Optional headless daemon binary + UDS API (no GraphQL required) | L | 📋 Pending · after RD7 + hub MVP |
| RD21 | Parallel agent execution (independent roles/adapters) | L | 📋 Pending · after async hub mailbox |
| RD22 | Configurable strategies: sequential, parallel, conditional workflows | L | 📋 Pending |
| RD23 | Agent templates for common tasks | M | 📋 Pending |

---

## Track: API Layer

GraphQL is **maybe later** (owner). Prefer simpler local protocols first.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RA0 | Documented local multi-client protocol options: UDS + JSON/JSON-RPC (preferred near-term) | M | 📋 Pending |
| RA1 | `async-graphql` + `axum` | L | 💤 Someday/Maybe · maybe later |
| RA2 | GraphQL subscriptions over WebSockets | M | 💤 Someday/Maybe |
| RA3 | Bridge agent bus → subscription resolvers | M | 💤 Someday/Maybe |
| RA4 | Keep Tauri IPC fallback for local file/desktop-hot paths | S | 📋 Pending |

---

## Track: Multi-Agent Protocols (MCP + A2A)

Lean external MCP first; promote hot tools into core later. A2A strategic only.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RM1 | MCP client (stdio + remote transports) | L | 📋 Pending · after hub MVP if not blocking |
| RM2 | MCP host — expose CA tools/resources | L | 📋 Pending · later |
| RM3 | Code-execution-first tool design for large payloads | M | 📋 Pending |
| RM4 | A2A Agent Cards / horizontal delegation | XL | 💤 Someday/Maybe |
| RM5 | Collaboration topologies on MCP/A2A | XL | 💤 Someday/Maybe · prefer hub.md RH30 first |

---

## Track: Resource Management (Rate Limiting + Budgets)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RB1 | Per-provider token-bucket (`governor`) | S | ✅ Done |
| RB2 | Lock-free hot-path limiter if needed | M | 💤 Someday/Maybe |
| RB3 | Per-session spend cap with **telemetry + soft warning default**; optional hard kill setting | M | 📋 Pending |
| RB3b | On cap: **pause**, write persistent markdown summary (objective, done, missing), delegate to user/agent, **shutdown until human wake** (owner) | M | 📋 Pending · pairs with hub RH23 |
| RB4 | Affine-typed compile-time budget ownership | M | 💤 Someday/Maybe · after runtime budgets |

---

## Track: Persistent Memory (single-agent framing — superseded)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RP1–RP4 | Two-tier briefing/deep store/decay/dedup as originally written | — | 📋 **Superseded by [`hub.md`](hub.md)** multi-agent hybrid memory |

---

## Track: Security & Human-in-the-Loop

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RS1 | Approval gate before destructive tools; surface exact args (mode configurable per task) | M | 📋 Pending |
| RS6 | **TCP remote auth** (token) + document LAN trust; TLS later | M | 📋 Pending |
| RS7 | Path canonicalization in `FileTools`; gate or remove unconstrained `read_file_absolute` | S | 📋 Pending |
| RS8 | Production CSP (replace null) | S | 📋 Pending |
| RS2 | No unvalidated token passthrough to downstream APIs | M | 📋 Pending |
| RS3 | MCP confused-deputy mitigations | M | 📋 Pending · with RM* |
| RS4 | Tool-poisoning sanitization | M | 📋 Pending |
| RS5 | No raw shell in tool layer — OS APIs only | S | ✅ Done |
