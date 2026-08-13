# Platform, Providers, Tools, and Security Roadmap

This capability roadmap contains the later daemon/API work and the near-term
reliability work needed by the hub.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| P1 | Internal event bus decoupled from `tauri::AppHandle` | Backend events can be consumed by multiple in-process subscribers | 📋 Pending |
| P2 | Per-task state and cancellation; task-scoped MCP configuration | Concurrent tasks cannot clobber input, cancellation, or MCP configuration | 📋 Pending |
| P3 | Provider adapters for Claude, Codex, Gemini, Grok, OpenCode, Ollama, and llama.cpp | Start/message/cancel/status/usage capabilities are typed; local models work offline | 🚧 **Partial** · Process discovery and quota adapters exist. C12 needs typed **start** (wake may spawn a new instance that joins the session), **message inject**, and **message capture** for Grok, Chat/Codex, Claude, and Gemini. Codex inject via `ca inbox watch` is the only working path. |
| P8 | Attach to an existing model process/service | A role can use an already-running OpenAI-compatible endpoint without spawning or terminating its process | 🚧 **Partial** · Endpoint mode and local Grok/Claude/Codex/Gemini process discovery are available; health checks, streaming, auth, and provider-specific adapters remain |
| P4 | Direct HTTP providers using the existing unused dependencies where useful | Provider health, structured errors, streaming, and usage accounting are tested | 📋 Pending |
| P5 | OS-level tool execution with configurable approval and relaxed-default sandbox | Every execution is audited and policy-controlled | 📋 Pending |
| P6 | LAN TCP authentication and later TLS | LAN remains available, but unauthorized clients are rejected | 📋 Pending |
| P7 | Local daemon and Unix-domain-socket API | Extract only after event/memory boundaries and a second client require it | 📋 Pending · later |
| P8 | GraphQL/WebSockets and actor framework evaluation | Adopt only if measured query/concurrency needs justify them | 💤 Maybe later |
| P9 | MCP external-server integration and promotion of frequently used tools | External MCP remains the lean default; promoted tools have security tests | 📋 Pending |
| P10 | Runtime budget pause/summary/shutdown behavior | Budget policy is tested end-to-end; affine typing remains postponed | 📋 Pending |
| P11 | A2A implementation support | Tracked primarily in `communication.md` as the next major milestone | 📋 Pending |

License work: implement the owner-approved dual AGPL-3.0 + Commercial scheme
with a dedicated legal/documentation review before release.
