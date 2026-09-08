# Platform, Providers, Tools, and Security Roadmap

This capability roadmap contains the later daemon/API work and the near-term
reliability work needed by the hub.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| P1 | Internal event bus decoupled from `tauri::AppHandle` | Backend events can be consumed by multiple in-process subscribers | 📋 Pending |
| P2 | Per-task state and cancellation; task-scoped MCP configuration | Concurrent tasks cannot clobber input, cancellation, or MCP configuration | 📋 Pending |
| P3 | Provider adapters for Claude, Codex, Gemini, Grok, OpenCode, Ollama, and llama.cpp | Start/message/cancel/status/usage capabilities are typed; local models work offline | 🚧 **Partial** · Process discovery and quota adapters exist. Orchestrate can select **DeepSeek via OpenCode** (`opencode models` / `opencode run -m deepseek/<model>`) and **Mistral via vibe** (explicit `-p/--workdir/--trust/--output`, unavailable when missing or unauthenticated). C12 supplies safe capture/refusal plus Grok and conditional Codex delivery. C14 now owns native managed-session lifecycle, Claude Channel integration, a serialized Codex writer broker, and app-owned Antigravity (`agy`) workers. **In flight (2026-09-08):** Meta **Muse Code** (`communication.md` C14.11, #273) and **Cursor `agent`** (C14.12, #275) join as managed harnesses; `HarnessId::{Muse,Cursor}` scaffold + roster identities is #271 (Claude). |
| P4a | Meta **Muse Spark 1.3** model provider (Meta Model API) | Direct HTTP inference against the Meta Model API (`dev.meta.ai`) is typed with health, structured errors, and usage accounting; presence-only auth check (no key read from disk); degrades to `unavailable` when unconfigured | 📋 **Not started — [#274](https://github.com/ACFHarbinger/Coding-Assistants/issues/274).** A concrete instance of P4. Separate from the C14.11 Muse Code *harness*: this is model inference the orchestrator can select as a role provider. Spike first — Meta's public posts do not confirm the endpoint path, model id string, auth scheme, or request/response schema; do not assume OpenAI-compatibility. Muse Glimmer (open-weight 30B, local) is an optional later addition, not in scope for #274. |
| P8 | Attach to an existing model process/service | A role can use an already-running OpenAI-compatible endpoint without spawning or terminating its process | 🚧 **Partial** · Endpoint mode and local Grok/Claude/Codex/Gemini process discovery are available; health checks, streaming, auth, and provider-specific adapters remain |
| P4 | Direct HTTP providers using the existing unused dependencies where useful | Provider health, structured errors, streaming, and usage accounting are tested | 📋 Pending |
| P5 | OS-level tool execution with configurable approval and relaxed-default sandbox | Every execution is audited and policy-controlled | 📋 Pending |
| P6 | LAN TCP authentication and later TLS | LAN remains available, but unauthorized clients are rejected | 📋 Pending |
| P7 | Local daemon and Unix-domain-socket API | Extract only after event/memory boundaries and a second client require it | 📋 Pending · later |
| P8 | GraphQL/WebSockets and actor framework evaluation | Adopt only if measured query/concurrency needs justify them | 💤 Maybe later |
| P9 | MCP external-server integration and promotion of frequently used tools | External MCP remains the lean default; promoted tools have security tests | 🚧 **Partial** · `hub::mcp::render_merged`/`render_replacing` render arbitrary server entries into each client config; `hub::mcp::creative` is a per-workspace catalog + `apply_to_workspace` for the 7 (now 8, incl. Ableton) creative-tool bridges. **#272 landed** (shared `hub::mcp::external` registry). **#276/#277 ready for review** — official **`@perplexity-ai/mcp-server`** (`npx -y`, `PERPLEXITY_API_KEY` from the client shell, never stored) and subscription **`perplexity-web-mcp-cli`** (`pwm-mcp`, `pwm login`, ~30-day quota-limited session, token-file presence only). Settings toggle remains #278 (Gemini). |
| P10 | Runtime budget pause/summary/shutdown behavior | Budget policy is tested end-to-end; affine typing remains postponed | 📋 Pending |
| P11 | A2A implementation support | Tracked primarily in `communication.md` as the next major milestone | 📋 Pending |

License work: implement the owner-approved dual AGPL-3.0 + Commercial scheme
with a dedicated legal/documentation review before release.

---

**2026-09-08 (Harbinger) — harness + MCP onboarding batch.** Parent
tracking issue [#270](https://github.com/ACFHarbinger/Coding-Assistants/issues/270).

- **P3 / C14.11** — Meta **Muse Code** managed harness ([#273](https://github.com/ACFHarbinger/Coding-Assistants/issues/273), owner: Muse). `communication.md` C14.11.
- **P4a** — Meta **Muse Spark 1.3** model provider ([#274](https://github.com/ACFHarbinger/Coding-Assistants/issues/274), owner: Muse).
- **P3 / C14.12** — **Cursor `agent`** managed harness ([#275](https://github.com/ACFHarbinger/Coding-Assistants/issues/275), owner: Cursor). `communication.md` C14.12.
- **P9 framework** — shared external-MCP-server registry ([#272](https://github.com/ACFHarbinger/Coding-Assistants/issues/272), owner: Claude) — generalises `hub::mcp::creative`; unblocks the two below.
- **P9** — Perplexity official API MCP ([#276](https://github.com/ACFHarbinger/Coding-Assistants/issues/276), owner: Grok) — **ready for review**.
- **P9** — Perplexity subscription web MCP ([#277](https://github.com/ACFHarbinger/Coding-Assistants/issues/277), owner: Grok) — **ready for review**.
- **Scaffold** — `HarnessId::{Muse,Cursor}` + roster identities + co-author trailers ([#271](https://github.com/ACFHarbinger/Coding-Assistants/issues/271), owner: Claude) — unblocks #273 / #275.
- **Settings UI** — toggle/configure the new MCP servers + surface the new harness rows ([#278](https://github.com/ACFHarbinger/Coding-Assistants/issues/278), owner: Gemini).
