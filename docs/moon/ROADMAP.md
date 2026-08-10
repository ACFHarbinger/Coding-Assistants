# Coding-Assistants Roadmap

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)

> **Version**: 1.1
> **Date**: 2026-08-10
> **Product identity (owner):** local-first **collaboration hub** for a human
> developer and external coding agents (Claude Code, Codex, Gemini/Antigravity,
> Grok Build, OpenCode, Ollama, llama.cpp). The in-process multi-role pipeline
> was an initial experiment only.

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending · 💤 Someday/Maybe

This is the **canonical roadmap index**. Per-area detail lives under
[`roadmaps/`](roadmaps/). Historical short-term checklist content from the old
`docs/ROADMAP.md` has been folded into those files; `docs/ROADMAP.md` is a
pointer stub.

---

## Priority ordering (2026-08-10)

1. **Cross-Agent Shared Memory & Coordination** — [`roadmaps/hub.md`](roadmaps/hub.md)
2. **Backend reliability / security / providers** — [`roadmaps/rust.md`](roadmaps/rust.md)
3. **Frontend hub UX (2D, approvals, component split)** — [`roadmaps/typescript.md`](roadmaps/typescript.md)
4. **Core Orchestration Daemon** (after RD7 bus; no early crate split — ADR 0003)
5. **Android companion** (monitor/approve after desktop hub) — [`roadmaps/kotlin.md`](roadmaps/kotlin.md)
6. **💤 Someday/Maybe:** TUI, 3D force-graph, GraphQL-first API, A2A, actor framework

---

## Track: Repo Scaffolding

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| T1 | Root scaffolding: LICENSE, README, .env.example, git config, pre-commit | S | ✅ Done |
| T2 | `.github/`, `.forgejo/`, `.gitea/`, `.gitlab/` CI/CD | M | ✅ Done |
| T3 | `docs/` documentation portal: MkDocs, Structurizr, ADRs | M | ✅ Done |
| T4 | `docs/moon/` roadmap and changelog | S | ✅ Done |
| T5 | `infra/global/docker/` docs site stack | S | ✅ Done |
| T6 | `infra/{k8s,helm,terraform,ansible}/` and other IaC scaffolding | M | ✅ Done |
| T6b | **Trim infra:** keep `docker`, `terraform`, `ansible`; mark for removal/archive: k8s, helm, serverless, firebase, aws, azure-pipelines, wordpress, webpack, nginx, proxy (owner 2026-08-10; execute as separate PR) | M | 📋 Pending |
| T7 | `.agent/` LLM coding-agent scaffolding | M | ✅ Done |
| T8 | `justfile` + `tools/` command runner | M | ✅ Done |
| T9 | `.devcontainer/` Dev Container definition | S | ✅ Done |
| T10 | `git/` repo-process automation | M | ✅ Done |
| T11 | Editor settings (`settings/`) | S | ✅ Done |
| T12 | Interactive documentation website (`docs/website/`) | M | ✅ Done |
| T13 | Dual license AGPL-3.0 + Commercial (Project-Mobile-Fortress scheme) | S | 📋 Pending |
| T14 | Speculative ideas live under [`archive/`](archive/) rather than deletion | S | 📋 Pending |

---

## Track: Application (index)

| Area | Roadmap | Near-term focus |
| --- | --- | --- |
| **Hub / memory / coord** | [`roadmaps/hub.md`](roadmaps/hub.md) | SQLite + markdown hybrid, CLI helper, wake, policies, adapters |
| Backend | [`roadmaps/rust.md`](roadmaps/rust.md) | RD7 bus, HTTP providers, budgets, security backlog, testing |
| Frontend | [`roadmaps/typescript.md`](roadmaps/typescript.md) | Component split, 2D observability, approval UI, memory/inbox views |
| Android | [`roadmaps/kotlin.md`](roadmaps/kotlin.md) | Watch agent activity; later send messages; after desktop hub |
| TUI | [`roadmaps/tui.md`](roadmaps/tui.md) | 💤 Someday/Maybe (experiment only) |

---

## Track: Target Architecture (Multi-Agent Orchestration Daemon)

Longer-term direction from research under [`research/`](research/) and
[`reports/`](reports/). **Not** the next milestone. Owner endorsed
[ADR 0003](../adr/0003-daemon-extraction-spike.md): internal event bus first;
physical daemon extract later; GraphQL **maybe later**; Unix domain socket API
acceptable; actors later.

| Area | Roadmap | Status note |
| --- | --- | --- |
| Backend daemon / API | [`roadmaps/rust.md`](roadmaps/rust.md) | RD7 first; RA* GraphQL delayed; RD2 actors delayed |
| Frontend GraphQL / 3D | [`roadmaps/typescript.md`](roadmaps/typescript.md) | TG* later; T3D* research/someday |
| TUI | [`roadmaps/tui.md`](roadmaps/tui.md) | 💤 entire file |
| Android API migration | [`roadmaps/kotlin.md`](roadmaps/kotlin.md) | After hub + desktop |

---

## 30-day success criterion (owner)

Complete a multi-agent + human task on a real repository (example:
Project-Mobile-Fortress) with joint quality (UI, gameplay, art, dashboards, etc.)
that **matches or exceeds** any single teammate alone. Define concrete
benchmarks during the week of 2026-08-10.

---

## Competing milestone plans

See agent reports under `.agent/reports/{grok,chat,claude,gemini}/` for Alpha
(Memory Hub First), Beta (Harness First), Gamma (Daemon Early), Delta (Security
First). **Owner + agents pick one** after reports; Grok recommends Alpha with
security P0 bugs embedded.
