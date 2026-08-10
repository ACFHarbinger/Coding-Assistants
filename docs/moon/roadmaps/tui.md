# Terminal UI (Ratatui) Roadmap — 💤 Someday/Maybe

> **Deprioritized (owner Q&A 2026-08-10):** TUI is a **nice-to-have / experiment**,
> not a Day-1 or near-term primary interface. Build only after the desktop hub
> (memory, messaging, adapters) is useful daily. Items are **not deleted**.
>
> Would likely live in a new `tui/` crate sharing the daemon/local bus once
> [`rust.md`](rust.md) multi-client work lands. See also
> [`../archive/README.md`](../archive/README.md).

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending · 💤 Someday/Maybe

## Track: Core TUI

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TU1 | Scaffold `ratatui` binary; connect over local socket/channel | L | 💤 Someday/Maybe |
| TU2 | Agent multiplexer layout (tmux/zellij-style panes) | L | 💤 Someday/Maybe |
| TU3 | PTY-backed panes (reuse RD5) | M | 💤 Someday/Maybe |
| TU4 | Evaluate `reratui` component model | M | 💤 Someday/Maybe |
| TU5 | Hot-reloaded JSON theme/config | M | 💤 Someday/Maybe |

## Track: Syntax Highlighting & Semantic Diffs

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TU6 | Syntax highlighting via `syntect` | M | 💤 Someday/Maybe |
| TU7 | Viewport-scoped highlighting ~60fps | M | 💤 Someday/Maybe |
| TU8 | Semantic diff by intent clusters | L | 💤 Someday/Maybe |
| TU9 | Tree-sitter fold/expand + hjkl navigation | M | 💤 Someday/Maybe |
