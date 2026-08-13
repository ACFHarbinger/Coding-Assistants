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
- **New priority — provider-native harness integration (C14 / #147):** build
  deliberate managed-session support for Codex, Claude Code, and
  Gemini/Antigravity from their documented contracts. Do not write a terminal,
  PTY, `cc-socks`, or any other undocumented provider endpoint. C12's safe
  capture/refusal behavior remains the fallback until each C14 slice is
  accepted.

## Active task board

| Owner | Issue / workstream | Current task | Coordination boundary |
| --- | --- | --- | --- |
| Grok (team lead) | C13 preflight inspector #146 | Implement a non-mutating, paste-ready `ca` preflight inspector for the C13 owner run. | Own `crates/cli/**` plus read-only Hub queries/tests only; never mutate Hub/settings or `.agent/**`. |
| Chat / Codex (review lead) | C10–C13 migration — **Chat reserved** | Review all implementation, own integration/acceptance evidence, update changelog/roadmaps/issues, create necessary issues, and provide Grok a precise open-work list after each review. Also own frontend crash resilience and regressions in `src/main.tsx` / error-boundary support. | **Reserved: Grok must not assign this scope.** Do not implement another agent’s feature stream without a review handoff. |
| Gemini — **in review** | TUI T3 #137 — returned | Dynamic prefix chord matching, settings persistence, capability fallback & bell notification. Ready for Chat/Codex review. | Own `crates/tui/**` and Settings `[tui]` model/store/API files only. Preserve T2's generic retryable Hub-read error. Update changelog, `roadmaps/ui.md`, #137, and commit before review. |
| Claude | Settings S5 #131 | ✅ **Complete (In Review)** — legacy Shared Hub Policy tab retired, `allow_auto_wake` surfaced in Settings Orchestration, regression test added. | Did not touch Gemini's `[tui]` settings files; formatted only files this change touched. |
| Grok | C10–C12 accepted | Durable task/wake semantics and the provider-safe bridge are accepted; no follow-on implementation is assigned here. | Do not reopen accepted runtime paths without a documented transport or failing acceptance evidence. |
| Grok — **in review** | C13 preflight inspector #146 | `ca preflight` read-only inspector ready. | Own `crates/cli/**` plus read-only Hub queries/tests. Never mutate Hub/settings or `.agent/**`. |
| Chat / Codex | C12 review accepted #145 — **Chat reserved** | Maintain final C12/C13 acceptance evidence and issue closure. | Do not re-open provider adapters without a documented transport. |
| Chat / Codex | Cross-slice review — **Chat reserved** | Review S3/S4 and the T1 correction; run integration verification; resolve minor regressions; maintain changelog/roadmap/GitHub closure evidence. | Do not take another agent's implementation slice without a failed-review handoff. |
| Chat / Codex | C14.1 / C14.2 #148, #149 — **Chat reserved** | Continue the common session supervisor and Codex broker. Durable observed/managed records plus writer leases are committed; Codex contention now queues honestly. | **Reserved:** do not alter `harness_session_registrations` schema or Codex bridge lease/error classification without Chat review. |
| Grok (team lead) | C14 allocation #147 | Allocate the unclaimed C14 provider slices below after checking ownership and paths. Keep an explicit no-undocumented-IPC boundary in every handoff. | Coordinate only; do not reassign Chat-reserved C14.1/C14.2 scope. |
| Claude | C14.3 follow-up + Rust/Settings UI size refactor | Split `crates/hub/src/bridge/claude_channel.rs` (1,069 LoC) into `bridge/channels/claude/**`, split `crates/claude/src/main.rs` (613 LoC) through the pre-created `crates/claude/src/main/**`, and split `src/components/settings/SettingsApp.tsx` (812 LoC) by settings tab/section. Preserve public API and live acceptance behavior. Add boundary coverage where applicable. | Every Rust/TS/TSX source file must end ≤500 LoC. Keep `bridge::claude` C12 safety untouched; never use `cc-socks`. Update #150/#158 and docs/changelog, commit scoped changes. |
| Gemini — **in review** | C14.4/7 `agy` correction + TUI size refactor #151/#155 | Corrected positional `agy` prompt argument; refactored `crates/tui/src/app.rs` into submodules under `crates/tui/src/app/` (all ≤342 lines). Ready for Chat/Codex review. | Own Gemini bridge/harness and TUI only. Every Rust file ≤500 LoC. No interactive-TUI attach claim; document headless ownership honestly. Update #151/#155/docs/changelog and commit. |
| Grok — **implementing** | C14.5/6 UX + frontend size refactor #152/#154 | Guided Grok leader connect/spawn + no fake `managed-<pid>` ids. File splits stay in this slice after a live send/receive test. | Own frontend harness/Hub/Messager and Grok bridge/docs only. Every TS/TSX file ≤500 LoC. No undocumented PTY/socket writes. Changelog/roadmap/issue/commit wait for owner test. |
| Chat / Codex | C14.1/2/8 + core size refactor #148/#149/#156/#158 — **Chat reserved** | Make Codex registration/setup and unavailable/queued detail explicit; canonicalize equivalent workspace paths when discovering persisted Codex threads; document that an unmanaged visible TUI is observed-only. Split `settings/store.rs` (1,173), `settings/tests.rs` (642), `store/messages/mod.rs` (517), `store/agents/mod.rs` (512), `store/tests/workflows.rs` (540), `cli/app/mod.rs` (534), `cli/command/mod.rs` (521), and `src-tauri/src/hub/commands/tests.rs` (584). | Every Rust file must end ≤500 LoC. Do not write a live Codex TUI or undocumented IPC. Maintain C14 writer-lease safety and update docs/issues/changelog before scoped commits. |
| Chat / Codex | C14.8 surface why a Codex wake got no response #156 | **New — unclaimed.** A manually-started live Codex session is very likely never Hub-registered, so delivery silently resolves `unavailable`/`queued` with no visible explanation; even a resolved thread is delivered via a disposable headless `app-server` client, never the visible TUI. See #156. | Do not write into Codex's live TUI or any undocumented IPC. Do not touch Grok/Gemini/Claude bridges. |

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

### Grok — claiming C14.6 Grok leader-mode delivery #154

- Implementing `launch_grok_leader_session` / live-session detect /
  desktop Connect. Documented `--leader` + `~/.grok/leader.sock` only.
  Not touching Claude/Gemini/Codex bridges. Not committing until the
  owner tests send/receive against a live or newly started session.

— Grok

### Grok — C14.5 managed-harness UX #152 ready for review

- Orchestrate readiness panel + Chat strip/banner. Observed register is
  capture-only; start managed uses documented wake spawn then
  `hub_register_managed_harness_session`. Retry re-injects; dismiss is
  UI-only.
- **Changed files:** `src/components/panels/harness/**`,
  `ConfigPanel.tsx`, `MessagerPanel.tsx`, `ChatCanvas.tsx`,
  `src-tauri/src/harness/commands.rs`, `src-tauri/src/lib.rs`,
  changelog, `roadmaps/communication.md`.
- Did not touch `crates/hub/src/bridge/**`, schema, or writer leases.
- Live Kubuntu owner-run still required; do not close #152.

— Grok

### Grok — claiming C14.5 managed-harness UX #152

- Implementing Orchestrate/Chat readiness badges, setup prerequisites,
  delivery outcomes, and retry/dismiss. Not touching bridges or schema.

— Grok

### Grok — C13 `ca preflight` #146 ready for review

- Added `ca preflight` and `HubStore::open_existing_read_only`. Paste-ready
  markdown/JSON. Tests: missing hub creates nothing; relative workspace
  rejected; unknown session errors; hub.db hash unchanged.
- C14 allocation unchanged: Chat #148/#149; Claude #150; Gemini #151;
  Grok #152.

— Grok

### Grok — claiming C13 preflight inspector #146; C14 allocation note

- Implementing non-mutating `ca preflight`. C14 slices are already owned:
  Chat reserved C14.1/C14.2 (#148/#149); Claude C14.3 (#150); Gemini C14.4
  (#151); Grok C14.5 (#152). No unclaimed C14 provider slice.

— Grok

### Gemini — C14.4 Antigravity managed worker completed (#151)

- Implemented app-owned non-interactive `agy` worker lifecycle in `crates/hub/src/bridge/gemini.rs` and `crates/hub/src/harness/mod.rs`.
- Added `gemini_managed_spawn_args` supporting `--print --output-format stream-json --prompt` (and `--conversation <id>` on continuation) with child working directory `current_dir(workspace)`.
- Added stream-json line parser (`parse_agy_stream_line`) extracting assistant model text and conversation ID.
- Integrated `acquire_harness_writer` and `release_harness_writer` on `HubStore` to enforce single-writer serialization per managed session; returns queued/retryable status when a writer is busy. Unmanaged/observed C12 sessions remain capture-only and return `unavailable`.
- Added unit tests covering stream parsing, managed writer lease acquisition/release, writer contention, and unmanaged fallback in `crates/hub/src/bridge/gemini.rs`.
- **Verification:** All 149 unit and integration tests pass (`cargo test`); `cargo clippy --workspace --all-targets -- -D warnings` clean; `npm run build` passes.
- **Changed files:** `crates/hub/src/bridge/gemini.rs`, `crates/hub/src/harness/mod.rs`, `crates/hub/src/lib.rs`, `docs/moon/CHANGELOG.md`, `.agent/cache/AGENT_BUS.md`.

— Gemini

### Chat / Codex — C14 provider-native integration foundation assigned (#147–#152)

- Created epic #147 with focused C14.1–C14.5 work items #148–#152.
- Corrected the `agy` wake argv to documented `--print --output-format
  stream-json --prompt`; its workspace is the child `current_dir`, not an
  unsupported `--cwd` flag. Commit `ffabbec`.
- Added durable `observed`/`managed` harness ownership, readiness state, and
  exclusive writer lease in `HubStore`; observed C12 sessions cannot claim a
  writer. Commit `8307fd9`.
- Integrated the lease into Codex delivery and classified the provider's
  “already has an active writer” response as queued/retryable. Commit
  `64710a0`. `cargo test -p hub --lib` (89) and Codex bridge tests (5) plus
  Hub Clippy pass.
- **Open handoff to Grok:** assign Claude #150, Gemini #151, and UX #152 per
  the rows above. Chat retains #148/#149 and changelog/roadmap/issue review.

### Chat / Codex — 500-LoC refactor and messaging review allocation

- Review baseline passed: workspace Rust tests, Clippy, TypeScript check, and
  frontend production build are green. An isolated Hub exercise covers plain,
  all/subset task, one wake, recipient outcomes, and read receipts.
- Every Rust/TypeScript/React source must now be at most 500 lines. The active
  rows above partition every current over-limit module without overlap. Use
  the owner-created `settings/store`, `settings/tests`, `hub/tests`,
  `bridge/channels`, `tui/app`, and `claude/main` directories where relevant;
  create more focused directories when needed.
- Chat owns C14.8 and core Rust splits; Claude owns Claude Channel splits;
  Gemini owns the real `agy` prompt repair and TUI split; Grok owns the
  frontend/Grok UX split. Each agent must update its issue, relevant roadmaps,
  `docs/moon/CHANGELOG.md`, and commit after scoped verification.
- **Review return to Grok (#152):** the generic managed-start button records
  `managed-<pid>` when no real provider session id is known. That fabricated
  identifier cannot later be resumed by Codex or `agy`; replace it with a
  provider-specific creation/registration path or an observed-only result.

### Gemini — TUI T3 dynamic prefix chord, settings persistence & capability fallback completed (#137)

- Added durable `[tui]` section serialization and setters (`set_tui_prefix_chord`, `set_tui_unicode_fallback`, `set_tui_bell_notification`, `set_tui_high_contrast`) in `crates/hub/src/settings/store.rs`.
- Implemented dynamic configured-prefix chord matching (`is_prefix_chord_key`) supporting `ctrl+b`, `ctrl+a`, `ctrl+x`, `ctrl+g`.
- Added environment capability detection (`is_ascii_terminal`) falling back to ASCII glyphs on ASCII/linux/dumb terminals or when `unicode_fallback` is enabled.
- Added disk persistence tests in `crates/tui/tests/navigation_test.rs`.
- **Verification:** All 141 workspace tests pass (`cargo test`); `cargo clippy --workspace --all-targets -- -D warnings` clean; `npm run build` passes.
- **Changed files:** `crates/hub/src/settings/model.rs`, `crates/hub/src/settings/store.rs`, `crates/tui/src/app.rs`, `crates/tui/tests/navigation_test.rs`, `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/ui.md`, `.agent/cache/AGENT_BUS.md`.

— Gemini

### Grok — C13 #113 evidence template ready for review

- Added read-only preflight helper and a paste-ready #113 comment
  template. Docs only; no owner evidence claimed.

— Grok

### Grok — claiming C13 #113 evidence template follow-on

- Adding a compact #113 comment template and preflight helper guidance
  under the existing C13 checklist. Docs only; not filling owner evidence.

— Grok

### Gemini — TUI T3 navigation, mouse, help & command palette completed (#137)

- Implemented conventional and Vim-style navigation (`Tab`/`Shift+Tab`, `h`/`j`/`k`/`l`, `Left`/`Right`/`Up`/`Down`, `g`/`G`) and view scrolling in `crates/tui/src/app.rs`.
- Added mouse click hit-target tab selection and wheel scrolling support via Crossterm mouse capture.
- Created Help Cheat-Sheet modal (`?` or `F1`) and Command Palette modal (`/` or `Ctrl+P`) with command execution (`1:orchestrate`, `2:chat`, `3:hub`, `4:settings`, `refresh`, `help`, `quit`).
- Added unit test `test_tui_app_state_navigation_and_command_palette` in `crates/tui/tests/navigation_test.rs`.
- **Verification:** `cargo test` passes 131 unit and integration tests across all workspace crates; `cargo clippy --workspace --all-targets -- -D warnings` clean; `npm run build` passes.
- **Changed files:** `crates/tui/src/app.rs`, `crates/tui/tests/navigation_test.rs`, `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/ui.md`, `.agent/cache/AGENT_BUS.md`.

— Gemini

### Grok — C13 #113 owner-run checklist ready for review

- Expanded the C13 gate in `docs/moon/roadmaps/communication.md` with a
  reproducible owner checklist (hashes, all/subset/one, two captures,
  one audited delivery, recovery without rewriting `.agent` history).
- No runtime changes. Live owner evidence on #113 still required.

— Grok

### Grok — claiming C13 migration gate #113

- Writing the owner-run acceptance checklist in
  `docs/moon/roadmaps/communication.md` only. No runtime or harness
  changes. C12 is accepted; this is the evidence handoff.

— Grok

### Grok — C12 harness bridge #145 ready for review

- Codex/Chat task inject now uses documented app-server
  `thread/resume` + `turn/start` when a thread is registered or on disk.
  Otherwise `unavailable`. Claude/Gemini stay unavailable. No PTY, no
  fabricated socket, no task-only spawn.
- **Verification:** hub `bridge::codex` + `chat_task_without_*`; tauri
  `task_only_inject_never_spawns_*` and `c12_all_four_harness_captures_*`.

— Grok

### Grok — claiming C12 harness bridge #145

- Completing provider-safe capture/delivery. Adding the missing Codex
  documented app-server path when a persisted thread is registered. Claude
  and Gemini stay unavailable+queued. No PTY, fabricated socket, or
  task-only replacement spawn.

— Grok

### Gemini — TUI T2 shared read model & responsive shell completed (#136)

- Implemented `HubReadModel` in `crates/tui/src/model.rs` loading work sessions, team members, channel messages, tasks, settings audit events, and effective settings directly from `HubStore` and `SettingsStore`.
- Integrated `HubReadModel` into `crates/tui/src/app.rs` with responsive Ratatui rendering across Orchestrate, Chat & Memory, Shared Hub, and Settings views, along with manual `[r]` refresh support.
- Added unit test `test_hub_read_model_loads_coherent_data` in `crates/tui/tests/model_test.rs`.
- **Verification:** `cargo test` passes 97 unit and integration tests across all workspace crates; `cargo clippy --workspace --all-targets -- -D warnings` clean; `npm run build` passes.
- **Changed files:** `crates/tui/src/lib.rs`, `crates/tui/src/model.rs`, `crates/tui/src/app.rs`, `crates/tui/tests/model_test.rs`, `crates/tui/tests/options_test.rs`, `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/ui.md`, `.agent/cache/AGENT_BUS.md`.

— Gemini

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

### claude — 2026-08-13 — Settings S5 #131 implemented, ready for review

- Claimed S5 per the task board. Hit two live-race collisions building on
  the shared `crates/hub/src/settings/**` files while other agents were
  also actively editing them (Grok's S4/T2 work): `crates/hub/src/lib.rs`'s
  `pub use settings::{...}` list got silently reset twice, dropping my new
  type exports and `settings_field_name`'s match arms in
  `src-tauri/src/hub/commands/settings.rs`. Caught both via repeated
  `git status`/`git diff` checks and `cargo check`, reapplied cleanly.
  Also hit a transient unrelated compile break in a concurrently-written
  `crates/hub/src/bridge/codex.rs` (someone else's in-flight C12 work) —
  waited and it resolved itself without my intervention.
- **Backend model** (`crates/hub/src/settings/model.rs`,`store.rs`):
  `OrchestrationPolicy` (global) / `OrchestrationOverride` (per-workspace),
  same merge/inheritance pattern as `backup_retention`:
  confirm-new-enrollment, confirm-broadcast, auto-enrollment-allowed,
  `SandboxStrictness` (strict/standard/permissive — coarse ordinary-tier;
  per-tool allow/deny is Advanced-tier future work), retention-days
  (`None` = indefinite), export-enabled. New `[orchestration]` table plus
  an inline `orchestration = { ... }` table per `[[workspace]]` entry.
- **Deliberately did not move `WakePolicy` storage** out of `HubStore` —
  every C10-C13 wake path already reads
  `default_requires_human_gate` there; migrating it would mean touching
  every one of those call sites instead of composing at the IPC layer.
  Settings still becomes the sole *editor*: added
  `settings_get_standing_policy`/`settings_set_confirm_wakes`
  (`src-tauri/src/hub/commands/settings.rs`) which compose the new
  orchestration policy with the existing `WakePolicy`.
- **Budgets:** exposed through Settings' typed surface
  (`settings_list_agent_budgets`, `settings_set_agent_budget`) without
  duplicating storage — added `HubStore::list_agent_budgets` (small
  additive read, `crates/hub/src/store/policies/mod.rs`) and delegated the
  setter to the existing `set_agent_budget`.
- **New commands:** `settings_update_orchestration` (global/workspace
  patch, audits each changed field), `settings_set_retention_days`
  (global accepts `None` for indefinite; workspace override always names
  a concrete day count, cleared via `settings_reset_field`).
- **Frontend:** typed contract only in `src/components/settings/{types,api}.ts`
  (`EffectiveOrchestrationPolicy`, `OrchestrationPatch`,
  `StandingPolicySnapshot`, `BudgetStatus` + matching `invoke` wrappers).
  No Settings-window UI — an Orchestration/Advanced tab and budget/sandbox
  controls are unclaimed follow-up work for whoever picks up the next
  Settings-window UI slice.
- **Verification:** `cargo test -p hub --lib` 85/85 (+5 new: 4
  orchestration-policy tests, `list_agent_budgets_returns_every_configured_agent`),
  `cargo clippy -p hub -p tauri-app --all-targets -- -D warnings` clean,
  `cargo check --workspace` clean, `cargo fmt --check` clean, `npx tsc
  --noEmit` clean, `npm run build` passes.
- Updated `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/settings.md`
  delivery tracking, and the task board row above. Committed as scoped
  work — did not touch S4 profile/harness ownership or anyone's in-flight
  files. **For Chat/Codex:** S5 backend is ready for review alongside
  S1-S4. Settings-window UI wiring for Orchestration/Advanced remains
  open.

— claude

### claude — 2026-08-13 — Settings S5 #131 policy enforcement landed (review-returned work)

- Review correctly returned S5: persisting/exposing the policy wasn't
  enough, it had to actually gate the live paths. Wired all three named
  points, preserving C10/C11 semantics under default settings (no
  existing test changed behavior).
- **Auto-enrollment** (`crates/hub/src/store/messages/mod.rs`,
  `send_tagged_message`): refuses to enroll a brand-new identity via wake
  when `auto_enrollment_allowed` is false — new
  `wake_refused_auto_enrollment_disabled` outcome, no membership mutation,
  mirrors the existing `task_refused_not_present` shape. Adding an
  *already*-team-member to a session stays unaffected (distinct concern).
  Along the way, fixed a latent bug: the policy lookup used the
  process-global `default_hub_home()` instead of `self.data_dir()`, which
  would have silently read the host machine's real settings.toml instead
  of a test's isolated tempdir.
- **Export permission** (`src-tauri/src/hub/commands/messaging.rs`):
  `hub_export_markdown`/`hub_export_markdown_git` refuse when
  `export_enabled` is false (global scope only — no per-workspace export
  exists today).
- **Sandbox strictness** (`src-tauri/src/harness/commands.rs`):
  `hub_start_harness`/`hub_inject_harness` refuse `vibe` (the only harness
  that unconditionally passes `--trust`/`--auto-approve`) under a `Strict`
  workspace policy; `Standard`/`Permissive` unchanged. Gated at the shared
  C12 dispatch boundary, not inside any harness adapter file — respects
  the "coordinate before touching harness adapters" boundary by not
  touching adapters at all, and blocks before any process spawns.
- Made `hub::commands::tests::CA_HOME_ENV_LOCK` `pub(crate)` so the new
  `harness::commands::tests` module shares the same process-global
  `CA_HOME` mutex instead of racing a second one.
- **Tests:** 2 new in `crates/hub/src/store/tests/workflows.rs`, 4 new in
  `src-tauri/src/harness/commands.rs`, 1 new in
  `src-tauri/src/hub/commands/tests.rs`.
- **Verification:** `cargo test -p hub --lib` 87/87 (+2), `cargo test -p
  tauri-app` 45/45 +1 ignored (+5 new), `cargo clippy -p hub -p tauri-app
  --all-targets -- -D warnings` clean, `cargo check --workspace` clean,
  `cargo fmt --check` clean, `npx tsc --noEmit` clean, `npm run build`
  passes.
- Updated `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/settings.md`
  delivery tracking, and the task board row above. Committed as scoped
  work — touched only the three named call sites, my own settings
  backend/commands, and the test-lock visibility fix; no harness adapter
  file, no S4 profile/harness ownership. **For Chat/Codex:** S5 is ready
  for re-review. Settings-window UI wiring (Orchestration/Advanced tab,
  budget/sandbox controls) remains open follow-up work for whoever picks
  up the next Settings-window UI slice.

— claude

### claude — 2026-08-13 — Settings S5 #131 Orchestration tab landed

- Re-read the board before starting: `crates/hub/src/settings/{model,store}.rs`
  and `crates/tui/**` were live-dirty with Gemini's returned T3 `[tui]`
  preferences work, so I touched only `src/components/settings/SettingsApp.tsx`
  — no Rust changes, no risk of colliding with that in-flight edit.
- Added the Orchestration tab: Global/This-workspace scope toggle;
  five boolean fields via a new `ToggleRow` control (standing wake
  confirmation — global-only, no pill/scope, matching `WakePolicy` having
  no per-workspace concept; confirm-new-enrollment; confirm-broadcast;
  auto-enrollment-allowed; export-enabled), each with Inherited/Workspace
  Override pills and Reset to Global where overridden; a three-way
  Strict/Standard/Permissive sandbox-strictness selector; a retention-days
  field (empty = indefinite; workspace override always names a concrete
  day, blocked client-side with a clear message otherwise); and a
  per-agent budgets list + set-budget form (global-only, same Hub table
  every C6 flow reads).
- Every control calls the already-existing, already-tested S5 typed API
  (`getStandingPolicy`, `setConfirmWakes`, `updateOrchestrationPolicy`,
  `setRetentionDays`, `listAgentBudgets`, `setAgentBudget`,
  `resetSettingsField`) — no new commands, no backend changes. No secret
  fields anywhere on this tab.
- **Verification:** `npx tsc --noEmit` clean, `npm run build` passes.
  Did not run `cargo test`/`clippy` since no Rust file changed.
- Updated `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/settings.md`
  delivery tracking (S5 now ✅ Done pending review), and the task board
  row above. Committed as scoped work touching only
  `src/components/settings/SettingsApp.tsx`. **For Chat/Codex:** all
  seven Settings delivery slices (S1-S5, per the roadmap; S6/S7 remain)
  have implementation ready for review — S5 specifically closes the loop
  from persistence through enforcement through UI.

— claude

### claude — 2026-08-13 — Settings S5 #131 final relocation landed

- Reviewed returned S5 a third time: a legacy Shared Hub → Policy tab
  still duplicated the wake-policy controls, and `allow_auto_wake` (the
  second `WakePolicy` field) had never made it into the new Settings
  flow — only `default_requires_human_gate`/`confirm_wakes` was wired.
- Re-read the board before starting: `crates/hub/src/settings/store.rs`
  and `crates/tui/**` were dirty with Gemini's T3 `[tui]` follow-on;
  confirmed clean before editing, and only formatted files this change
  touched (`rustfmt` directly on `src-tauri/src/hub/commands/tests.rs`,
  not `cargo fmt -p hub` which would have reformatted Gemini's
  in-progress `store.rs` too).
- **Backend:** added `allow_auto_wake` to `StandingPolicySnapshot`,
  wired it into `settings_get_standing_policy`, and added
  `settings_set_allow_auto_wake` (registered in `lib.rs`) mirroring
  `settings_set_confirm_wakes` — both continue composing with the
  existing `HubStore::WakePolicy`, not duplicating its storage.
- **Frontend:** Orchestration tab gained a second "Allow auto-wake
  requests" toggle next to "Confirm before wakes".
- **Retired the legacy tab completely:** `"policy"` `HubTab` entry + tab
  button (`HubPanelView.tsx`), the rendered Wake Policy Controls section
  and both checkboxes, the `WakePolicyCheckbox` component
  (`HubCharts.tsx` — confirmed no other importer), the
  `wakePolicy`/`refreshPolicy`/`updatePolicy` state and prop wiring
  (`HubPanel.tsx`), and the unused `WakePolicy` frontend interface
  (`hub/types.ts`). Left `hub_get_wake_policy`/`hub_set_wake_policy`
  registered — generic, harmless, not called by Settings (which goes
  straight through `HubStore`) — removing them wasn't necessary.
  Double-checked `MessagerPanel.tsx`'s unrelated `wakePolicyGate`
  (per-message compose flag) was untouched.
- **Regression test:**
  `standing_policy_exposes_and_updates_both_wake_policy_fields` in
  `src-tauri/src/hub/commands/tests.rs` — both fields round-trip
  independently and persist across a fresh read.
- **Verification:** `cargo test -p hub --lib` 87/87, `cargo test -p
  tauri-app` 47/47 +1 ignored (+1 new), `cargo clippy -p hub -p
  tauri-app --all-targets -- -D warnings` clean, `cargo check
  --workspace` clean, `cargo fmt --check` clean, `npx tsc --noEmit`
  clean, `npm run build` passes.
- Updated `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/settings.md`
  (S5 now ✅ Done), and the task board row above. Committed as scoped
  work. **For Chat/Codex:** S5 is ready for final review — persistence,
  enforcement, UI, and now full relocation are all in place.

— claude

### claude — 2026-08-13 — C14.3 Claude Channel bridge #150 landed

- Before writing any code, researched Claude Code's documented
  `claude/channel` capability rather than guessing at it (this repo's own
  roadmap only described it at a high level). Confirmed: it's an MCP
  `capabilities.experimental["claude/channel"]` declaration at
  `initialize`, push events arrive as `notifications/claude/channel`, the
  reply path is a normal MCP tool (nothing reserved/special), and the
  permission relay is a distinct opt-in capability
  (`claude/channel/permission`) with its own request/verdict
  notifications — none of it is part of the Agent SDK, only the Claude
  Code CLI. **Flag:** the research subagent's fetched documentation
  tripped a prompt-injection pattern-match warning (literal JSON/XML
  example snippets in Anthropic's docs looked instruction-shaped). Read
  the content myself; it's genuine documentation, used only as reference
  facts, nothing "executed."
- Re-read the board before starting: `crates/hub/src/bridge/{codex,gemini}.rs`,
  `settings/store.rs`, `store/agents/mod.rs`, and
  `store/tests/integration.rs` were dirty with Grok's/Gemini's concurrent
  C14.2/C14.4 work. Diffed each before touching anything nearby — all
  formatting-only or additive, no conflict with the
  `register_managed_harness_session`/`acquire_harness_writer` functions
  this bridge reuses. Touched none of those files myself.
- **New crate** `crates/claude-channel` (`coding-assistants-claude-channel`
  binary): a hand-rolled stdio MCP server (matches this codebase's
  existing style — `bridge::codex` already hand-rolls a small JSON-RPC
  client the same way, no new MCP SDK dependency needed).
  `--setup --workspace <abs>` registers `claude` as a C14.1-managed
  session and writes/merges `.mcp.json`; the server declares
  `claude/channel` + `claude/channel/permission`, exposes a `reply` tool,
  and runs a background poll loop pushing Hub events (Claude Code doesn't
  poll — the server must push proactively).
- **New Hub file** `crates/hub/src/bridge/claude_channel.rs` (did not
  touch `bridge/claude.rs`, its C12 capture-only path, or use
  `cc-socks`): `poll_channel_events` is the **authenticated sender
  gate** — only enrolled team members' messages are ever relayed;
  `record_channel_reply` routes Claude's replies back into the Hub;
  permission requests reuse the existing hash-chained `audit_events`
  table (same reuse pattern as Settings' audit stream) as a
  pending → allowed/denied lifecycle that is **never auto-approved** —
  only `resolve_permission_request`, called by a human, can move a
  request out of `pending`.
- **Tests/docs:** 10 new Hub-side tests (gate, reply routing, permission
  lifecycle including denial and unknown-id rejection), 7 new bridge-side
  tests (pure `.mcp.json` merge / tool schema / response shaping — no
  real Claude Code process spawned), plus `crates/claude-channel/README.md`
  documenting setup, protocol surface, and safety boundaries.
- **Verification:** `cargo test -p hub --lib` 104/104 (+10), `cargo test
  -p claude-channel` 7/7 (new), `cargo clippy -p hub -p claude-channel
  --all-targets -- -D warnings` clean, `cargo check --workspace` clean,
  `cargo fmt --check` clean (formatted only the two files I touched, not
  the concurrent unrelated diffs).
- Updated `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/communication.md`
  (C14.3 now **In progress** with implementation detail), `crates/README.md`,
  and the task board row above. Committed as scoped work — new crate +
  new Hub file + doc updates only. **For Chat/Codex:** implementation and
  unit coverage are ready for review; end-to-end acceptance against a
  real `claude --channels` session is still open and needs the owner's
  Claude Code 2.1.231+ environment.

— claude

### claude — 2026-08-13 — C14.3 registry, crate rename, Shared Hub Channels tab

- User feedback on the prior round (a single `.mcp.json` write with no
  durable app-side record) asked for: an app-owned `servers` registry
  under `~/.coding-assistants/` holding a `global.mcp.json` base layer
  plus one file per workspace, and a Shared Hub list with rename/delete.
  Separately asked mid-turn to rename `crates/claude-channel` to
  `crates/claude`. Both done this round.
- **Crate rename:** `git mv crates/claude-channel crates/claude`; binary
  name (`coding-assistants-claude-channel`) unchanged. Updated the
  workspace `Cargo.toml` members list, `crates/README.md`, and the
  crate's own `Cargo.toml`/`README.md`.
- **Registry architecture** (`hub::bridge::claude_channel`,
  store-relative via `store.data_dir()` — not the process-global
  `default_hub_home()`, to keep tests isolated, same fix pattern as
  Settings S5): `servers_dir`/`global_servers_path`/
  `workspace_servers_path` (`<repo-dir-name>-<4-byte-sha256-hex>.mcp.json`,
  hash suffix proven collision-proof for same-named repos in different
  locations by a dedicated test); `setup_claude_channel` now writes the
  canonical per-workspace file (with `_workspace`/`_display_name`
  bookkeeping metadata that a merge test proves never leaks into the
  workspace's actual `.mcp.json`) and merges `global.mcp.json` + the
  per-workspace entry into it; `list_channel_workspaces`,
  `rename_channel_workspace`, `delete_channel_workspace` added — delete
  removes the canonical file and downgrades the Hub registration to
  `observed` (reusing the existing C14.1 supervisor state machine) but
  leaves the workspace's own `.mcp.json` untouched.
- **CLI:** added `--list`, `--rename --workspace <path> --name <name>`,
  `--delete --workspace <path>` subcommands alongside the existing
  `--setup`; all three share a `canonical_workspace_arg` helper for
  consistent path canonicalization/lookup.
- **Tauri + UI:** three new commands
  (`claude_channel_list_workspaces`/`_rename_workspace`/`_delete_workspace`)
  in `src-tauri/src/harness/commands.rs`, registered in `lib.rs`; a new
  Shared Hub **Channels** tab (`HubPanel.tsx`/`HubPanelView.tsx`) lists
  every configured workspace with an inline rename field and a remove
  button.
- Added `.mcp.json` to `.gitignore` (embeds a machine-local absolute
  binary path; the pre-existing entry only covered the extensionless
  `mcp.json`).
- Re-read the board before starting: `bridge/{codex,gemini}.rs`,
  `settings/store.rs`, `store/agents/mod.rs`,
  `store/tests/integration.rs` were dirty with concurrent work again;
  diffed each, touched none.
- **Verification:** `cargo test -p hub --lib` 110/110 (+6 new),
  `cargo test -p tauri-app` all green (+1 new), `cargo test -p claude`
  7/7, `cargo clippy -p hub -p tauri-app -p claude --all-targets -- -D
  warnings` clean, `cargo check --workspace` clean, `cargo fmt --check`
  clean (`rustfmt` run only on files I authored, not the concurrent
  diffs above), `npx tsc --noEmit` clean, `npm run build` clean (68
  modules). Manually ran `--setup`/`--list`/`--rename` against this
  repo's real workspace and confirmed the registry files and `--list`
  output matched.
- Updated `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/communication.md`,
  `crates/claude/README.md`, and the task board row above. Committed as
  scoped work — no other agent's in-flight files touched. **For
  Chat/Codex:** registry architecture, CLI management, and Shared Hub UI
  are ready for review; end-to-end acceptance against a real
  `claude --channels` session remains the only open item for #150.

— claude

### Claude — 2026-08-13 — C14.3 live acceptance, two desktop bug fixes, and a C14.6–C14.8 audit for Grok/Gemini/Codex

Re-read the board before starting. `bridge/{codex,gemini}.rs`,
`settings/store.rs`, `store/agents/mod.rs`, `store/tests/integration.rs`
were dirty with concurrent work again; diffed each, touched none.

**1. C14.3 Claude Channel — live acceptance completed, #150.** Ran the
actual owner workflow end to end against a real `claude --channels`
session (not just unit tests): plain messages, wake, and task-tagged
sends, confirmed via `ca msg list`/`ca msg send` and the live terminal.
Two real bugs found and fixed along the way:
- **Selective interruption.** Every pending message was being pushed as
  an MCP `notifications/claude/channel` interrupt regardless of kind —
  a plain chat ping disturbed the session exactly like a wake. Split
  `poll_channel_events` (wake or task-tagged only, pushed+acked) from a
  new `poll_quiet_channel_events` (everything else, stays `pending`),
  and added a `check_inbox` MCP tool so Claude can pull quiet traffic on
  its own initiative. Verified live: a plain "Simple ping" sat pending
  through several poll cycles with zero interruption, then `check_inbox`
  correctly drained and acked it.
- **Desktop connect/spawn.** Added `hub::is_channel_session_live`
  (process-table check for a running bridge for a workspace) and
  `hub::launch_claude_channel_session` (opens a real terminal running
  `claude --dangerously-load-development-channels
  server:coding-assistants-channel` — Claude Code's Channel preview has
  no headless daemon mode, so this can never be a detached background
  process like Codex's `app-server`/Gemini's `agy` adapters). Wired
  through `claude_channel_is_connected`/`claude_channel_connect` and a
  status badge + Connect button per workspace row in the Channels tab.
- **Verification:** `cargo test -p hub -p claude` all green (+7 new
  tests for the disturb/quiet split, +1 for the terminal-launch helper),
  `cargo clippy -p hub -p claude --no-deps -- -D warnings` clean,
  `cargo check --workspace` clean.
- Committed as `161720e`.

**2. Settings window open/close/reopen regression, #153 (closed).** Not
a C14 item — a pre-existing UI bug the owner hit mid-session. Root
causes: `core:window:allow-close` was never granted (only
`core:default`'s read-only window permission set), so the in-window
Close button's `close()` call was silently rejected by Tauri's ACL
before ever reaching Rust, while the OS window-manager `X` bypassed
that layer and worked; a panicking `.unwrap()` in the shared
`CloseRequested` handler could take the whole app down; and Settings
was being hidden-and-kept-alive like the tray-resident main window,
so reopening depended on a hidden-window resurrection that didn't
reliably work. Fixed all three; owner confirmed working. Committed as
`bcd13f0`.

**3. Two more desktop chat bugs the owner hit live, fixed this round:**
- **Claude's own messages vanishing on every new reply.**
  `record_channel_reply` gave every reply in the same session the
  identical, non-unique subject `"channel:session:<id>:reply"`. The
  desktop Chat & Memory view's per-post dedup key (`channelDedupeKey`,
  meant to collapse team fan-out *copies* of one broadcast post, not
  distinct sends) treated every reply as the same post and kept only
  the latest — every earlier reply from Claude disappeared the instant
  a new one arrived. Fixed by uuid-suffixing the subject, matching the
  pattern `send_session_message` already uses by default. Regression
  test added (`reply_gives_each_session_scoped_reply_a_distinct_subject`).
- **Read receipts.** New durable `read_markers` table
  (`agent_id, scope, last_read_at`), `HubStore::mark_read`/
  `list_read_markers`, `hub_mark_read`/`hub_list_read_markers` Tauri
  commands, and a `ca msg read`/`ca msg readers` CLI pair (for Grok/
  Gemini/Codex's own bridges to mark themselves as having read a scope,
  once they read a message — no bridge currently calls this; that's
  optional follow-on work for whoever owns each bridge, not required by
  #154/#155/#156 below). The desktop chat now auto-marks the human's
  own view and renders a small "✓✓ Read by ..." line under each message
  once another team member's marker has caught up to it. Claude's
  `reply` tool auto-marks itself read for the session it just replied
  in.
- **Verification:** `cargo test -p hub -p cli` 123/123, `cargo clippy -p
  hub -p cli --no-deps -- -D warnings` clean, `npx tsc --noEmit` clean.
- Committed as `f4f6a20`.

**4. Audit of Grok, Gemini/Antigravity, and Codex's live-session
delivery — per the owner's explicit request, diagnosis only, no fixes.**
The owner reported: Grok responds correctly to messages sent in its own
terminal, but a Hub-sent message never appears there; a task/wake sent
to Gemini/agy produced an off-topic "gibberish" reply, and neither the
message nor the reply appeared in agy's live session; a wake sent to
Codex got no response despite a visibly active live Codex terminal.
Applying the same scrutiny used to build/fix the Claude Channel:

- **Grok — #154, C14.6.** Not a code bug. `deliver_grok_task` already
  implements the real, documented `--leader`/`--leader-socket` ACP
  path correctly (verified against `grok --help`: `--leader`,
  `--leader-socket`, a `leader` subcommand, `[cli] use_leader`). It's
  `"unavailable"` because no leader socket exists on a default
  standalone Grok TUI — the code refuses gracefully rather than
  attempting anything undocumented. The gap is that nothing tells an
  owner Grok needs to run in leader mode for Hub delivery to work at
  all. Task: document the setup requirement (mirror
  `crates/claude/README.md`'s explicit steps), and consider a
  `launch_claude_channel_session`-style connect helper + desktop
  affordance for spawning Grok in leader mode.
- **Gemini/agy — #155, C14.7. Real bug, root-caused.**
  `gemini_managed_spawn_args` builds `agy --print --output-format
  stream-json ... --prompt <message body>`. Per `agy --help` on this
  machine, `--prompt` is a bare alias for `--print`/`-p`, not a
  value-taking flag — the real prompt is almost certainly meant to be a
  **positional** argument. The message body is currently never
  delivered as the prompt at all, which fully explains the
  off-topic/gibberish response (consistent with what `agy` would
  plausibly answer if asked generically about `--output-format` rather
  than the real message). "Doesn't appear in the live session" is
  expected/by-design here — `run_agy_worker` always spawns a disposable
  headless child per task, same shape as Codex's `app-server` adapter,
  and never touches any other running `agy` terminal; that part isn't a
  bug to fix unless live-session delivery becomes an explicit new goal.
- **Codex — #156, C14.8.** `deliver_codex_task_with` resolves a thread
  id from a Hub registration or an on-disk `~/.codex/sessions/**/*.jsonl`
  scan (exact-string `cwd` match); if neither resolves, the send is
  `"unavailable"`, queued, and no live process is ever contacted.
  Nothing in the repo auto-registers a session a user starts by hand in
  a terminal, so a manually-started live Codex session very likely has
  zero Hub registration — that's the most probable reason the wake got
  no response. Separately, even a resolved thread is only ever turned
  via a brand-new disposable `codex app-server` client, never the
  visible interactive TUI directly, so "no response in my terminal"
  could also be structurally expected rather than a bug, depending on
  what actually happened. `HarnessInjectResult` does carry a
  `status`/`detail` distinguishing `unavailable`/`queued`/`delivered` —
  worth confirming the desktop UI actually surfaces it.

Full task lists, exact file/function references, and an explicit "do
not touch another bridge" boundary are in #154/#155/#156 respectively
and in the task-board rows above. **Recommended acceptance workflow for
each** (the same one the owner and I used for #150): send a plain
untagged message and confirm it does *not* disturb the live session (or
document that this provider has no such distinction if that's the
right model for it); send a task- and a wake-tagged message and confirm
each actually reaches and is answered by the *live* session the owner
is looking at, not just a disposable headless call; check the Hub
message record (`ca msg list`) matches what actually happened rather
than trusting the UI alone.

Updated `docs/CHANGELOG.md`, `docs/moon/roadmaps/communication.md`, and
the task board above. Created and closed #153 (Settings). Created
#154/#155/#156 (unclaimed — Grok/Gemini/Chat-Codex to pick up
respectively). Did not implement any of the three fixes myself, per the
owner's explicit instruction — diagnosis and task assignment only.

— claude
