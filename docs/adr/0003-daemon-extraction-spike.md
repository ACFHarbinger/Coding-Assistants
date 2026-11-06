# 3. Core Orchestration Daemon extraction — spike findings (RD1)

Date: 2026-08-08

## Status

Accepted

## Context

[`docs/moon/roadmaps/platform.md`](../moon/roadmaps/platform.md) (RD1, sourced from
the target-architecture research) calls for evaluating whether to split
`src-tauri/` into a headless `tokio`-based Core Orchestration Daemon crate
plus a thin Tauri IPC shim, so the same daemon can eventually serve the GUI,
a future TUI (tracked in [`ui.md`](../moon/roadmaps/ui.md)), and the Android companion
app over one API instead of being wired directly into `invoke()` handlers.

This is a spike: the goal is a decision and a concrete next step, not the
full extraction itself.

### Current coupling, measured

`src-tauri/src/` is ~1300 lines across five modules. Grepping for
`AppHandle`/`tauri::` usage outside `lib.rs` (which is necessarily
Tauri-specific):

| Module | `AppHandle` references | Why |
| --- | --- | --- |
| `file_tools.rs` | 0 | Pure `tokio::fs` wrapper — already fully portable. |
| `llm_client.rs` | 2 | Takes `&AppHandle` solely to call `app.emit("agent-event", ...)` while streaming provider output. |
| `agents.rs` | 3 | Takes `&tauri::AppHandle` solely to call `app.emit("agent-event", ...)` at phase boundaries and for `[[ASK_USER]]`/`[[ASK_AGENT]]` events. |
| `tcp_server.rs` | 5 | Takes `AppHandle` to (a) `app.listen_any("agent-event", ...)` and re-broadcast to TCP clients, and (b) `app.emit(...)` to forward Android-originated requests back into the Tauri command layer. |

The pattern is already telling: **every one of these usages is either
"emit an event" or "listen for an event."** None of `agents.rs`,
`llm_client.rs`, or `tcp_server.rs` actually needs a Tauri *application* —
they need a place to publish `AgentEvent`s and, in `tcp_server.rs`'s case,
a place to subscribe to them. `TcpServer` is already, functionally, a second
"headless-ish" client of the agent system living outside the GUI — it just
currently reaches that data by piggybacking on Tauri's event bus rather than
a purpose-built one.

## Decision

**Don't extract a separate crate/binary yet. Decouple the event-emission
path from `tauri::AppHandle` first, inside `src-tauri/`, then extract once
the API layer's shape (RA1, GraphQL) is known.**

Rationale:

1. **The real coupling is one seam, not the whole crate.** All three
   Tauri-dependent modules touch `AppHandle` for exactly one reason: to
   publish/subscribe to `AgentEvent`. Replacing that with an
   in-process `tokio::sync::broadcast::<AgentEvent>` channel removes the
   `tauri` dependency from `agents.rs` and `llm_client.rs` entirely, and
   reduces `tcp_server.rs`'s usage to construction-time wiring.
2. **A physical crate split now would be guessing at a boundary we don't
   know yet.** RA1 (`async-graphql` + `axum` API layer) will define what a
   "daemon" actually needs to expose (mutations, queries, subscription
   topics). Extracting a crate before that exists risks drawing the
   module boundary in the wrong place and re-drawing it once GraphQL
   resolvers need access patterns the split didn't anticipate.
3. **Low risk, immediately useful.** The broadcast-channel refactor is a
   small, mechanical, independently testable change (~S/M effort) that
   both de-risks the eventual extraction *and* is a hard prerequisite for
   RA3 ("internal `tokio::sync::broadcast` channel from agent/actor state
   changes to the GraphQL subscription resolvers") — so it isn't
   throwaway spike work, it's the first real increment of RD2/RA3.
4. **`tcp_server.rs` is a working precedent, not a blocker.** It already
   proves a non-GUI client can consume the same event stream; the
   refactor just gives it (and the future GraphQL/TUI clients) a proper
   subscription point instead of re-listening to Tauri's own event bus.

## Consequences

- Follow-up roadmap item added: **RD7** — introduce an internal
  `tokio::sync::broadcast::<AgentEvent>` bus in a new `event_bus.rs`
  module; `agents.rs`/`llm_client.rs` publish to it instead of taking
  `&AppHandle`; the Tauri shim (`lib.rs`) and `tcp_server.rs` both become
  subscribers. This removes the `tauri` dependency from `agents.rs` and
  `llm_client.rs`.
- The actual crate/binary split (a `daemon/` crate with `src-tauri/` as a
  thin client) is deferred until RA1 lands, at which point it should be
  scoped as its own roadmap item informed by the GraphQL API's real
  shape rather than speculated now.
- No code changes ship as part of this spike beyond this record; RD7 is
  tracked separately in [`platform.md`](../moon/roadmaps/platform.md).
