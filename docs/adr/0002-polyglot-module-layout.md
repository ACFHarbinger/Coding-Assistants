# 2. Single-app layout, not one directory per language

Date: 2026-08-08

## Status

Accepted

## Context

The scaffolding this repo's tooling was adapted from (Tauri-App-Template)
defaults to one top-level directory per language for polyglot projects
generated from it. Coding-Assistants isn't a polyglot multi-service
project — it's a single Tauri desktop app (Rust backend + React/TypeScript
frontend) with an optional Kotlin/Android companion app, following Tauri's
own conventional layout.

## Decision

Keep Tauri's standard layout: `src/` for the React/TypeScript frontend,
`src-tauri/` for the Rust backend, and `android/` for the companion app,
each with its own dependency manifest (`package.json`, `Cargo.toml`,
`build.gradle.kts`). Repo-process tooling (`git/`, `tools/`, `infra/`,
`docs/`) stays generic/shared rather than split per language.

## Consequences

- CI, `.pre-commit-config.yaml`, and `justfile` recipes key off these three
  fixed paths (`src/`, `src-tauri/`, `android/`) rather than a discoverable
  list of language directories.
- A future hosted service (if ever added) gets its own top-level directory
  and its own ADR, rather than retrofitting this one.
