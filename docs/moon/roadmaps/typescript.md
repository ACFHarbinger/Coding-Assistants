# Frontend (React/TypeScript) Roadmap

> Tracks planned work for `src/`. Sourced from
> [`docs/moon/research/Multi-Agent AI App Architecture.md`](../research/Multi-Agent%20AI%20App%20Architecture.md)
> and [`docs/moon/reports/AI Coding Tools Feature Report.md`](../reports/AI%20Coding%20Tools%20Feature%20Report.md).

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending

## Current State

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TS1 | Scaffold `package.json`, `tsconfig.json`, `src/App.tsx`, Vite build | S | ✅ Done |

## Track: GraphQL/WebSocket Client

Depends on the daemon API work in [`rust.md`](rust.md) (Track: API Layer).

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TG1 | GraphQL client wired to the daemon's mutation/query API (command invocation, historical logs, metric summaries) | M | 📋 Pending |
| TG2 | GraphQL Subscription client over WebSockets for live telemetry (token generation, tool-call/PTY output streaming, agent status) | M | 📋 Pending |

## Track: 2D Telemetry Dashboard

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TD1 | Dashboard view: line charts of aggregate token consumption per provider, correlated with task/time | M | 📋 Pending |
| TD2 | Progress rings for active agent/sub-agent completion velocity | S | 📋 Pending |
| TD3 | Heatmap of tool-invocation frequency per agent (file access vs. web queries vs. shell) | M | 📋 Pending |
| TD4 | Cost/spend view reflecting the backend's per-session `Budget` (see `rust.md` Track: Resource Management) | S | 📋 Pending |

## Track: 3D Force-Graph Visualization

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| T3D1 | Integrate `react-three-fiber` + `3d-force-graph` (or `react-force-graph`) as a new view rendering the live agent/MCP network in 3D | L | 📋 Pending |
| T3D2 | Node representation: distinct geometry per entity type (agent vs. context/memory block vs. MCP server) | M | 📋 Pending |
| T3D3 | Edge representation: force-directed links driven by live GraphQL subscription events, with particle flow density/speed reflecting message volume | L | 📋 Pending |
| T3D4 | Dynamic clustering: force-directed physics pulling collaborating entities together (e.g. two agents editing the same file) | M | 📋 Pending |
| T3D5 | Node click-through overlay panels: token usage, active system prompt, real-time execution log for the selected agent | M | 📋 Pending |

## Track: Human-in-the-Loop UI

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TH1 | Approval modal for destructive tool calls, surfacing the exact command/args from the backend's `RS1` approval gate (see `rust.md`) | M | 📋 Pending |
