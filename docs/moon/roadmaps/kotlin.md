# Android Companion App (Kotlin) Roadmap

> Tracks planned work for `android/`.
> **Owner 2026-08-10:** Android is **after** the desktop hub is mostly feature
> complete. Highest mobile value: **watch** agent activity; next: **send messages**.
> Full task configuration stays on desktop. Mainly monitoring/approval.

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending · 💤 Someday/Maybe

## Current State

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| K1 | Scaffold `build.gradle.kts`, `app/src/`, TCP client to pair with the desktop app | S | ✅ Done |

## Track: Monitor & Message (priority after desktop hub)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| KM1 | Reliable live activity stream (agent events, status) for watch-only use | M | 📋 Pending · after desktop hub |
| KM2 | Send short messages / approvals to running agents from Android | M | 📋 Pending · after KM1 |
| KM3 | TCP auth/token support when desktop RS6 lands (keep LAN for now) | M | 📋 Pending |

## Track: Daemon / multi-client API Alignment

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| KA1 | Evaluate migrating raw TCP client to whatever multi-client API ships (UDS bridge, WS, or GraphQL-later) | M | 📋 Pending · later |
| KA2 | Lightweight mobile monitoring of hub telemetry (tokens, agent status) | M | 📋 Pending · later |

## Explicitly not near-term

- Full task/role configuration on mobile (desktop-only for now)
- Replacing desktop as primary control surface
