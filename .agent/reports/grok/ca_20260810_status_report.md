# Coding-Assistants — Grok (Build) Status Report

**Date:** 2026-08-10  
**Author:** Grok (Build / xAI)  
**Scope:** Code + docs analysis, product feedback, owner Q&A synthesis, roadmap recommendations  
**Authority:** Independent agent report under `.agent/reports/grok/`. Does not supersede owner decisions.

---

## 1. Executive summary

Coding-Assistants (CA) today is a **working alpha Tauri desktop app** that runs a **sequential multi-role LLM pipeline** (CLI providers, marker-based HITL, Android TCP remote). The documentation and moon research describe a far larger **orchestration daemon + GraphQL + MCP/A2A + 3D** platform.

The owner’s Q&A session **re-centers the product**: CA is a **collaboration hub for external coding agents (Claude Code, Codex, Gemini/Antigravity, Grok Build, OpenCode, Ollama, llama.cpp) plus a human developer**, not a self-contained multi-role experiment. The sequential in-app roles were an initial experiment only. v1 is **personal / solo power-developer**, local-first with later cloud sync, and success in 30 days is measured by **shipping quality work together on a real repo** (e.g. Project-Mobile-Fortress) that **matches or exceeds any single teammate alone**.

**Grok’s judgment:** Keep Tauri/Rust, ADR 0003 (event bus before daemon extract), and the scaffolding discipline. **Change the center of gravity** to durable multi-agent memory + messaging + configurable gates. Demote GraphQL, actors, A2A, 3D, TUI, and heavy infra. Next milestone should be “hub spine,” not “empty cathedral.”

---

## 2. What was read / evidence base

| Area | Paths | Notes |
| --- | --- | --- |
| Backend | `src-tauri/src/{lib,agents,llm_client,tcp_server,file_tools}.rs` | ~1,350 LOC total |
| Frontend | `src/App.tsx` (~900), `index.css` | Monolith UI |
| Android | `android/...` ~1k Kotlin | TCP client |
| Product docs | `docs/ARCHITECTURE.md`, `SECURITY.md`, dual roadmaps | Honest security gaps |
| Future | `docs/moon/**`, ADRs 0002–0003 | Research-forward |
| Process | `.agent/cache/*` merge experiment | Live multi-agent coord pilot |
| Owner Q&A | All Claude / Gemini / Grok / Chat answer sets (this session) | Binding for roadmap edits |

---

## 3. Current implementation — pros

| Keep | Why |
| --- | --- |
| End-to-end vertical slice | UI → IPC → multi-role → stream → HITL → Android is rare for alpha |
| CLI provider seam | Fits “harness of choice”; OpenCode/ollama path exists |
| `KillOnDrop` + cancel token | Process hygiene |
| Marker HITL (`[[ASK_USER]]` / `[[ASK_AGENT]]`) | Simple, demos collaboration |
| `.agent/` file contract | Matches how external agents already work on disk |
| ADR 0003 / RD7 direction | Right: broadcast bus before physical daemon split |
| `governor` rate limit (RB1) | Real cost/throttling concern addressed early |
| `tokio::fs` for file tools (RD3) | Non-blocking I/O |
| Honest SECURITY.md | Documents TCP no-auth, CSP null, path policy |
| Multi-agent meta-tooling | Reports/cache/messages conventions — product demo in itself |

---

## 4. Current implementation — cons

| Change | Why |
| --- | --- |
| Wrong product abstraction | Sequential roles ≠ hub for external agent sessions |
| No durable cross-agent mailbox | Today’s multi-agent report experiment had to invent bus files |
| Memory = overwrite `project_memory.md` | Not multi-tier, multi-writer, or multi-scope |
| CLI stdout scraping | Brittle; no reliable tools/tokens/cost |
| MCP is write-`mcp.json` only | Not host/client protocol |
| Global `AppState` races | Concurrent tasks clobber cancel/input |
| Security holes | Path traversal, `read_file_absolute`, LAN TCP no auth |
| Dual unreconciled roadmaps | Agents thrash |
| Scaffolding bloat | k8s/helm/wordpress for a local desktop app |
| No tests | Refactors will regress silently |
| LM Studio stub | Advertised, returns error |

---

## 5. Plans (`docs/moon` + research) — pros

- Headless multi-client eventual shape matches “desktop + Android (+ later cloud).”
- RD5/RD6 (PTY + stream-json) needed for real agent CLIs.
- RS1 HITL approval before destructive tools.
- Memory tiers (RP*) right *direction*, wrong *scope* (single-agent framing).
- Short-term `docs/ROADMAP.md` items (parallel, memory, tests) were underweighted in moon.

---

## 6. Plans — cons

- GraphQL/A2A/actors/3D/affine budgets as near-term noise for a personal hub.
- Research docs risk being treated as committed architecture.
- Scaffolding tracks marked ✅ while product identity was wrong.
- No first-class “session bridge / wake / shared inbox” track until owner Q&A.

---

## 7. Owner Q&A — product contract (Grok synthesis)

### DECIDED (owner)

| ID | Decision |
| --- | --- |
| PC-identity | **Collaboration hub** for external agents + human; multi-role LLM app was experiment only |
| PC-audience | Personal / solo power developer now; later org utility + cloud sync |
| PC-success-30d | Joint task on real repo (e.g. Project-Mobile-Fortress) quality ≥ best single member |
| PC-replace-cli | CA becomes the daily driver UI; harness (direct tool vs OpenCode router) deferred to agents |
| PC-android | After desktop mostly complete; watch changes high; send messages slightly lower |
| PC-providers-v1 | Claude Code, Codex CLI, Gemini/Antigravity, Grok Build, OpenCode, Ollama, llama.cpp |
| PC-collab | **Async first**, then parallel; sequential/parallel discussion with clear role boundaries |
| PC-wake-gate | **Configurable** human gate (session/task setting) |
| PC-auth | Prefer **standing policies** over per-message modals |
| PC-memory | **Hybrid**: local DB long-term + git-tracked markdown for high-priority/high-importance |
| PC-memory-tiers | Multi-tier incl. compressed long-term + richer recent; both **global + per-repo** scopes |
| PC-conflict | Conflict detection **user setting** |
| PC-reports-path | `.agent/reports/` + messages = **temporary** convention |
| PC-adr0003 | **Endorse** event bus first, no daemon crate yet |
| PC-graphql | **Maybe later** |
| PC-actors | Interest, **later** |
| PC-tui | Nice-to-have / experiment / secondary |
| PC-3d | **Research only**; 2D observability first |
| PC-api | Unix domain socket without GraphQL **OK**; not required that every client share one protocol |
| PC-stack | Free to switch if product fit better |
| PC-tcp | Keep LAN for now; auth/TLS **future** roadmap item |
| PC-tools | Agents **may execute** tools/commands; permission **setting** |
| PC-sandbox | User setting; default **relaxed** for now |
| PC-cost | Default telemetry + soft warning; optional hard kill; budget exhaustion → pause, summarize MD, delegate, shutdown until wake |
| PC-wiring | Declarative agent wiring first; dynamic A2A later |
| PC-mcp | Lean external MCP first; promote hot tools into core later |
| PC-priority | Next milestone weight: **Memory** |
| PC-roadmaps | Moon primary; fold old ROADMAP items; additive + deprioritize (not hard-delete ideas); archive/ for speculative; separate research/impl/infra maps OK |
| PC-infra | Keep **docker, terraform, ansible**; mark rest for trim/delete (do not require bulk delete in this commit if staged as roadmap item) |
| PC-license | Dual **AGPL-3.0 + Commercial** (Project-Mobile-Fortress scheme) |
| PC-naming | Keep `tauri-app` package names |
| PC-agent-write | For now agents write under own `.agent/reports/{name}` or shared |

### OPEN / agent-owned

| Topic | Note |
| --- | --- |
| True session resume vs inject | Owner deferred to agents — Grok lean: **inject + durable transcript first**; resume IDs as adapter-specific enhancement |
| Provider harness | Owner deferred — Grok lean: **stream-json / headless CLI adapters + wire unused `async-openai`/`reqwest` for cloud HTTP** dual path |
| Trust levels / identities | Owner wants further analysis |

---

## 8. Keep / Change / Archive (Grok recommendations)

### Keep

1. Tauri + Rust privileged backend  
2. ADR 0003 / RD7 event bus path  
3. Rate limiting + expand to soft/hard budgets  
4. `.agent/` as human+AI contract surface (evolve protocol, don’t abandon files entirely for markdown tier)  
5. Android as thin remote (monitor/approve) after desktop hub  
6. Multi-tier roadmaps under `docs/moon/`  

### Change (with avenues)

| What | Why | Avenues |
| --- | --- | --- |
| Product model → hub objects | Owner identity | A) Hub-first rewrite of types; B) Dual-mode hub+pipeline; C) Library-first `ca-core` |
| Cross-agent memory + messaging | #1 milestone | A) SQLite + markdown hybrid; B) files-only inbox v0; C) SQLite only |
| Wake signal ≠ durable store | Owner + Claude R.2 | A) file-watch; B) UDS; C) both |
| Shared CLI helper | Any agent can R/W/poll | A) `ca` binary; B) MCP tools; C) both |
| Standing policies + per-task gates | Owner R.9–10 | Policy file + UI settings |
| Wire HTTP providers | Owner R.6 | Use existing Cargo deps; trait over CLI+HTTP |
| Demote 3D/TUI/A2A/GraphQL/actors | Owner + Gemini | Someday/Maybe tags, not delete |
| Security backlog | Claude R.4 | AppState per-task, mcp path, TCP auth |
| Dual license | Owner R.32 | Copy PMF dual-license pattern |

### Archive / demote (not delete)

- T3D* 3D force-graph → Research / Someday  
- Full `tui.md` → Someday/Maybe  
- GraphQL as first multi-client API → later alternative after UDS/JSON  
- A2A → strategically interesting backlog  
- Affine compile-time budgets → after runtime budgets + pause-on-exhaustion policy  

**Not recommended:** rewrite core in C++ (bottleneck is LLM latency and product model, not native parse speed).

---

## 9. Competing milestone plans (for joint decision)

Owner asked for **several competing plans**, then pick one together.

### Plan Alpha — “Memory Hub First” (Grok recommended)

**Goal (30 days):** Shared SQLite + markdown memory, agent identities, async mailbox, tiny `ca` CLI helper, wake via file-watch, standing policies stub, RD7 event bus, 2D activity log polish. Joint Fortress task as acceptance demo.

| Week focus | Exit (acceptance every ~5 items) |
| --- | --- |
| W1 | Schema + CLI read/write/poll; identity attribution |
| W2 | Markdown tier for high-priority; dual scope global/repo |
| W3 | Wake signal + gate settings; transcripts with TTL |
| W4 | RD7 bus; desktop UI for inbox/memory; Fortress dry-run |

### Plan Beta — “Harness & Tools First”

Adapters for all seven providers with stream-json where possible; tool execution with permission setting; path sandbox setting; wire HTTP APIs. Memory stays thin (append-only log).

**Risk:** Still thrash without shared memory (this session’s bus problem).

### Plan Gamma — “Daemon Early”

UDS daemon + thin Tauri client early; multi-client from day one.

**Risk:** Delays memory; owner endorsed ADR 0003 (bus first).

### Plan Delta — “Security Hardening First”

AppState race, path traversal, TCP auth, CSP — before features.

**Risk:** Correct but doesn’t prove multi-agent collab value in 30 days.

**Grok vote:** **Alpha**, with Delta’s P0 bugs as **must-fix backlog items inside Alpha** (not a separate multi-week plan).

---

## 10. Roadmap edits performed / intended by Grok this pass

See commit for:

1. New track **Cross-Agent Shared Memory & Coordination** (priority above daemon).  
2. Fold short-term `docs/ROADMAP.md` items into moon per-area files; **stub** `docs/ROADMAP.md` → `docs/moon/ROADMAP.md`.  
3. Demote T3D* and TUI to **Someday/Maybe**.  
4. Backlog: AppState single-task, global mcp.json race, TCP auth.  
5. Scaffolding: mark infra trim (keep docker/terraform/ansible).  
6. Wire unused Cargo deps to direct HTTP API item (not drop).  
7. Budget RB3 semantics: pause + summary + delegate + shutdown until wake.  
8. GraphQL / actors / A2A tagged later.  
9. Android priorities: monitor/approve first.  
10. License dual AGPL+Commercial as roadmap/docs note (full LICENSE copy can follow).  
11. `docs/moon/archive/` for speculative placement pointer.  

---

## 11. Session resume OPEN (agent discussion lean)

| Approach | Pros | Cons |
| --- | --- | --- |
| Message + context injection only | Portable across tools | Loses tool-native memory |
| True session resume | Continuity | Per-vendor fragile |
| Hybrid | Resume when API allows; else inject transcript summary | Complexity |

**Grok lean:** Hybrid with **transcript always durable**; resume IDs opportunistic.

---

## 12. Closing opinion

The merge-coordination experiment proved the product need: **without a single discoverable bus and durable memory, agents invent parallel channels**. Owner answers correctly put **Memory + Coordination** above the orchestration cathedral.

If CA becomes the only UI the owner uses for Claude/Codex/Gemini/Grok, the acceptance test is not “GraphQL subscriptions work” — it is “we shipped Fortress features together that none of us would alone, without re-explaining context every session.”

I agree with the **shared report structure** (canonical `ca_20260810_shared_report.md`) for owner fill after decisions are recorded. Independent reports under per-agent dirs remain the deep evidence base.

---

## 13. Changelog (this report)

| Date | Change |
| --- | --- |
| 2026-08-10 | Initial Grok report from full analysis + complete owner Q&A |
