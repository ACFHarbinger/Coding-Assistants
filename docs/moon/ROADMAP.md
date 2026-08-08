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
| T5 | `infra/docker/` infrastructure: Dockerfile, Compose stack (docs site) | S | ✅ Done |
| T6 | `infra/{k8s,helm,terraform,ansible}/` — additional infra-as-code scaffolding | M | ✅ Done |
| T7 | `.agent/` LLM coding-agent scaffolding | M | ✅ Done |
| T8 | `justfile` + `tools/` command runner | M | ✅ Done |
| T9 | `.devcontainer/` Dev Container definition | S | ✅ Done |
| T10 | `git/` repo-process automation (hooks, backlog sync, label taxonomy) | M | ✅ Done |
| T11 | Editor settings (`settings/`: VS Code, IDEA, Sublime, Obsidian) | S | ✅ Done |
| T12 | Interactive documentation website (`docs/website/`) | M | ✅ Done |

## Track: Application

See per-area detail in [`docs/moon/roadmaps/`](roadmaps/): [frontend](roadmaps/typescript.md), [backend](roadmaps/rust.md), [Android companion app](roadmaps/kotlin.md).
