# Kubuntu / KDE Konsole Acceptance Checklist for `ca tui` (U7 / T1–T8)

> **Reference:** Roadmap U7 ([`docs/moon/roadmaps/ui.md`](../roadmaps/ui.md)), GitHub Issues [#134](https://github.com/ACFHarbinger/Coding-Assistants/issues/134) through [#142](https://github.com/ACFHarbinger/Coding-Assistants/issues/142).  
> **Target Environment:** Linux Kubuntu (KDE Plasma + Konsole).

---

## 1. Automated Test Suite Verification

Run the automated test suite in the repository worktree:

```bash
cargo test -p tui
```

Verify that all 45+ tests across the test binaries pass cleanly:
- `danger_memory_audit_test.rs` (T5): Cancel-first danger modals, exact target confirmations, memory search, truthful budgets.
- `harness_panes_test.rs` (T6): Owned interactive PTY lifecycle, observed session read-only enforcement, prefix chords (`Ctrl+B`), VT/ANSI parsing.
- `multi_instance_coherence_test.rs` (T7): Version-stamped reject-and-refresh, conflict banner, refresh and retry, escape dismissal.
- `konsole_resilience_test.rs` (T8): Narrow (<80 cols) and wide (≥120 cols) layouts, UTF-8/ASCII fallback, no foreign PID attach, C10–C13 desktop parity without markdown bus writes.
- `navigation_test.rs`, `options_test.rs`, `session_and_orchestration_test.rs`, `settings_and_profiles_test.rs`, `model_test.rs`.

---

## 2. Interactive Terminal Acceptance Checklist

Launch the TUI client in a local Konsole window:

```bash
cargo run -p tui-bin --
```

### A. Terminal Lifecycle & Mouse-Off Selection
- [ ] **Startup & Alternate Screen**: TUI initializes in the alternate screen with raw mode enabled.
- [ ] **Mouse-Off Native Selection**: Left-click and drag inside Konsole to select text. Verify native selection highlights text and middle-click pastes outside or inside without mouse-capture interception.
- [ ] **Clean Exit**: Press `q`, `Ctrl+C`, or `:quit` in command palette. Verify Konsole returns cleanly to the primary screen buffer with cursor visible and canonical mode restored.
- [ ] **Panic Restoration**: Terminal cleanup hook resets raw mode and restores terminal if an internal panic occurs.

### B. Responsive Layouts & Character Sets
- [ ] **Standard (80×24)**: Top header, tab bar (`1: Orchestrate`, `2: Chat & Memory`, `3: Shared Hub`, `4: Settings`, `5: Harness Panes`), active panel, and bottom status bar render without overlapping or clipped borders.
- [ ] **Narrow (<80 cols, e.g. 70×24)**: Resize Konsole window narrower than 80 columns. Layout adapts gracefully to single-column views. Split tiles in Harness Panes collapse automatically to single-pane view.
- [ ] **Wide (≥120 cols, e.g. 140×40)**: Maximize or widen Konsole window. Layout expands cleanly. Harness Panes split mode renders dual side-by-side terminal viewports.
- [ ] **UTF-8 vs ASCII Fallback**: In standard UTF-8 locale, header displays `⚡` and pane statuses use `●`, `✓`, `✗`. When launched with `LANG=C` or with `unicode_fallback = true` in `settings.toml`, badges fall back to `[*]`, `[OK]`, `[X]`, `[OBS]`.

### C. Harness Panes & Multiplexing (T6 / #140)
- [ ] **Navigate to Panes**: Press `5` or type `:panes` in command palette (`/` or `Ctrl+P`).
- [ ] **Launch Owned Pane**: Press `c` or `Ctrl+B c` to open launcher modal. Select a harness (e.g. Gemini or Claude) with Owned mode (`[m] Toggle Mode` showing `Mode: Owned [Interactive PTY]`), and press Enter.
- [ ] **Interactive Input**: Verify prompt loads in the VT viewport. Type commands (e.g. `ls`, `echo test`) and verify keystrokes are received and rendered.
- [ ] **Prefix Chords**:
  - `Ctrl+B d`: Detaches focus from pane back to TUI navigation.
  - `Ctrl+B s`: Toggles horizontal split tiles in wide terminals (≥100 cols).
  - `Ctrl+B n` / `Ctrl+B p`: Cycles through active pane tabs.
  - `Ctrl+B x`: Closes the active harness pane and terminates child process.
- [ ] **Attach Observed Pane**: Press `c`, toggle mode to `Observed [Read-Only Captured]`, and press Enter.
- [ ] **Read-Only Enforcement**: Focus the observed pane and press keys. Verify status bar warns `Observed sessions are read-only` and no keystrokes are sent.

### D. Multi-Instance Coherence (T7 / #141)
- [ ] **Concurrent Instances**: Open a second Konsole tab or window and run `cargo run -p tui-bin --`.
- [ ] **Stale Write Detection**: Modify a setting in the second instance or desktop app (`settings.toml`). In the first instance, attempt a setting change or save.
- [ ] **Conflict Banner**: Verify non-blocking red/amber status banner appears:
  `Conflict: settings.toml was modified by another instance. Press [r] to Refresh & retry, [Esc] to dismiss.`
- [ ] **Refresh & Retry**: Press `r`. Verify local state refreshes to disk state, banner disappears, and subsequent writes succeed.
- [ ] **Dismiss**: If banner is dismissed with `Esc`, verify banner hides and normal navigation continues.

### E. Safety Boundary & Desktop Parity (T8 / #142)
- [ ] **No Foreign Process Attach**: Observed sessions attach solely to captured streams / transcripts; no PTY master or raw writer is created for external PIDs.
- [ ] **Zero Markdown Bus Writes**: Create work sessions, send messages, request/approve wake gates, and resolve tasks. Inspect `git status` and workspace directory: confirm zero writes to `.agent/cache/AGENT_BUS.md` or any markdown coordination files. All coordination is stored durably in `HubStore` (SQLite) and `settings.toml`.
