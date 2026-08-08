# Terminal UI (Ratatui) Roadmap

> A new planned interface, not yet scaffolded — no directory exists for it
> yet (would likely live in a new `tui/` crate alongside `src-tauri/`, sharing
> the daemon once [`rust.md`](rust.md)'s Core Orchestration Daemon track
> lands). Sourced from
> [`docs/moon/research/Multi-Agent AI App Architecture.md`](../research/Multi-Agent%20AI%20App%20Architecture.md)
> and [`docs/moon/reports/AI Coding Tools Feature Report.md`](../reports/AI%20Coding%20Tools%20Feature%20Report.md).

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending

Depends on [`rust.md`](rust.md) Track: Core Orchestration Daemon and Track:
API Layer — the TUI is a second client of the same daemon, not a
reimplementation of the agent logic.

## Track: Core TUI

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TU1 | Scaffold a `ratatui`-based binary crate; connect to the daemon over a local socket/channel (bypassing network serialization for low-latency local use) | L | 📋 Pending |
| TU2 | Agent multiplexer layout: resizable panes, each tracking one agent or telemetry stream, `tmux`/`zellij`-style | L | 📋 Pending |
| TU3 | Embed PTY-backed panes (reusing the daemon's `RD5` PTY integration) so raw ANSI-colored tool output renders live in a pane | M | 📋 Pending |
| TU4 | Evaluate `reratui` (React-Fiber-style component/hook model for Ratatui) to keep the TUI's component mental model close to the React GUI's | M | 📋 Pending |
| TU5 | Hot-reloaded JSON config (theme hex values, scroll/interaction physics, notification hooks) mirroring OpenCode's `tui.json` approach | M | 📋 Pending |

## Track: Syntax Highlighting & Semantic Diffs

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| TU6 | Syntax highlighting via `syntect` (Sublime Text grammars), translated to Ratatui spans via a `syntect`-to-Ratatui bridge crate | M | 📋 Pending |
| TU7 | Viewport-scoped highlighting with cached parse state per visible region, to hold ~60fps on large files | M | 📋 Pending |
| TU8 | Semantic diff view: cluster changed hunks by intent (e.g. "refactor auth" vs. "CSS update") rather than strict file-path order | L | 📋 Pending |
| TU9 | Tree-sitter-backed fold/expand navigation for diff context, with `hjkl` keyboard navigation | M | 📋 Pending |
