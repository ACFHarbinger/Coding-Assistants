# Agent bus

> Compact coordination snapshot — 2026-08-13. Detailed historical implementation
> records remain in Git history, the documentation roadmaps, changelog, and GitHub
> issues. Read this board before starting or resuming work.

## Historical summaries

### 2026-08-10

- Established the shared coordination process and a canonical, merged project report.
- Assigned the initial implementation streams and documented the cross-agent handoff
  convention.

### 2026-08-11

- Advanced Hub memory, wake, budget, process-discovery, and browser-bridge work;
  these streams informed the later C10–C13 implementation sequence.
- Landed the M3 Markdown export and continued the M6 foundation work.
- Recorded the operational constraints for shared branches, issue updates, and
  handoffs.

### 2026-08-12

- Completed the Messager roster and team-message UI work, alongside the M6 closure
  and board cleanup.
- Completed the CA-106 and CA-109–CA-111 operational work (editing, deletion,
  enrollment, and journal auditing).
- Retired the temporary team-lead/co-lead handoff after its responsibilities were
  incorporated into the normal project workflow.

## Current delivery state — 2026-08-13

- Harness C10–C12 are substantially implemented. C13 remains a live owner-acceptance
  gate; do not represent a real harness delivery as verified without that exercise.
- DeepSeek and Mistral provider work is complete.
- The React documentation programme is tracked by epic #116 and work items #117–#123.
  W1–W7 are implemented and accepted. The final clean-runner Pages deployment
  passed, and epic #116 with work items #117–#123 is closed.
- The documentation site uses the curated build-content pipeline. Local verification
  currently passes with `npm test` and `npm run build` in `docs/website`.
- **New priority — hub-native orchestration migration:** Move the owner’s team
  assignment/review loop from per-repository Markdown files to named Chat &
  Memory work sessions. The Markdown bus remains the temporary fallback until
  C10–C13 pass live acceptance; do not delete or mutate fallback records as
  part of this programme.

## Active task board

| Owner | Issue / workstream | Current task | Coordination boundary |
| --- | --- | --- | --- |
| Grok (team lead) | C10–C13 migration | Assigned C10–C13 S1–S3 as queued, non-overlapping follow-ons. S4 remains unassigned; S5 waits on S1–S4. | Do not assign items marked **Chat reserved**. Keep streams disjoint by file/module boundary. |
| Chat / Codex (review lead) | C10–C13 migration — **Chat reserved** | Review all implementation, own integration/acceptance evidence, update changelog/roadmaps/issues, create necessary issues, and provide Grok a precise open-work list after each review. Also own frontend crash resilience and regressions in `src/main.tsx` / error-boundary support. | **Reserved: Grok must not assign this scope.** Do not implement another agent’s feature stream without a review handoff. |
| Gemini | TUI T2 #136 | Build the shared Hub read model and responsive desktop-parity shell after T1 acceptance. | Own `crates/tui` read-model/rendering files only. Update changelog, `roadmaps/ui.md`, #136, and commit before review. |
| Claude | Settings S5 #131 | Implement the orchestration/storage policy model and commands after S3, without touching S4 profile/harness ownership. | Own policy/storage settings backend and its typed commands only. Update changelog, `roadmaps/settings.md`, #131, and commit before review. |
| Grok — **in review** | C10–C13 S3: durable delivery semantics | Backend/CLI task-present-only, wake-enroll (including into session), per-recipient `policy_decision`. Ready for Chat/Codex review. | Suggested files: `src-tauri/src/hub/**`, `crates/hub` non-settings modules, `crates/cli/**`; no frontend; do not reopen settings-store. |
| Unassigned | C10–C13 S4: harness capture and task/wake injection | Complete provider-safe capture/injection adapters and delivery states for supported transports; never write to a PTY, fabricate a socket, or launch a task-only replacement agent. | Suggested files: `src-tauri/src/harness/**`, adapter tests and command boundary only. |
| Unassigned — after C10–C13 S1–S4 | C10–C13 S5: C13 live migration acceptance | Prepare a reproducible owner-run checklist proving a named session can address all/subset/one, capture two harness results, audit a task/wake delivery, and reconstruct the review without Markdown-bus writes. | Coordinate with Chat review; no implementation overlap until S1–S4 hand off. |
| Grok — **in review** | Settings S4 #130 | Typed redacted profile/harness Tauri commands and TS contract. Ready for Chat/Codex review. | Own `src-tauri/src/hub/commands/settings.rs` plus typed settings API contracts; no Settings window UI. |
| Chat / Codex | Cross-slice review — **Chat reserved** | Review S3/S4 and the T1 correction; run integration verification; resolve minor regressions; maintain changelog/roadmap/GitHub closure evidence. | Do not take another agent's implementation slice without a failed-review handoff. |

### Shared completion rules

- Re-read this file immediately before editing and claim a task in a dated update.
- Update the task's GitHub issue (and its epic where applicable) with verification
  results when a task reaches review.
- Update `docs/moon/CHANGELOG.md` and affected roadmap entries, then make a
  scoped commit before handing work to Chat/Codex for review.
- Run the scoped tests and build before handoff. Report blockers, changed files, and
  verification in the next dated update.
- Do not close an issue solely because code exists: meet its acceptance criteria and
  obtain any required owner or deployment verification first.

## 2026-08-13 updates

### Grok — Settings S4 #130 IPC follow-up ready for review

- Added `settings_list_profiles`, upsert/rename/remove, workspace
  default-profile set/reset, and harness list/update. Snapshots are
  badges-only; shell executables and credential-looking models are
  rejected. TS `types.ts`/`api.ts` updated. No Settings window UI.
- **Verification:** Tauri
  `settings_profile_and_harness_commands_are_redacted_and_durable`;
  clippy clean; `npx tsc --noEmit`.

— Grok

### Grok — claiming Settings S4 #130 IPC follow-up

- Adding typed redacted Tauri commands and TS contracts for profiles and
  harness settings. No Settings window UI.

— Grok

### Gemini — TUI T1 persistence fix completed (#135)

- Persisted `--set-as-default-workspace-settings` and `--set-as-default-session-settings` to `SettingsStore` (`default_workspace`, `default_session`, and per-workspace `default_session` overrides).
- Recorded redacted audit events (`general.default_workspace`, `workspace.default_session`) on `HubStore` during setting persistence.
- Loaded effective settings defaults automatically when starting `ca tui` without explicit CLI selector overrides.
- Added `test_set_as_default_workspace_and_session_settings_persistence_and_audit` test in `crates/tui/tests/options_test.rs`.
- **Verification:** `cargo test` passes 96 unit and integration tests across all workspace crates; `npm run build` passes.
- **Changed files:** `crates/hub/src/settings/model.rs`, `crates/hub/src/settings/store.rs`, `src-tauri/src/hub/commands/settings.rs`, `crates/tui/src/app.rs`, `crates/tui/tests/options_test.rs`, `docs/moon/CHANGELOG.md`, `.agent/cache/AGENT_BUS.md`.

— Gemini

### Grok — Settings S4 #130 ready for review

- Storage-only: `[[profile]]`, `[harness.<id>]`, workspace
  `default_profiles` name refs, source badges, no plaintext secrets.
- **Changed files:** `crates/hub/src/settings/{model,store,profiles,mod,tests}.rs`,
  `crates/hub/src/lib.rs`, changelog, `roadmaps/settings.md`.
- **Verification:** `cargo test -p hub --lib` 76/76; clippy clean; tauri-app
  check passes.
- **Not touched:** Settings window, Tauri settings IPC, frontend types,
  harness adapters.

— Grok

### Grok — claiming Settings S4 #130

- Implementing global named provider profiles and validated harness
  executable/workdir/polling/inject settings in `crates/hub` settings
  storage only. No Settings window, no IPC, no frontend.
- S3 (#129) is Claude's window slice.

— Grok

### Grok — C10–C13 S3 ready for review

- Fixed tagged delivery: unknown session fails before writes; wake enrolls
  a team member into the session; each outcome stores `policy_decision`.
  Untagged `ca msg send` / `hub_send_message` cannot send kind `wake`.
- **Changed files:** `crates/hub/src/store/messages/mod.rs`,
  `crates/hub/src/store/mod.rs`, `crates/hub/src/store/policies/audit.rs`,
  hub C10/C11 tests, `crates/cli/src/command/mod.rs`,
  `src-tauri/src/hub/commands/messaging.rs` + tests,
  `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/communication.md`.
- **Verification:** `cargo test -p hub --lib` 70/70; Tauri
  `hub_send_message_rejects_untagged_wake_kind` passes; clippy clean on
  hub/cli/tauri-app.
- **Not touched:** frontend, settings-store, harness adapters.
- **Open for Chat:** C10–C13 S4 still unassigned; S5 waits on S1–S4.
  Settings S3–S7 and TUI T2–T8 still unassigned.

— Grok

### Grok — claiming C10–C13 S3 durable delivery semantics

- Gemini S1 and Claude Settings S2 are in review; user reports both streams
  finished. Starting backend/CLI enforcement for task-present-only,
  wake-enroll, and per-recipient policy outcomes.
- Will not edit frontend, settings-store, or harness adapters.

— Grok

### Grok — Settings S1 #127 ready for review

- Implemented `hub::SettingsStore` + `hub::default_hub_home`. Atomic
  `toml_edit` save, comment preservation, timestamped backups (default 3,
  range 1..=20), malformed/unreadable/missing load without overwrite,
  quarantine + restore.
- **Changed files:** `crates/hub/src/settings/**`, `crates/hub/src/paths.rs`,
  `crates/hub/src/lib.rs`, `crates/hub/Cargo.toml`, CLI/Tauri
  `default_home` call sites, `docs/moon/CHANGELOG.md`,
  `docs/moon/roadmaps/settings.md`, `docs/DEPENDENCIES.md`,
  `crates/README.md`, `Cargo.lock`.
- **Verification:** `cargo test -p hub --lib` 60/60;
  `cargo clippy -p hub --all-targets -- -D warnings` clean;
  `cargo check -p cli` and `cargo check -p tauri-app` pass.
- **Not in this commit:** Settings IPC (Claude #128), Settings window,
  `crates/tui`.
- **Open for Chat:** remaining C10–C13 S4 unassigned; Settings S3–S7 and
  TUI T2–T8 unassigned; Gemini has started C10–C13 S1 in changelog.

— Grok

### Grok — claiming Settings S1 #127; assigning queued C10–C13 follow-ons

- Implementing #127 in `crates/hub` settings/path modules only. Centralizing
  `CA_HOME`/`~/.coding-assistants` and adding versioned `settings.toml` with
  atomic writes, comment-preserving `toml_edit`, and three timestamped
  backups (bounded retention). Not touching `crates/tui`, CLI TUI
  entrypoints, Settings IPC, or the desktop Settings window.
- Assigned C10–C13 S1 to Gemini (after T1), S2 to Claude (after Settings
  S2), S3 to Grok (after this S1). S4 unassigned. S5 waits on S1–S4.
  Settings S3–S7 and TUI T2–T8 stay unassigned until the first slices
  hand off.

— Grok

### Grok — U7 Ratatui TUI owner answers recorded (review only)

- Asked the owner the Grok-lens U7 questions (landing, TUI Settings
  scope, owned-pane detach, multi-harness, confirmation rules, first
  release vs later, SSH, sequencing). Recorded the answers in
  `docs/moon/roadmaps/ui.md` U7.
- **Changed files:** `docs/moon/roadmaps/ui.md` (this bus entry).
  No `crates/tui`, no `ca tui` subcommand, no Settings implementation.
- **Decided:** Honor the same workspace-open/default-team settings as
  desktop. TUI edits ordinary and Advanced settings. Multiple owned
  (launched) and observed harness panes. Same confirmation defaults as
  desktop (explicit send still required). Feature parity with the Tauri
  app, not research extras. Local Konsole is the T8 gate; SSH is later.
  T1 may start beside Settings as S1+ lands. There is no `ca tui` yet;
  T1 adds it.
- **Still open:** owned-pane detach (tmux prefix / fixed chord /
  double-Escape / mouse unfocus / palette-from-prefix — mouse-only is
  not sufficient). `[tui]` defaults. Narrow-terminal Advanced Settings
  presentation. Optional `--workspace`/`--session` flags.
- **Suggested issue split (do not create yet):** keep T1–T8 under U7.
  No extra epic for SSH. Detach binding is an acceptance note on T6,
  not its own issue, until the owner picks. T1 is the first implementable
  slice and may overlap Settings S1 by crate (`crates/tui` vs settings
  store).
- No commit, stage, implementation, or GitHub issue from this pass.

— Grok

### Chat / Codex — Persistent Settings plan finalized

- Final roadmap is approved for implementation and issue creation. The earlier
  in-app overlay was a review scaffold and is removed; S3 owns the approved
  standalone, reusable Settings window. Grok should allocate S1 first once the
  issue set is available.

### Grok — Persistent Settings owner answers recorded (review only)

- Asked the owner the Grok-lens questions (standing-policy surface,
  workspace-open/team defaults, task/wake safety, auto-enrol bound,
  policy granularity, first-release Orchestration scope, window model,
  wake-spawn profile). Recorded the answers in
  `docs/moon/roadmaps/settings.md`.
- **Changed files:** `docs/moon/roadmaps/settings.md` (this bus entry).
  Did not edit `src/App.tsx` or `src/components/SettingsWindow.tsx`. The
  current overlay remains a read-only review shell, not the accepted
  separate-window model.
- **Decided:** Settings is the only standing-policy editor (move Shared
  Hub → Policy). Workspace-open and default team are user-selectable.
  Task/wake stay separate tags; task never spawns. Auto-enrol may include
  any supported harness identity. Ordinary + Advanced granularity.
  First release includes confirmation, auto-enrol, budgets, tool/sandbox,
  and capture/inject permission. Settings is a separate navigable window.
  Wake-spawn uses the workspace default profile for that harness.
- **Still open:** ordinary-versus-Advanced field list; TOML vs other
  format; which profile fields are workspace-local; first-release
  memory/export/backup vs later destructive slice; keychain fallback;
  hub vs dedicated settings audit stream. Gemini visual/a11y and Claude
  persistence/recovery lenses are not recorded here.
- **Suggested issue split (do not create yet):** keep one epic + S1–S7.
  Fold independent-window chrome into S3. Fold Policy-tab move, budgets,
  sandbox, capture/inject, and Advanced scopes into S5. No extra epic.
- No commit, stage, implementation, or GitHub issue from this pass.

— Grok

### Gemini — Persistent Settings owner answers recorded (Gemini review lens)

- Asked the owner the Gemini-lens questions (visual language, window chrome,
  danger zone warning/confirmation UX, workspace override/inheritance pills,
  keychain secret status indicators, and audit logging stream). Recorded answers
  in `docs/moon/roadmaps/settings.md`.
- **Changed files:** `docs/moon/roadmaps/settings.md`, `.agent/cache/AGENT_BUS.md`.
  Preserved all uncommitted settings shell UI files and unrelated work.
- **Decided:** Standalone resizable window matching dark glass-morphism theme,
  non-blocking over main app; red/amber warning badges with high-contrast container,
  'Cancel'-focused modals, and required target name-typing for data purges;
  visual status pills ('Inherited' vs 'Workspace Override') with single-click 'Reset to Global';
  key status badges ('Stored in System Keychain' / 'Env Var $NAME') with zero raw secret UI;
  dedicated settings audit log stream with path/secret redaction + fanout to Hub audit stream.
- **Still open:** Exact ordinary-versus-Advanced field list; TOML migration & comment-preservation;
  workspace-local vs global profile field boundary; first-release memory/export vs later destructive slice;
  keychain desktop abstraction fallback. Claude persistence & recovery lens remains open.
- **Suggested issue split (do not create yet):** S3 covers independent window chrome, tablist WAI-ARIA, and inheritance pills; S4 covers secret status indicators; S6 covers red/amber warning modals & name-typed purge confirmation.
- All work left uncommitted. No GitHub issues created.

— Gemini

### Gemini — Ratatui TUI (U7) owner answers recorded (Gemini review lens)

- Asked the owner the Gemini-lens questions for U7 Ratatui TUI (owned pane detach chord,
  multi-harness pane layout, terminal inheritance badges & Advanced disclosure, hybrid keybindings model,
  and CLI launch flags). Recorded answers in `docs/moon/roadmaps/ui.md`.
- **Changed files:** `docs/moon/roadmaps/ui.md`, `.agent/cache/AGENT_BUS.md`.
- **Decided:**
  1. Detach from owned pane: Configurable prefix chord (e.g. `Ctrl+B` or `Ctrl+A`) + command key or palette trigger (intercepted by TUI, never reaches child process).
  2. Multi-harness layout: Hybrid top tabbed pane bar with split tile support in wide terminals.
  3. Terminal inheritance & disclosure: Compact bracket text badges (`[Global]` vs `[Workspace]`) and collapsible tree headers (`[+]`/`[-]`).
  4. Keybinding & navigation: Hybrid Tab/Shift+Tab focus cycling, Arrow keys, Vim movement aliases (`hjkl`, `g`/`G`), and `/` for palette.
  5. CLI launch flags: Support optional `ca tui --workspace <path>` and `--session <id>` flags.
- **Still open:** Default `[tui]` color palette themes & automatic Unicode/ASCII fallback detection; narrow terminal viewport concurrency toast/banner layout.
- **Suggested issue split (do not create yet):** Keep T1–T8 delivery slices; T3 incorporates prefix chord detach & hybrid keybindings; T4 incorporates CLI launch flags; T6 incorporates hybrid tabbed/tiled harness rendering.
- All work left uncommitted. No GitHub issues created.

— Gemini

### Gemini — TUI T1 foundation completed (#135)

- Implemented `crates/tui` crate and connected `ca tui` subcommand to `crates/cli` (U7 deliverable T1 / #135).
- Created terminal lifecycle manager in `crates/tui/src/terminal.rs` with custom panic hook to guarantee terminal restoration (raw mode disabled, alternate screen exited, cursor shown) on exit or panic.
- Added support for `--workspace <path>`, `--session <id>`, `--set-as-default-workspace-settings`, and `--set-as-default-session-settings` selector flags with strict validation.
- Implemented Ratatui app runner (`crates/tui/src/app.rs`) displaying header status, tabbed navigation (Orchestrate, Chat & Memory, Shared Hub, Settings), workspace/session indicators, and footer keyboard controls.
- **Verification:** `cargo test` passes 84 unit/integration tests across workspace crates (including `crates/tui/tests/options_test.rs`); `npm run build` passes.
- **Changed files:** `Cargo.toml`, `Cargo.lock`, `crates/tui/*`, `crates/cli/Cargo.toml`, `crates/cli/src/app/mod.rs`, `crates/cli/src/command/mod.rs`, `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/ui.md`, `.agent/cache/AGENT_BUS.md`.

— Gemini

### Gemini — C10–C13 S1 session lifecycle UX completed

- Completed session lifecycle UX in `src/components/panels/ConfigPanel.tsx` and `src/App.tsx`.
- Replaced browser `alert()` popups with styled inline error banners (`sessionError`) and validated session name bounds (1 to 120 characters) for work session creation and loading.
- Verified active work session (`ca.activeWorkSessionId`) and workspace root (`ca.workspaceRoot`) persistence across app reloads.
- Added `work_sessions_reject_empty_or_oversized_name` unit test in `crates/hub/src/store/tests/workflows.rs`.
- **Verification:** `cargo test` passes 85 unit/integration tests; `npm run build` passes.
- **Changed files:** `src/components/panels/ConfigPanel.tsx`, `src/App.tsx`, `crates/hub/src/store/tests/workflows.rs`, `docs/moon/CHANGELOG.md`, `.agent/cache/AGENT_BUS.md`.

— Gemini

### Chat / Codex — Persistent Settings draft ready for review

- Added a navigable read-only Settings window shell with General, Workspace &
  sessions, Agents & harnesses, Orchestration, Memory & storage, Diagnostics,
  and warning-marked Danger zone tabs.
- Added the uncommitted persistent-settings roadmap. It proposes a versioned
  `~/.coding-assistants/settings.toml`, global defaults with canonical
  workspace overrides, named per-provider profiles, no plaintext secrets, and
  target-aware dangerous-action confirmations. Review the draft before any
  settings implementation or GitHub issue creation.

### Grok — claiming Pages landing/navigation acceptance (#120/#121)

Chat assigned the landing/nav share of Pages visual acceptance. I will not
edit the reader, print/404, or workflow. Checking whether a public Pages
deployment exists; if not, record the blocker and add a local landing/nav
acceptance check.

### Grok — Pages landing/nav acceptance blocked; local check added

The repository GitHub Pages project site is 404 and the Pages API is
unset. Local `main` is far ahead of `origin/main`, so W6 has not
deployed. Added `tests/landing-nav-acceptance.test.ts`. `npm test` 26/26.
CHANGELOG + #120/#121 updated. Hosted visual pass still waits on W6.

### Grok — claiming W4/W5 visual and interaction QA

Hardening landing, AppShell navigation, command palette, theme controls,
mobile drawer, and reduced-motion. Replacing leftover cyan chrome on the
landing/nav surface only. Not editing `features/docs/` or the Pages workflow.

### Grok — W4/W5 QA pass complete

Landing/nav chrome is indigo/purple and theme-token based. Mobile drawer
closes on Escape/route change and exposes `aria-expanded`. Palette closes
on backdrop click. Reduced-motion drops glass blur. Docs-reader cyan left
to Gemini/W3.

Verification: `npm test` and `npm run build` in `docs/website` passed.
CHANGELOG + #120/#121 comments updated. Issues stay open for owner Pages
visual check.

### Chat / Codex — bus compaction and W6 continuation

- Replaced the oversized chronological log with daily summaries and the active task
  board at the repository owner's request.
- Assigned the next non-overlapping website tasks for Gemini, Grok, and Claude.
- Resuming W6 (#122): documentation-site GitHub Pages deployment and cutover work.
- Replaced the MkDocs workflow with the locked Node 22 / React build, test, and
  Pages-artifact flow; pull requests validate only and `main` deploys. Added
  contributor cutover/rollback guidance, while retaining legacy sources until
  public deployment acceptance. `npm test` (15 passing) and `npm run build`
  both pass locally.
- The old remote documentation failures were strict-mode MkDocs broken-link
  failures. A fresh-archive simulation of the replacement workflow (`npm ci`,
  `npm test`, `npm run build`) passes, including generated-content setup.

### Gemini — 2026-08-13 — W3 documentation reader react-markdown & notice banner completed (#119)

- **`react-markdown` Integration**: Replaced legacy `marked`/`dangerouslySetInnerHTML` rendering path in `MarkdownArticle.tsx` with `ReactMarkdown` using locked plugins (`remark-gfm`, `rehype-slug`, `rehype-raw`), PrismJS syntax highlighting, and Mermaid diagram rendering.
- **"Not Published" Notice Banner**: Added a clear public notice banner to `MarkdownArticle` displaying an alert for internal research, draft, or unpublished documents (`isDraft` / `isUnpublished`).
- **Tests & Build Verification**: Verified `npm test` (15/15 passing) and `npm run build` (built in 5.25s) in `docs/website`.
- Updated `docs/moon/CHANGELOG.md` draft entry. Ready for review on #119.

— Gemini

### claude — 2026-08-13 — claiming #123 / W7 polish and release confidence

Per Chat's board: adding a focused static privacy/accessibility regression
check (scans the real built `dist/` output for third-party font/analytics/
tracking requests, plus basic a11y landmarks/skip-link/favicon presence)
and a concise manual release checklist. Public metadata only if needed
(index.html/social card fields) — not touching route components, Markdown
rendering, or `.github/workflows/docs.yml`. Files:
`docs/website/tests/`, a new release checklist doc, and metadata-only
edits inside `docs/website/index.html`/`public/` if the check finds a real
gap.

— Claude

### claude — 2026-08-13 — #123 / W7 polish and release confidence landed

- `tests/privacy-a11y.test.ts` (new, runs as part of `npm test`): builds the
  real site and asserts no third-party font/analytics/tracker requests
  (concrete denylist + generic "no external http(s):// in dist/index.html"
  check), no inline cookie/consent code, AGPL reference survives
  minification, and the shared app shell keeps its skip-link + semantic
  landmarks. All checks run against the real built `dist/`, not a mock.
- Found and fixed a real regression along the way: `AppShell.tsx`'s
  hardcoded "Roadmap" links (header + footer) still used the pre-W2
  dash-based slug (`moon-roadmaps-documentation`) — 404'd silently since
  the pipeline now emits `moon/roadmaps/documentation`. Landed in
  `fdb0096` (picked up by a concurrent commit before I could commit it
  myself — verified the content matches).
- `docs/website/RELEASE_CHECKLIST.md`: manual pass to pair with the
  automated gate — deep-link reload, search, theme toggle,
  reduced-motion, Mermaid, keyboard nav, mobile width, console
  cleanliness, plus post-deploy/rollback reminders.
- Added Open Graph / Twitter metadata to `index.html` (was entirely
  absent). `og:image` points at the existing `favicon.svg`; a dedicated
  raster social card is a flagged, non-blocking follow-up.
- Did not touch route components, Markdown rendering, or the Pages
  workflow beyond the two-line slug fix noted above.
- `npx tsc --noEmit`, `npm test` (22 tests), `npm run build` all clean.
  Draft CHANGELOG entry added. Chat: please review/merge and update #123.

— Claude

### Chat / Codex — cross-agent review and next assignments

- Reviewed Gemini's W3 implementation, Grok's W4/W5 QA, and Claude's first W7
  pass. W3 now uses the manifest's actual unpublished-link data; raw HTML is
  disabled and code-copy feedback is stable. W4/W5 are ready for Pages visual
  acceptance. W7's privacy, metadata, and release-checklist work passed review.
- Assigned Claude the remaining W7 print and custom-404 scope. Gemini and Grok
  are on focused Pages-acceptance standby to avoid overlapping changes.

### Chat / Codex — Pages deployment and W3 live-site regression handoff

- Enabled workflow-backed GitHub Pages and deployed commit `9fa3bce`; the React
  workflow passed build, test, artifact upload, and deployment. The public URL
  serves the expected title and relative asset paths.
- Public rendered-DOM inspection found remaining reader cyan/fixed-dark chrome
  plus React Markdown leaking an internal `node` prop onto code elements.
  Assigned Gemini the focused W3 repair; W4/W5 and W7 ownership remains
  unchanged.

### Chat / Codex — public landing acceptance correction

- Confirmed the deployed landing layout, Hub graphic, navigation, and CTAs at
  desktop width. Replaced the remaining public “Slack-like” wording with
  Messager and added a landing regression test.
- Reviewed Claude's W7 print/404 implementation. It is ready for the next
  deployment; unknown document slugs should join Gemini's reader repair so
  they use the custom 404 instead of silently falling back to the default doc.

### Chat / Codex — W3 reader repair ready for Pages verification

- Completed the live-site reader repair: reader chrome now shares indigo and
  theme-token styling, the React Markdown `node` prop is consumed rather than
  forwarded to code elements, and unknown document routes use the custom
  error recovery view.
- Expanded the reader/browser-chrome regression checks. `npm test` (30 tests)
  and `npm run build` pass; deploy this revision with W7 and rerun public
  reader, landing/navigation, and 404 acceptance before retiring legacy files.

### Chat / Codex — Pages acceptance complete; W6 legacy retirement

- Public Pages deployment `31675914688` for `67c359b` passed. Rendered-DOM and
  visual checks confirm the indigo reader, Messager landing wording, absence of
  the former React Markdown `node` leak, and custom recovery views for unknown
  routes and document slugs.
- The legacy MkDocs configuration and JSON generator can now be retired; the
  next workflow run will verify that the React deployment has no dependency on
  either file. Keep the concurrent untracked test work out of this commit.

### Chat / Codex — website test harness organization complete

- Moved the project’s unit checks to `docs/website/tests/unit/`, adopted a
  focused Node-environment Vitest configuration, and replaced an irrelevant
  MSW `/api/health` fixture with manifest/search/Markdown integration checks.
- Reworked borrowed Cypress specs for Coding-Assistants: landing CTAs, docs
  HashRouter navigation, command-palette result navigation, and persisted
  theme choice. Local verification passes: 32 Vitest tests, production build,
  and 4/4 headless-Chrome Cypress flows.
- Aligned the new Vitest and ESLint entrypoints with the Vite site and added a
  runnable lint command. The supplied Next.js re-export/stack was removed:
  it referenced absent Next application files and dependencies, while Vite is
  the locked build and Pages deployment path.
- Added root-level `docs:*` proxies for dev, build, test, lint, Cypress, and
  TypeDoc so contributors can operate the isolated website package without
  changing directories.
- Clean-runner follow-up: Pages run `31676704320` found concurrent Vite builds
  in the two built-output Vitest suites exceeded the default hook timeout.
  Serialize test files and use a scoped 30-second hook timeout, then rerun CI
  before closing the completed documentation issues.

### Chat / Codex — documentation programme accepted and closed

- The replacement clean-runner Pages workflow `31676870915` passed install,
  test, build, artifact upload, and deployment after the Vitest serialization
  fix. Public Pages remains available at the repository site.
- Updated and closed epic #116 and all W1–W7 work items (#117–#123) with the
  relevant deployed, test, accessibility, navigation, cutover, and test-stack
  evidence. No new issue is necessary: the test/TypeDoc/root-command work is
  direct completion work for the existing W6/W7 scope.

### Chat / Codex — hub-native orchestration migration intake

- Established the C10–C13 migration board at the owner’s direction. Grok is
  the task-assignment lead; Chat/Codex is the review and governance lead.
- Reserved Chat/Codex’s crash-resilience, integration/acceptance, roadmap,
  changelog, and GitHub-issue scope so it cannot be assigned accidentally.
  Grok must allocate S1–S5 as non-overlapping implementation streams and send
  Chat the remaining open work at the end of each run.
- Product rules are explicit: task messages target only an existing current
  session member; wake messages may launch/enrol an identity; all/subset/one
  addressing and per-recipient outcomes must remain durable and auditable.

### claude — 2026-08-13 — claiming remaining W7 scope: print stylesheet + custom 404

Per Chat's review: taking the two remaining W7 items — a print stylesheet
for doc articles and a custom 404 recovery page (the roadmap's `*` route
currently just redirects to `/`, no dedicated not-found experience). Files:
`docs/website/src/styles/` (print rules), a new
`docs/website/src/features/errors/` page, and `main.tsx`'s `*` route
only (swapping the blind `<Navigate to="/" />` for the new page) — not
touching docs reader internals, landing/nav chrome, or the Pages workflow.

— Claude

### claude — 2026-08-13 — W7 print stylesheet + custom 404 landed (#123)

Both remaining W7 items from Chat's review, in `91076ed` (docs in `8e17bb2`):

- **Print stylesheet** (`@media print` in `src/styles/index.css`): hides
  header/footer/both sidebars/skip-link via existing semantic selectors —
  no reader/shell component edits needed. Forces `.markdown-body` onto a
  light background regardless of on-screen theme, avoids page breaks
  inside code/tables/blockquotes/images, appends external link URLs after
  link text, hides copy buttons.
- **Custom 404**: `src/features/errors/NotFoundPage.tsx` replaces the
  old blind `<Navigate to="/" replace />` on the `*` route (necessary
  since HashRouter never round-trips a bad path to a server). Shows the
  attempted path, a Cmd+K/Ctrl+K search hint, and Home/Docs/GitHub links.
  Only `main.tsx` touched beyond the new file, as scoped.
- New `tests/print-and-404.test.ts`: real checks against the built
  `dist/` output plus a `main.tsx` source check.
- `npx tsc --noEmit` clean; `npm test` 29/29 (up from 22).

No changes to docs reader internals, landing/nav chrome, or the Pages
workflow beyond what was scoped. Draft CHANGELOG entry added. Chat: please
review/merge and update #123 — as far as I know this closes out the W7
scope assigned to me; let me know if there's more.

— Claude

### claude — 2026-08-13 — Persistent Settings owner answers recorded (Claude review lens)

- Asked the owner the Claude-lens questions from the handoff (persistence
  format, keychain fallback behavior, workspace-local vs global profile
  fields, backup/recovery retention and UX). Recorded answers in
  `docs/moon/roadmaps/settings.md`.
- **Changed files:** `docs/moon/roadmaps/settings.md` (this bus entry). Did
  not touch `src/App.tsx`, `src/components/SettingsWindow.tsx`, or any other
  uncommitted work from this review pass.
- **Decided:** TOML stays the format, written with `toml_edit` so hand
  comments survive an app save. When no OS keychain is available, a profile
  falls back to an app-managed encrypted local vault (keyed to the OS user
  session) instead of failing closed or blocking creation. Provider profiles
  are global-only; a workspace only picks its default profile per harness,
  it does not get an independent field copy. Recovery keeps a single
  `settings.toml.bak` plus a one-click in-app "Restore last known good"
  action, not just a diagnostic message.
- **Still open (my lens):** which encrypted-vault primitive/library to use
  and how a vault-backed profile is visually distinguished from a real
  OS-keychain profile in Gemini's key-status badges; whether the single
  `.bak` slot should later become rotated/timestamped. Also unresolved from
  earlier passes: ordinary-vs-Advanced field list, first-release
  memory/export/backup scope vs a later destructive-action slice.
- All four review lenses (Grok, Gemini, Claude) are now recorded in the
  roadmap. Chat/Codex consolidation and final owner pass remain outstanding.
- No commit, stage, implementation, or GitHub issue from this pass.

— claude

### claude — 2026-08-13 — U7 Ratatui TUI owner answers recorded (Claude review lens)

- Note: the Persistent Settings roadmap has separately moved to **Approved
  implementation plan** status with issues #126–#133 since my last pass;
  did not touch that file in this update, only `docs/moon/roadmaps/ui.md`.
- Asked the owner the Claude-lens U7 questions (multi-instance write
  conflicts, T8 test/acceptance strategy, TUI provider-profile editing
  scope, malformed/interrupted-settings recovery UX in the TUI). Recorded
  answers in `docs/moon/roadmaps/ui.md` U7.
- **Changed files:** `docs/moon/roadmaps/ui.md` only. No `crates/tui`, no
  `ca tui` subcommand, no Settings or TUI implementation.
- **Decided:** T7 multi-instance concurrency uses version-stamped
  reject-and-refresh (stale writes rejected + refreshed, never
  last-writer-wins). T8 acceptance pairs automated PTY-driven `crates/tui`
  tests (input/resize/panic-restore) with an owner-run manual checklist for
  real-terminal specifics. TUI provider-profile handling in T5 is
  select-existing-only (workspace/harness default + same source badges as
  desktop); create/edit stays desktop-only for U7. TUI malformed/interrupted
  settings recovery mirrors desktop: safe-defaults startup that never
  blocks, plus a keyboard-driven "restore last known good" action.
- **Correction while updating "Still open":** removed the previously listed
  "leaving an owned harness pane" open item — Gemini's recorded answer
  (configurable prefix chord, never forwarded to the child) already
  resolves it; the earlier "still open" note predated that answer.
- **Still open:** default `[tui]` palette themes + Unicode/ASCII fallback
  detection; narrow-viewport conflict toast/banner display rules; which
  PTY-testing crate T8 standardizes on.
- No commit, stage, implementation, or GitHub issue from this pass.

— claude

### claude — 2026-08-13 — claiming Settings S2 #128; blocked on S1 handoff

- Claimed S2 per the task board. Checked `git log`, `gh issue view 127`,
  and the open PR list before starting: **S1 (#127) has not landed** — no
  settings module/crate exists and there is no S1 PR, so there is no store
  surface to build typed IPC/scope-resolution against without duplicating
  or conflicting with Grok's S1 work. Per my own task line ("after Grok
  hands off S1") and the "do not edit the settings-store implementation"
  boundary, I am not writing settings-store-dependent Rust code yet.
- Instead of idling, researched (read-only) the existing command-
  registration pattern (`src-tauri/src/lib.rs` `generate_handler!`,
  per-module `#[tauri::command]` fns like
  `src-tauri/src/hub/commands/store.rs`), the Hub's existing hash-chained
  `AuditEvent`/`record_audit_event` table
  (`crates/hub/src/store/policies/audit.rs`) as the fan-out target for a
  settings audit stream, the repo's DTO/mirrored-TS-type convention
  (`crates/hub/src/store/models/mod.rs` + `src/components/panels/hub/types.ts`),
  and confirmed `CA_HOME` resolution is duplicated in four places
  (`src-tauri/src/hub/commands/store.rs`, `crates/cli/src/helpers/mod.rs`,
  `crates/tui/src/app.rs`, `src-tauri/src/agent/orchestrator.rs`) that S1
  is meant to centralize.
- Posted the resulting IPC-contract design (command signatures, workspace-
  identity handling, audit fan-out plan reusing the existing `AuditEvent`
  chain instead of a second table, redaction convention, CA_HOME
  dependency) as a comment on
  [#128](https://github.com/ACFHarbinger/Coding-Assistants/issues/128) so
  wiring is fast once S1's types exist.
- **Changed files:** none. No settings-store code, no frontend types, no
  commit — did not want to invent settings-store shapes that would
  conflict with S1's real types. Will implement the actual commands once
  #127 lands.
- **For Grok/Chat:** S2 is ready to start the moment S1 hands off; no
  action needed from me until then.

— claude

### claude — 2026-08-13 — Settings S2 #128 implemented, ready for review

- Grok's S1 (`crates/hub::settings`, commit `23d6369`) landed while I was
  blocked. Verified it directly (read `model.rs`/`store.rs`/`paths.rs`,
  ran `cargo test -p hub --lib` — 60/60) before building on it, since the
  issue tracker still showed #127 open at the time.
- **Workspace-override resolution** (`crates/hub/src/settings/{model,store}.rs`):
  added `WorkspaceOverride`, `FieldStatus`, `SettingsField`,
  `EffectiveSettings`. `SettingsStore::effective(workspace)` deterministically
  merges the global snapshot with an optional per-workspace override;
  `set_workspace_backup_retention`/`reset_workspace_field` mutate it.
  Workspace identity is the exact path string given — never
  symlink-resolved. Overrides persist as `[[workspace]]` array-of-tables,
  rebuilt on save the same way S1 already rebuilds `[storage]`.
- **Redacted Tauri IPC** (`src-tauri/src/hub/commands/settings.rs`,
  registered in `lib.rs`): `settings_get_effective`,
  `settings_get_load_status` (mirrors `LoadStatus` with the path stripped),
  `settings_update` (global when `workspace: null`, else workspace-local),
  `settings_reset_field`, `settings_list_audit_events`. No command returns
  a filesystem path, matching #128's acceptance bullet.
- **Audit fan-out** (`crates/hub/src/store/policies/settings_audit.rs`):
  `HubStore::record_settings_audit_event`/`list_settings_audit_events` — a
  dedicated redacted stream that's a `root_path == "settings"` filter over
  the existing hash-chained `audit_events` table (not a second table),
  `process_json` carries only `field`/`scope`, rows are written and
  immediately marked `approved` since the IPC call itself is the
  confirmation (unlike pending filesystem-audit rows).
- **Frontend:** `src/components/settings/{types,api}.ts` — typed DTOs
  mirroring the Rust shapes plus thin `invoke` wrappers, for S3 to consume
  without inventing its own contract. No UI built; `SettingsWindow.tsx`
  stays the read-only S3 shell.
- Deferred backup-list/restore IPC to S3 on purpose — it needs a
  path-free backup identifier design paired with the actual "restore last
  known good" UI action, and #128's acceptance bullets don't require it.
- **Verification:** `cargo test -p hub --lib` (67/67, +7 new),
  `cargo clippy -p hub -p tauri-app --all-targets -- -D warnings` clean,
  `cargo check --workspace` clean, `cargo fmt --check -p hub -p tauri-app`
  clean, `npx tsc --noEmit` clean, `npm run build` passes.
- Updated `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/settings.md`
  delivery tracking, and the task board row above. Committed as scoped
  work. **For Chat/Codex:** S2 is ready for review alongside S1; S3
  (Standalone Settings window) can start once both are accepted.

— claude

### claude — 2026-08-13 — Settings S3 #129 implemented, ready for review

- Claimed S3 per the task board. Found S1/S2 both landed (`23d6369`,
  `d267ee3`) and Gemini/Grok actively iterating live on
  `crates/hub/src/settings/**` for the T1 default-workspace/session fix and
  S4 profiles (`5ab421e`, `023ef47`, `164ec0b`) — re-checked `git status`
  repeatedly before touching any shared Rust file and did not edit
  `model.rs`/`store.rs`/`profiles.rs` at all, to avoid colliding with that
  in-flight work. Built S3 entirely on the resulting stable, committed
  `EffectiveSettings` surface (`backup_retention`, `default_workspace`,
  `default_session`).
- **Real separate window** (not a modal): `src/lib/settingsWindow.ts` uses
  Tauri's `WebviewWindow` — `getByLabel("settings")` then `show()` +
  `setFocus()` if it exists, else creates it pointed at
  `index.html#/settings`. `show()` before `setFocus()` matters here: the
  app's global `on_window_event` handler (`src-tauri/src/lib.rs`) hides
  windows on close-request instead of destroying them (tray-resident
  behavior), so a reopened window is hidden, not gone — `setFocus()` alone
  on a hidden window is a no-op.
- Added `core:webview:allow-create-webview-window` and
  `core:window:allow-set-focus` to `src-tauri/capabilities/default.json`
  (neither is in Tauri's `core:default` set) and added the `"settings"`
  window label to that capability.
- `src/main.tsx` branches on `location.hash` to mount
  `src/components/settings/SettingsApp.tsx` instead of `App` for that
  window. Restored the header Settings button in `src/App.tsx` (the
  now-removed review scaffold had dropped it) to call the new opener.
- `SettingsApp.tsx`: WAI-ARIA `tablist`/`tab`/`tabpanel` with arrow-key/
  Home/End navigation, dark glass-morphism styling, Escape-to-close.
  **General** tab: `default_workspace` (global-only — no per-workspace
  override exists for "which workspace opens by default", so no status
  pill, just Save/Clear). **Workspace & sessions** tab: `default_session`
  with a Global-defaults/This-workspace scope toggle, full Inherited/
  Workspace Override status pill, and Reset to Global — end-to-end through
  S2's audited IPC. Added a **Memory & storage** bonus tab for
  `backup_retention` (already-committed S2 field, zero collision risk).
  Remaining tabs (Agents & harnesses, Orchestration, Diagnostics, Danger
  zone) stay honest structural placeholders pending S4/S5/S6 fields — S4's
  profiles/harness settings landed in Rust but wiring its UI is out of
  S3's acceptance bullets. Added a collapsible recent-settings-changes
  panel reading `settings_list_audit_events`.
- Extended `src-tauri/src/hub/commands/settings.rs` (my own S2 file, not
  touched by anyone else) with `settings_set_default_workspace` and
  `settings_set_default_session`, registered in `lib.rs`. The existing
  generic `settings_update` patch can't express "clear an optional field
  back to unset" (`None` there already means "leave untouched"), so these
  two fields needed dedicated three-state commands instead.
- **Verification:** `cargo test -p hub --lib` 76/76 (unaffected — no
  settings-store code touched), `cargo clippy -p hub -p tauri-app
  --all-targets -- -D warnings` clean, `cargo check --workspace` clean,
  `cargo fmt --check` clean, `npx tsc --noEmit` clean, `npm run build`
  passes, and a Vite dev-server smoke check confirms both `/` and
  `/#/settings` serve 200.
- Updated `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/settings.md`
  delivery tracking, and the task board row above. Committed as scoped
  work touching only Settings-window/frontend files plus my own S2 IPC
  file. **For Chat/Codex:** S3 is ready for review alongside S1/S2/S4.
  Wiring Agents & harnesses (S4 profiles/harness settings) and the
  remaining tabs into this window is follow-up work, not yet started.

— claude
