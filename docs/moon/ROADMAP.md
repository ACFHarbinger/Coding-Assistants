# Coding-Assistants Roadmap

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)

> **Version**: 1.0
> **Date**: 2026-08-08

## Overview

This document tracks planned scaffolding/tooling work for the Coding-Assistants
repo itself (not application features — see the per-area roadmaps in
[`docs/moon/roadmaps/`](roadmaps/) and [`docs/moon/CHANGELOG.md`](CHANGELOG.md)
for the application's own history).

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending

---

## Track: Repo Scaffolding

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| T1 | Root scaffolding: LICENSE, README, .env.example, git config, pre-commit | S | ✅ Done |
| T2 | `.github/`, `.forgejo/`, `.gitea/`, `.gitlab/` CI/CD: workflows, issue/PR templates, dependabot | M | ✅ Done |
| T3 | `docs/` documentation portal: MkDocs, Structurizr, ADRs | M | ✅ Done |
| T4 | `docs/moon/` roadmap and changelog | S | ✅ Done |
| T5 | `infra/global/docker/` infrastructure: Dockerfile, Compose stack (docs site) | S | ✅ Done |
| T6 | `infra/{k8s,helm,terraform,ansible}/` — additional infra-as-code scaffolding | M | ✅ Done |
| T7 | `.agent/` LLM coding-agent scaffolding | M | ✅ Done |
| T8 | `justfile` + `tools/` command runner | M | ✅ Done |
| T9 | `.devcontainer/` Dev Container definition | S | ✅ Done |
| T10 | `git/` repo-process automation (hooks, backlog sync, label taxonomy) | M | ✅ Done |
| T11 | Editor settings (`settings/`: VS Code, IDEA, Sublime, Obsidian) | S | ✅ Done |
| T12 | Interactive documentation website (`docs/website/`) | M | ✅ Done |

## Track: Application

See per-area detail in [`docs/moon/roadmaps/`](roadmaps/): [frontend](roadmaps/typescript.md), [backend](roadmaps/rust.md), [Android companion app](roadmaps/kotlin.md).

## Track: Target Architecture (Multi-Agent Orchestration Daemon)

Longer-term direction, synthesized from
[`docs/moon/research/Multi-Agent AI App Architecture.md`](research/Multi-Agent%20AI%20App%20Architecture.md)
and [`docs/moon/reports/AI Coding Tools Feature Report.md`](reports/AI%20Coding%20Tools%20Feature%20Report.md).
The current app (a single Tauri process calling `agents.rs`/`llm_client.rs`
directly via `invoke()`) evolves into a headless Core Orchestration Daemon
(Rust/Tokio, actor model) exposing a GraphQL-over-WebSockets API, with the
existing React GUI, a new [Ratatui TUI](roadmaps/tui.md), and the Android
companion app all as clients of that one daemon. Items are tracked in the
relevant per-area roadmap rather than duplicated here:

| Area | Roadmap | Key new tracks |
| --- | --- | --- |
| Backend | [`roadmaps/rust.md`](roadmaps/rust.md) | Core Orchestration Daemon, GraphQL API, MCP + A2A protocols, rate limiting + affine budgets, two-tier memory, human-in-the-loop security |
| Frontend | [`roadmaps/typescript.md`](roadmaps/typescript.md) | GraphQL/WebSocket client, 2D telemetry dashboard, 3D force-graph visualization, approval-gate UI |
| Terminal UI | [`roadmaps/tui.md`](roadmaps/tui.md) *(new interface, not yet scaffolded)* | Ratatui multiplexer, PTY panes, syntax highlighting, semantic diffing |
| Android companion | [`roadmaps/kotlin.md`](roadmaps/kotlin.md) | Migrate TCP client to the daemon's GraphQL/WebSocket API |

All items in this track are currently 📋 Pending and unsequenced beyond the
dependency notes in each roadmap file (API layer before its clients, etc.).
