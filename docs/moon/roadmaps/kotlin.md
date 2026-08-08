# Android Companion App (Kotlin) Roadmap

> Tracks planned work for `android/`. Sourced in part from
> [`docs/moon/research/Multi-Agent AI App Architecture.md`](../research/Multi-Agent%20AI%20App%20Architecture.md).

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending

## Current State

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| K1 | Scaffold `build.gradle.kts`, `app/src/`, TCP client to pair with the desktop app | S | ✅ Done |

## Track: Daemon API Alignment

Depends on [`rust.md`](rust.md) Track: API Layer.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| KA1 | Evaluate migrating the companion app's raw TCP client to the GraphQL/WebSocket API once the Core Orchestration Daemon lands, so desktop, web, and mobile clients share one API surface | M | 📋 Pending |
| KA2 | If migrated: subscribe to the same live telemetry (token usage, agent status) the GUI dashboard consumes, for a lightweight mobile monitoring view | M | 📋 Pending |
