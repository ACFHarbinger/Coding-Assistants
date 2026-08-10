# Frontend (React/TypeScript) Roadmap

> Tracks planned work for `src/`. Updated 2026-08-10 for hub-first product
> identity. GraphQL client work depends on a future multi-client API (maybe
> later); 3D visualization is research/someday only. **2D observability first.**

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending · 💤 Someday/Maybe

## Current State

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TS1 | Scaffold `package.json`, `tsconfig.json`, `src/App.tsx`, Vite build | S | ✅ Done |

---

## Track: Hub UX (near-term)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| THU1 | Component extraction — split `App.tsx` into panels (config, events, output, memory/inbox, remote) | M | 📋 Pending |
| THU2 | Inbox / message browser for cross-agent durable messages (hub store) | M | 📋 Pending |
| THU3 | Memory browser: search, edit, delete/stale, scope toggle (global vs workspace) | M | 📋 Pending |
| THU4 | Standing-policy editor UI (who may wake/ask whom) | M | 📋 Pending |
| THU5 | Per-task settings: human gate, tool permission, sandbox strictness, worktree isolation | M | 📋 Pending |
| THU6 | Task history sidebar; re-run previous tasks | M | 📋 Pending |
| THU7 | Keyboard shortcuts for common actions | S | 📋 Pending |
| THU8 | Theme toggle (beyond dark-only glass-morphism) | S | 📋 Pending |
| THU9 | Frontend test suite (Vitest + React Testing Library) | M | 📋 Pending |

---

## Track: 2D Telemetry / Observability

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TD1 | Dashboard: token/cost lines per provider vs time | M | 📋 Pending |
| TD2 | Progress rings for active agent completion | S | 📋 Pending |
| TD3 | Heatmap of tool-invocation frequency | M | 📋 Pending |
| TD4 | Cost/spend view reflecting budget soft/hard settings | S | 📋 Pending |
| TD5 | **2D DAG / graph of agent collaboration** (SVG or similar) for debugging — preferred over 3D | M | 📋 Pending |

---

## Track: Human-in-the-Loop UI

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TH1 | Approval modal for tool calls when policy requires it (exact command/args) | M | 📋 Pending |
| TH2 | Budget-exhaustion modal: authorize extension or accept summary + shutdown | M | 📋 Pending |

---

## Track: Multi-client API consumer

Depends on backend multi-client work; GraphQL not required near-term.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TG0 | Optional client for local UDS/JSON API if daemon lands | M | 📋 Pending · after RD20 |
| TG1 | GraphQL client for mutations/queries | M | 💤 Someday/Maybe · maybe later |
| TG2 | GraphQL subscriptions over WebSockets | M | 💤 Someday/Maybe |

---

## Track: 3D Force-Graph Visualization — 💤 Research / Someday

**Rationale (owner 2026-08-10):** not a V1 requirement; research only. Prefer
2D observability (TD5). Items retained for later evaluation — not deleted.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| T3D1 | `react-three-fiber` + force-graph live agent/MCP network | L | 💤 Someday/Maybe |
| T3D2 | Node geometry per entity type | M | 💤 Someday/Maybe |
| T3D3 | Edge particle flow from live events | L | 💤 Someday/Maybe |
| T3D4 | Dynamic clustering of collaborators | M | 💤 Someday/Maybe |
| T3D5 | Node click-through overlay panels | M | 💤 Someday/Maybe |
