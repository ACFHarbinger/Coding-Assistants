# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Claude — Settings S5 final relocation (#131) (2026-08-13)

- Review returned S5 once more: a legacy Shared Hub → Policy tab still
  duplicated the standing wake-policy controls the new Settings
  Orchestration tab was supposed to be the sole editor for, and its second
  field (`allow_auto_wake`) had never been surfaced in the new flow at
  all — only `default_requires_human_gate` (renamed `confirm_wakes`) was
  wired.
- **Backend:** added `allow_auto_wake` to `StandingPolicySnapshot`
  (`src-tauri/src/hub/commands/settings.rs`), populated it in
  `settings_get_standing_policy`, and added a new
  `settings_set_allow_auto_wake` command (registered in `lib.rs`)
  mirroring `settings_set_confirm_wakes`. Both continue to compose with
  the existing Hub `WakePolicy` storage, not duplicate it.
- **Frontend:** `SettingsApp.tsx`'s Orchestration tab gained a second
  "Allow auto-wake requests" toggle next to "Confirm before wakes",
  reusing the existing `ToggleRow`/`StandingPolicySnapshot` plumbing.
- **Retired the legacy tab entirely:** removed the `"policy"` `HubTab`
  entry and its tab button (`HubPanelView.tsx`), the rendered Wake Policy
  Controls section and its two checkboxes, the `WakePolicyCheckbox`
  component (`HubCharts.tsx`, no other importer), the `wakePolicy`
  state/`refreshPolicy`/`updatePolicy` logic and prop wiring
  (`HubPanel.tsx`), and the now-unused `WakePolicy` frontend interface
  (`hub/types.ts`). Left `hub_get_wake_policy`/`hub_set_wake_policy`
  registered in `lib.rs` — they're generic, harmless, and Settings itself
  doesn't call them (it goes straight through `HubStore`), so removing
  them wasn't necessary to complete the relocation. Verified
  `MessagerPanel.tsx`'s unrelated `wakePolicyGate` (a per-message compose
  flag, not the standing `WakePolicy` object) was untouched.
- **Regression coverage:** added
  `standing_policy_exposes_and_updates_both_wake_policy_fields` in
  `src-tauri/src/hub/commands/tests.rs`, asserting both fields round-trip
  independently (setting one never clobbers the other) and persist across
  a fresh read.
- Verified with `cargo test -p hub --lib` (87/87), `cargo test -p
  tauri-app` (47/47 +1 ignored, +1 new), `cargo clippy -p hub -p
  tauri-app --all-targets -- -D warnings` clean, `cargo check --workspace`
  clean, `cargo fmt --check` clean (formatted only files this change
  touched, to avoid reformatting Gemini's concurrently in-flight `[tui]`
  work in `crates/hub/src/settings/store.rs`), `npx tsc --noEmit` clean,
  `npm run build` passes.

### Gemini — TUI T3 dynamic prefix chord, settings persistence & capability fallback (#137) (2026-08-13)

- Added durable `[tui]` section serialization and setters (`set_tui_prefix_chord`, `set_tui_unicode_fallback`, `set_tui_bell_notification`, `set_tui_high_contrast`) in `crates/hub/src/settings/store.rs`.
- Implemented dynamic configured-prefix chord matching (`is_prefix_chord_key`) supporting `ctrl+b`, `ctrl+a`, `ctrl+x`, `ctrl+g`.
- Added environment capability detection (`is_ascii_terminal`) falling back to ASCII glyphs on ASCII/linux/dumb terminals or when `unicode_fallback` is enabled.
- Added disk persistence tests in `crates/tui/tests/navigation_test.rs`.
- **Verification:** All 141 workspace tests pass (`cargo test`); `cargo clippy --workspace --all-targets -- -D warnings` clean; `npm run build` passes.

### Chat / Codex — review: T3/S5 returned; C13 evidence preparation accepted (2026-08-13)

- Returned T3 #137: `[tui]` values are parsed but are neither saved nor used
  to match the configured prefix chord; capability detection and the desktop
  notification path are also still absent. The interaction foundation remains
  accepted.
- Returned S5 #131: its new Settings tab works against existing typed commands,
  but the legacy Shared Hub policy editor remains active and its `allow_auto_wake`
  control was not relocated. The two editors must be unified before S5 closes.
- Accepted C13's evidence template and read-only shell preflight as preparatory
  documentation only. Added #146 for a non-mutating `ca` preflight inspector;
  live owner evidence in #113 remains the closing gate.

### Claude — Settings S5 Orchestration tab (#131) (2026-08-13)

- Added the Orchestration tab to `src/components/settings/SettingsApp.tsx`
  using the already-typed S5 IPC surface: a Global-defaults/This-workspace
  scope toggle (matching Workspace & sessions and Memory & storage); a new
  `ToggleRow` control for the five boolean policy fields (standing wake
  confirmation, confirm-new-enrollment, confirm-broadcast,
  auto-enrollment-allowed, export-enabled) with Inherited/Workspace
  Override status pills and Reset to Global where a workspace override
  exists; a three-way Strict/Standard/Permissive sandbox-strictness
  selector; a retention-days field (empty = indefinite, workspace override
  always names a concrete day count); and a per-agent budgets list plus
  set-budget form (global only, reading/writing the same Hub budget table
  every C6 flow already uses).
- Standing wake confirmation is deliberately global-only in the UI too,
  matching the backend: `WakePolicy` has no per-workspace concept, so its
  toggle doesn't participate in the scope switch or show a status pill.
- No new Tauri commands or backend changes — every control here calls the
  S5 typed API (`getStandingPolicy`, `setConfirmWakes`,
  `updateOrchestrationPolicy`, `setRetentionDays`, `listAgentBudgets`,
  `setAgentBudget`, `resetSettingsField`) that already existed and was
  already tested. No secret fields anywhere on this tab.
- Verified with `npx tsc --noEmit` clean and `npm run build` passes.
  Rust side unchanged — did not touch `crates/hub/src/settings/**` or any
  Tauri command file, since Gemini's TUI `[tui]` settings follow-on was
  concurrently live in those same files.

### Grok — C13 evidence template and preflight helper (#113) (2026-08-13)

- Added a copy-paste preflight helper (read-only hashes of the Markdown
  fallback) and an issue-comment-ready evidence template under the C13
  gate in `docs/moon/roadmaps/communication.md`.
- Does not record owner evidence and does not change runtime behavior.
  #113 stays open until Harbinger completes the live run.

### Chat / Codex — review: S5 enforcement, T3 foundation, and C13 checklist (2026-08-13)

- Accepted S5's runtime enforcement pass after end-to-end workspace
  verification. A strict sandbox now also has a focused injection-path test.
  Issue #131 remains open for its Settings-window Orchestration controls.
- Accepted Grok's C13 owner-run checklist as the required evidence handoff;
  #113 remains open until Harbinger performs and records the live run.
- Returned T3 #137 for its remaining preference, pane-prefix, terminal
  fallback, and notification contract. Restored the T2 generic retryable
  failure notice and added a regression test so local Hub errors are not
  exposed in the terminal UI.
- Accepted C10/C11's durable all/subset/one and task/wake boundary after
  their store, Tauri, CLI, and desktop paths passed the full workspace suite.
  The original C12 issue #112 is superseded by the reviewed #145 bridge.

### Claude — Settings S5 policy enforcement (#131 returned) (2026-08-13)

- Review returned S5 for runtime enforcement: the persisted orchestration
  policy needed to actually gate the live auto-enrollment, export, and
  sandbox-strictness paths, not just be stored/exposed. Wired all three,
  preserving C10/C11 semantics under default settings (auto_enrollment_allowed
  and export_enabled both default `true`) — no existing test's behavior
  changed.
- **Auto-enrollment** (`crates/hub/src/store/messages/mod.rs`,
  `send_tagged_message`): a wake refuses to enroll a brand-new (not-yet-
  team-member) identity when `orchestration.auto_enrollment_allowed` is
  false, recording a `wake_refused_auto_enrollment_disabled` outcome with
  no membership mutation — mirroring the existing `task_refused_not_present`
  shape. Adding an *already*-team-member to a session is a distinct,
  always-allowed concern and stays unaffected. Fixed the policy lookup to
  resolve against `self.data_dir()` rather than the process-global
  `default_hub_home()`, so a `HubStore` opened at an arbitrary path (every
  hub-crate unit test) reads its own co-located `settings.toml`.
- **Export permission** (`src-tauri/src/hub/commands/messaging.rs`):
  `hub_export_markdown`/`hub_export_markdown_git` refuse with a clear
  error when `orchestration.export_enabled` is false (global scope — there
  is no per-workspace export today).
- **Sandbox strictness** (`src-tauri/src/harness/commands.rs`):
  `hub_start_harness`/`hub_inject_harness` refuse the `vibe` harness (the
  only identity that unconditionally passes `--trust`/`--auto-approve` —
  see `crates/hub/src/harness/mod.rs::vibe_spawn_args`) when the target
  workspace's effective `sandbox_strictness` is `Strict`; `Standard`/
  `Permissive` are unchanged. Gated at the shared C12 dispatch boundary
  (`hub_start_harness`/`hub_inject_harness`) rather than inside any harness
  adapter file, so no adapter was touched and the block happens before any
  process ever spawns.
- Made `crate::hub::commands::tests::CA_HOME_ENV_LOCK` `pub(crate)` so the
  new `harness::commands::tests` module coordinates on the same
  process-global `CA_HOME` mutex instead of racing a second one.
- Added focused tests: two in `crates/hub/src/store/tests/workflows.rs`
  (refusal on a new identity; unaffected session-add for an existing
  member), four in `src-tauri/src/harness/commands.rs` (strict blocks only
  `vibe`; standard/permissive never block; a workspace override can relax
  a strict global default; `hub_start_harness` rejects before spawning),
  one in `src-tauri/src/hub/commands/tests.rs` (export commands honor the
  policy both ways).
- Verified with `cargo test -p hub --lib` (87/87, +2 new), `cargo test -p
  tauri-app` (45/45 +1 ignored, +5 new), `cargo clippy -p hub -p
  tauri-app --all-targets -- -D warnings` clean, `cargo check --workspace`
  clean, `cargo fmt --check` clean, `npx tsc --noEmit` clean, `npm run
  build` passes.

### Gemini — TUI T3 interaction foundation (#137) (2026-08-13)

- Implemented conventional and Vim-style navigation (`Tab`/`Shift+Tab`, `h`/`j`/`k`/`l`, `Left`/`Right`/`Up`/`Down`, `g`/`G`) and list scrolling in `crates/tui/src/app.rs`.
- Added mouse click hit-target tab selection and wheel scrolling support via Crossterm mouse capture.
- Created popup Help Cheat-Sheet modal (`?` or `F1`) and modal Command Palette (`/` or `Ctrl+P`) with command execution (`1:orchestrate`, `2:chat`, `3:hub`, `4:settings`, `refresh`, `help`, `quit`).
- Added unit test `test_tui_app_state_navigation_and_command_palette` in `crates/tui/tests/navigation_test.rs`.
- The complete T3 preference/notification contract remains tracked in #137:
  persistent `[tui]` preferences, configurable pane prefix, ASCII fallback,
  and KDE notification/bell delivery are not yet implemented.

### Grok — C13 owner-run acceptance checklist (#113) (2026-08-13)

- Expanded the C13 migration gate in `docs/moon/roadmaps/communication.md`
  into a reproducible owner-run checklist: preflight hashes of the Markdown
  fallback, all/subset/one plus task/wake coverage, two harness captures,
  one audited delivery, reconstruction without the bus, and a recovery path
  that does not rewrite historical `.agent` records.
- C12 remains accepted. This slice is documentation only; live owner
  evidence on #113 is still required before the bus is demoted.

### Chat / Codex — TUI and harness bridge review acceptance (#136, #145) (2026-08-13)

- Accepted TUI T2 after workspace tests, Clippy, and the production frontend
  build passed. The TUI now treats a failed Hub snapshot as a visible,
  retryable condition instead of silently presenting it as an empty Hub; a
  regression test covers an unreadable Hub home.
- Accepted C12's provider-safe bridge: all four harnesses capture into the
  shared session, Codex uses its documented app-server delivery route, Grok
  uses its documented ACP leader route, and unsupported Claude/Gemini control
  transports remain explicitly queued and unavailable. No task-only path
  writes a PTY or starts a replacement harness.
- Returned Settings S5 #131 for runtime enforcement. Its policy values and
  typed commands are durable, but auto-enrollment, export permission, and
  sandbox strictness are not yet consumed by their respective live flows.

### Claude — Settings S5 orchestration and storage policy backend (#131) (2026-08-13)

- Added `hub::settings::OrchestrationPolicy` (global) and
  `OrchestrationOverride` (per-workspace), following the same
  merge/inheritance pattern as `backup_retention`: confirm-new-enrollment,
  confirm-broadcast, auto-enrollment-allowed, `SandboxStrictness`
  (strict/standard/permissive — a coarse ordinary-tier control; per-tool
  allow/deny lists are Advanced-tier future work), retention-days
  (`None` = indefinite), and export-enabled. Persisted as a new
  `[orchestration]` table plus an inline `orchestration = { ... }` table
  inside each `[[workspace]]` entry. `EffectiveSettings.orchestration` now
  carries the merged view with per-field Inherited/Override status.
- Deliberately did **not** move `WakePolicy`'s existing
  `default_requires_human_gate` storage out of `HubStore` — every C10-C13
  wake path already reads it there, and migrating it would mean touching
  every one of those call sites instead of composing at the IPC layer.
  Settings still becomes the sole *editor*: `settings_get_standing_policy`
  and `settings_set_confirm_wakes` (`src-tauri/src/hub/commands/settings.rs`)
  compose the new orchestration policy with the existing `WakePolicy`.
- Exposed per-agent budgets through Settings' typed command surface
  (`settings_list_agent_budgets`, `settings_set_agent_budget`) without
  duplicating storage: added `HubStore::list_agent_budgets` (a small
  additive read over the existing `agent_budgets` table) and delegated the
  setter to the existing `set_agent_budget`.
- New typed commands: `settings_update_orchestration` (global or
  workspace-scoped patch, audits each changed field),
  `settings_set_retention_days` (global accepts `None` to mean
  indefinite; a workspace override always names a concrete day count —
  `settings_reset_field` clears it instead).
- Frontend contract only (`src/components/settings/{types,api}.ts`) — no
  Settings-window UI. Wiring an Orchestration/Advanced tab, and budget/
  sandbox controls into the window, is follow-up work for whichever slice
  picks up the Settings-window UI next.
- Verified with `cargo test -p hub --lib` (85/85, +5 new: 4 orchestration-
  policy tests plus `list_agent_budgets_returns_every_configured_agent`),
  `cargo clippy -p hub -p tauri-app --all-targets -- -D warnings` clean,
  `cargo check --workspace` clean, `cargo fmt --check` clean, `npx tsc
  --noEmit` clean, `npm run build` passes.

### Grok — C12 provider-safe harness bridge (#145) (2026-08-13)

- Added `hub::deliver_codex_task`: task-only Chat/Codex delivery uses the
  documented app-server `initialize` / `thread/resume` / `turn/start` path
  when a persisted thread is registered (`diskSessionId`) or found on disk.
  Missing thread or app-server failure is `unavailable` with `pid = None`;
  the durable inbox keeps the task.
- Wired `inject_harness_with_store` for `chat`/`codex` through that bridge.
  Grok still uses ACP; Claude and Gemini remain explicit `unavailable`.
  Task-only inject never starts a replacement harness or writes a PTY.
- Tests: registered-thread delivery (injected RPC), missing-thread
  unavailable, and four-harness task-only inject outcomes.

### Gemini — TUI T2 shared read model & responsive shell (#136) (2026-08-13)

- Implemented `HubReadModel` in `crates/tui/src/model.rs` providing a unified, read-only snapshot of Hub data (work sessions, team roster, channel messages, tasks, settings audit events, effective settings) directly from `HubStore` and `SettingsStore`.
- Integrated `HubReadModel` into `crates/tui/src/app.rs` with responsive Ratatui views across all desktop-parity tabs (Orchestrate, Chat & Memory, Shared Hub, Settings) and added an `[r]` manual/on-demand read model refresh command.
- Added unit test `test_hub_read_model_loads_coherent_data` in `crates/tui/tests/model_test.rs`.
- **Verification:** `cargo test` passes 97 unit and integration tests across all workspace crates; `cargo clippy --workspace --all-targets -- -D warnings` clean; `npm run build` passes.

### Grok — Settings S4 typed profile/harness commands (#130) (2026-08-13)

- Added redacted Tauri commands for profile list/upsert/rename/remove,
  workspace default-profile select/reset, and harness list/update. Upsert
  accepts only a `SecretReference`; listed snapshots expose source badges
  and never a credential.
- Extended the frontend settings API contract (`types.ts` / `api.ts`) to
  match those commands. No Agents tab or Settings window UI in this slice.
- Verified with `settings_profile_and_harness_commands_are_redacted_and_durable`,
  `cargo clippy -p tauri-app --all-targets -- -D warnings`, and `npx tsc --noEmit`.

### Chat / Codex — Settings S4 review acceptance (#130) (2026-08-13) [DRAFT]

- Accepted the completed profile/harness command boundary after Hub/Tauri tests,
  clippy, and the frontend build passed. Profile reads stay redacted and every
  mutation records a settings-audit event.

### Chat / Codex — Settings/TUI review and lifecycle integration (#129, #135) (2026-08-13) [DRAFT]

- Accepted the standalone reusable Settings window and the corrected `ca tui`
  default-setting flags after combined Rust and frontend verification. The TUI
  selectors are invocation-only unless their explicit persist flags are used.
- Exercised the actual TUI default-persistence/audit helper in its integration
  test and applied the standard Rust formatter to the new TUI crate.
- Corrected the tray-style main-window close path so it also hides the
  independent Settings window, while Settings may still close independently.
- Returned Settings S4 #130 for its remaining typed Tauri profile/harness
  command boundary; its validated durable-storage half is accepted.

### Claude — Settings S3 standalone window (#129) (2026-08-13)

- Added the approved standalone Settings window using Tauri's multiwindow
  API: `src/lib/settingsWindow.ts` opens/focuses a single reusable
  `"settings"` `WebviewWindow` (`index.html#/settings`), branched in
  `src/main.tsx`. Reopening an already-open window `show()`s + `setFocus()`s
  it rather than creating a duplicate — required because the app's global
  `CloseRequested` handler hides windows instead of destroying them
  (tray-resident behavior), so a plain `setFocus()` on a hidden window
  would silently do nothing.
- Added `src-tauri/capabilities/default.json` permissions
  (`core:webview:allow-create-webview-window`, `core:window:allow-set-focus`)
  needed for that window-creation call; neither is in Tauri's `core:default`
  set. Added the `"settings"` window label to the existing capability.
- Restored the header's Settings button in `src/App.tsx` (the prior review
  scaffold had removed it) to call the new opener instead of toggling a
  modal.
- `src/components/settings/SettingsApp.tsx`: the window's root component —
  WAI-ARIA `tablist`/`tab`/`tabpanel` with arrow-key/Home/End navigation,
  dark glass-morphism styling, Escape-to-close. General and Workspace &
  sessions tabs are real, backed by Settings S2 IPC end-to-end:
  `default_workspace` (General; global-only, no per-workspace override
  exists for "which workspace opens by default") and `default_session`
  (Workspace & sessions; global default with a workspace override, full
  Inherited/Workspace Override status pill and Reset to Global). Added a
  Memory & storage tab for `backup_retention` as a low-risk bonus using
  already-committed S2 IPC. Remaining tabs stay honest structural
  placeholders pending S4/S5/S6 fields. A collapsible panel lists recent
  settings-audit events end-to-end.
- Extended `src-tauri/src/hub/commands/settings.rs` with
  `settings_set_default_workspace` and `settings_set_default_session`
  (registered in `lib.rs`) since the existing generic `settings_update`
  patch can't express "clear an optional field back to unset" — these two
  fields needed dedicated three-state (untouched/set/cleared) commands.
- Verified with `cargo test -p hub --lib` (76/76, unaffected — no
  settings-store code touched, only new Tauri commands calling existing
  S1/S2/S4 store methods), `cargo clippy -p hub -p tauri-app --all-targets
  -- -D warnings` clean, `cargo check --workspace` clean, `cargo fmt
  --check` clean, `npx tsc --noEmit` clean, `npm run build` passes.

### Grok — Settings S4 profiles and harness configuration (#130) (2026-08-13)

- Added global named `ProviderProfile` records to `settings.toml`
  (`[[profile]]`) with non-secret model/base URL fields and a
  `SecretReference` (`keychain` id, env-var *name*, or existing provider
  login). Snapshots expose source badges only; raw credentials are
  rejected on write and never stored.
- A workspace selects a default profile per harness by name and does not
  copy profile fields. Rename updates those references; remove clears
  them and never deletes a keychain secret.
- Persisted validated `[harness.<id>]` executable, absolute workdir,
  capture-polling, and inject-permission settings. Executables must be a
  single program name or path (no shell).
- Verified with `cargo test -p hub --lib` (76 passed) and
  `cargo clippy -p hub --all-targets -- -D warnings`.

### Gemini — TUI T1 foundation & default settings persistence fix (#135) (2026-08-13) [DRAFT]

- Persisted `--set-as-default-workspace-settings` and `--set-as-default-session-settings` through `SettingsStore` (`default_workspace`, `default_session`, and per-workspace `default_session` overrides).
- Recorded redacted audit events (`general.default_workspace`, `workspace.default_session`) on `HubStore` during setting persistence.
- Loaded effective settings defaults automatically when starting `ca tui` without explicit CLI selector overrides.
- Added `test_set_as_default_workspace_and_session_settings_persistence_and_audit` test in `crates/tui/tests/options_test.rs`.

### Chat / Codex — Settings S1/S2 review acceptance (#127, #128) (2026-08-13) [DRAFT]

- Accepted the versioned, comment-preserving local settings store and its
  redacted typed IPC/scope/audit boundary after `cargo test -p hub -p cli -p
  tui` passed (78 tests). Settings backups default to three retained timestamped
  files; workspace paths remain deliberately distinct.
- Returned TUI foundation #135 for a narrow correction: its opt-in
  `--set-as-default-…-settings` flags must persist through the typed
  Settings/audit path instead of only changing in-memory UI status.

### Grok — C10–C13 S3 durable delivery semantics (2026-08-13)

- `send_tagged_message` now fails before any write when the session id is
  unknown, enrolls a wake target into the work session even if they are
  already on the standing team, and records a stable `policy_decision` on
  every per-recipient `SendOutcome` (`task_refused_not_present`,
  `accepted`, `wake_enrolled`, `wake_denied_policy`, `wake_denied_budget`).
- Task still refuses a recipient who is not a current team+session member
  and does not spawn or enroll. Mixed task+wake keeps that refuse-first
  rule. `send_session_message` also reports a missing session as not found.
- `ca msg send` and `hub_send_message` reject kind `wake`; tagged delivery
  must go through `ca msg tag` / `hub_send_tagged_message`.
- Verified with `cargo test -p hub --lib` (70 passed) and
  `cargo clippy -p hub -p cli -p tauri-app --all-targets -- -D warnings`.

### Claude — Settings S2 typed IPC and scope resolution (#128) (2026-08-13)

- Extended `hub::settings` (S1) with workspace-override resolution:
  `WorkspaceOverride`, `FieldStatus` (`Inherited`/`Override`),
  `EffectiveSettings`, and `SettingsField`. `SettingsStore::effective`
  deterministically merges the global default with a workspace's override;
  `set_workspace_backup_retention`/`reset_workspace_field` mutate it.
  Workspace identity is the exact path string given — never
  symlink-resolved — so distinct paths to one repository keep separate
  overrides. Overrides persist as a `[[workspace]]` array-of-tables,
  rewritten on save alongside the existing `[storage]` handling.
- Added redacted Tauri commands in `src-tauri/src/hub/commands/settings.rs`:
  `settings_get_effective`, `settings_get_load_status`, `settings_update`,
  `settings_reset_field`, `settings_list_audit_events`. None return a
  filesystem path; `settings_get_load_status` mirrors `LoadStatus` with its
  path stripped.
- Added `HubStore::record_settings_audit_event` /
  `list_settings_audit_events` (`crates/hub/src/store/policies/settings_audit.rs`):
  a dedicated, redacted settings-audit view (`root_path == "settings"`,
  `process_json` carries only `field`/`scope`) that is a typed filter over
  the same hash-chained `audit_events` table other Hub consumers already
  read, not a second table. Settings changes are recorded and immediately
  marked `approved` since the IPC call itself is the confirmation.
- Added frontend typed IPC client `src/components/settings/{types,api}.ts`
  mirroring the Rust DTOs, for the S3 Settings window to consume.
- Backup listing/restore IPC is intentionally deferred to S3 so it can pair
  the "restore last known good" action with real UI rather than exposing a
  bare-path or backup-id surface ahead of that need.
- Verified with `cargo test -p hub --lib` (67 passed, up from 60),
  `cargo clippy -p hub -p tauri-app --all-targets -- -D warnings`,
  `cargo check --workspace`, `npx tsc --noEmit`, and `npm run build`.

### Grok — Settings S1 versioned store (#127) (2026-08-13)

- Added `hub::SettingsStore` for versioned `settings.toml` under
  `CA_HOME` or `~/.coding-assistants`. Loads use `toml_edit`, never crash
  on a missing/malformed/unreadable file, and leave a broken original in
  place. Saves validate, write a sibling temp file, fsync, and replace
  atomically. Hand-authored comments and unknown tables survive a save.
- Default backup retention is three timestamped copies in
  `settings-backups/` (bounded `1..=20`). Restore is path-checked to that
  directory; a malformed file can be quarantined before defaults are
  written.
- Centralized `CA_HOME`/home resolution as `hub::default_hub_home` and
  pointed the Tauri hub store, orchestrator, and `ca` CLI helper at it.
- Verified with `cargo test -p hub --lib` (60 passed) and
  `cargo clippy -p hub --all-targets -- -D warnings`.

### Gemini — C10–C13 S1 session lifecycle UX (2026-08-13) [DRAFT]

- Verified and completed Orchestrate Create/Load team chat session controls in `ConfigPanel.tsx` and `App.tsx`.
- Replaced browser `alert()` popups with inline error feedback banners and name validation (1 to 120 characters) for work session creation and loading.
- Preserved workspace root (`ca.workspaceRoot`) and active work session (`ca.activeWorkSessionId`) persistence across app reloads.
- Added `work_sessions_reject_empty_or_oversized_name` unit test in `crates/hub/src/store/tests/workflows.rs`.

### Gemini — TUI T1 foundation (#135) (2026-08-13) [DRAFT]

- Implemented `crates/tui` crate and `ca tui` subcommand entrypoint in `crates/cli` (U7 deliverable T1 / #135).
- Created safe terminal lifecycle manager using Crossterm and Ratatui with a custom panic hook ensuring raw mode and alternate screen state are always cleanly restored on exit or panic.
- Supported invocation selector flags `--workspace <path>`, `--session <id>`, `--set-as-default-workspace-settings`, and `--set-as-default-session-settings`, with strict validation and error feedback.
- Rendered responsive Ratatui layout covering header status, tabbed navigation (Orchestrate, Chat & Memory, Shared Hub, Settings), active workspace/session indicators, and footer keyboard controls.

### Chat / Codex — desktop crash recovery boundary (#143) (2026-08-13) [DRAFT]

- Wrapped the desktop React root in a top-level error boundary. A render failure
  now shows a local recovery screen with Reload application instead of leaving
  a blank window; internal exception details stay development-console-only.
- Verified with the production frontend build. The root app does not yet have a
  frontend unit-test harness, so the issue retains a follow-up forced-throw
  boundary-test acceptance item for when that harness is added.

### Chat / Codex — Ratatui TUI delivery plan (#134) (2026-08-13) [DRAFT]

- Approved a first-class Kubuntu-focused `ca tui` programme with feature parity
  for orchestration, Chat & Memory, Shared Hub, wake approvals, and supported
  Settings access. It shares durable Hub state and policy/audit contracts with
  desktop rather than creating a second agent-control model.
- Defined the interactive-harness safety boundary: the TUI can render and
  accept user input for explicitly launched, owned PTY processes only; observed
  provider sessions stay read-only and C10–C12 safety constraints remain.
- Locked dark high-contrast terminal styling, tmux-style configurable pane
  controls, local multi-instance reject-and-refresh behavior, KDE notification
  plus optional bell, and `portable-pty`/virtual-terminal acceptance coverage.

### Chat / Codex — persistent settings delivery plan (#126) (2026-08-13) [DRAFT]

- Approved the local-first Persistent Settings plan: a standalone reusable Settings window, tabbed General through Danger zone surfaces, global defaults with deliberate path-distinct workspace overrides, global named provider profiles, and redacted settings auditing.
- Locked versioned, comment-preserving `settings.toml` persistence below `CA_HOME`/`~/.coding-assistants`, atomic writes, selected-backup recovery, and three timestamped last-known-good backups by default with user-configured retention.
- Established safe orchestration defaults: confirmation is required for wakes, new enrollment, and broadcast delivery; task remains non-spawning. The first release includes retention, non-destructive export, and backup configuration; irreversible transcript/memory purges require the later danger-zone flow.

### Chat / Codex — Messager startup recovery (#113) (2026-08-13) [DRAFT]

- Fixed the blank desktop window caused by `MessageStream` reading missing
  `channelMessages`, `contextMenu`, and `mutating` props during the initial
  Chat & Memory render. The production build passes and a rendered Vite smoke
  check now reaches the Chat & Memory empty state.

### Chat / Codex — documentation website test organization (#123) (2026-08-13) [DRAFT]

- Moved the website unit suites to `docs/website/tests/unit/` and migrated
  them from the Node test runner to the project’s configured Vitest harness.
  Added an integration check that joins the generated documentation manifest,
  MiniSearch index, and canonical roadmap instead of a borrowed fake health
  API/MSW fixture.
- Replaced borrowed Cypress assumptions with Coding-Assistants HashRouter
  smoke and E2E flows for the landing CTAs, docs reader, command-palette
  navigation, and persisted light-theme selection. Added documented commands
  for unit, integration, and Cypress runs.
- `npm test` passes 32 Vitest tests; `npm run build` passes; and Cypress 14.5.4
  passes all four local Chrome smoke/E2E flows.
- Kept and aligned the new Vitest configuration with the Vite project, and
  turned the new ESLint entrypoint into a local flat config with Cypress
  globals and a runnable `npm run lint` script. Removed the incompatible
  unused Next.js re-export/stack: this site has no Next application or Next
  dependency, and its supported build and Pages path is Vite.
- Replaced borrowed TypeDoc simulation entry points and Mobile Fortress naming
  with the website’s shared types and search API. `npm run docs:api` now emits
  an ignored local Markdown reference rather than creating a competing Pages
  output.
- Added root-package `docs:*` command proxies, including `npm run docs:dev`,
  so the isolated Vite website can be launched and verified from the repository
  root without mixing it with the desktop app toolchain.
- Serialized the two production-output Vitest suites and set their explicit
  30-second hook budget after GitHub’s clean runner showed concurrent Vite
  builds exceeding Vitest’s default 10-second hook timeout.
- Verified the stabilization in clean GitHub Pages workflow `31676870915`
  (install, test, build, artifact upload, and deployment all passed), then
  closed documentation epic #116 and its completed W1–W7 issues #117–#123.

### Chat / Codex — W3 public-reader acceptance repair (#119) (2026-08-13) [DRAFT]

- Replaced the reader’s remaining fixed dark/cyan chrome with the shared
  dark-first indigo theme tokens, so the sidebar, table of contents,
  pagination, headings, Mermaid fallback, and code-copy controls match the
  landing-page system in both themes.
- Prevented React Markdown’s internal `node` value from being forwarded onto
  rendered `<code>` elements, and routed unknown documentation slugs to the
  custom error page instead of silently opening the default article.
- Extended the browser-chrome regression suite to cover reader token use,
  the safe Markdown code override, and unknown-document recovery. `npm test`
  (30 tests) and `npm run build` pass locally; the repair is ready for Pages
  deployment verification.

### Chat / Codex — public-site acceptance corrections (#120) (2026-08-13) [DRAFT]

- Replaced a public landing-page “Slack-like” reference found during the live
  Pages visual pass with the project’s required Messager terminology. Added a
  regression assertion so the forbidden legacy name cannot return.

### Claude — W7 print stylesheet and custom 404 (#123) (2026-08-13) [DRAFT]

- Added a `@media print` stylesheet (`src/styles/index.css`): hides
  `header`/`footer`/`aside`/`nav` chrome and the skip-link via existing
  semantic selectors (no reader/shell component edits needed), forces the
  `.markdown-body` article onto a light, ink-friendly background
  regardless of the active on-screen theme, avoids page breaks inside
  code blocks/tables/blockquotes/images, appends external link URLs after
  the link text (a printed page can't be clicked), and hides
  copy-to-clipboard buttons.
- Replaced the catch-all route's blind `<Navigate to="/" replace />` with
  a real custom 404 page (`src/features/errors/NotFoundPage.tsx`):
  shows the attempted path, a Cmd+K/Ctrl+K search hint, and Home/Docs/
  GitHub recovery links. Necessary because `HashRouter` never round-trips
  a bad path to a server — there's no host-level 404 to fall back on.
- Added `tests/print-and-404.test.ts`: static checks against the real
  built `dist/` output (print media block present with the expected
  hides, 404 copy present in the bundle) plus a `main.tsx` source check
  that the catch-all route no longer silently redirects.
- `npx tsc --noEmit` clean; `npm test` — 29/29 tests pass (up from 22).

### Grok — W4/W5 Pages acceptance (#120, #121) (2026-08-13) [DRAFT]

- Public GitHub Pages was enabled and the React Docs workflow successfully
  deployed commit `9fa3bce`. The landing/navigation pass found the expected
  layout, Hub graphic, navigation, and CTAs; the remaining public legacy-name
  wording was corrected by Chat/Codex and awaits the next deployment.
- Added `docs/website/tests/landing-nav-acceptance.test.ts` for the landing/navigation share of acceptance: dark-first theme boot, no Google Fonts in `index.html`, Hub graphic and docs/GitHub CTAs, slash-based Roadmap slug, skip link, mobile-drawer ARIA, Cmd/Ctrl+K MiniSearch, and persisted Dark/Light/System.
- The follow-up deployment will also confirm the refreshed Messager wording.
  No reader, print/404, or workflow files were changed.

### Chat / Codex — documentation review follow-up (#119, #122) (2026-08-13) [DRAFT]

- Reviewed Gemini's reader migration and completed the manifest integration:
  public “not published” notices now derive from
  `manifest.unpublishedLinks` for the page that contains such a link, rather
  than unreachable draft/unpublished flags on published documents.
- Removed the obsolete `marked` and `rehype-raw` dependencies. Canonical
  Markdown is rendered with the locked `react-markdown` + GFM + heading-slug
  path and does not opt into raw HTML rendering. Code-copy feedback now uses a
  stable value instead of a freshly random identifier on every render.
- Added a `pretest` content-generation step so clean CI checkouts generate
  ignored `src/content/` artifacts before tests import or build the website.
  Removed duplicate local-font imports from the entrypoint.

### Claude — W7 polish and release confidence (#123) (2026-08-13) [DRAFT]

- New static regression suite (`tests/privacy-a11y.test.ts`, run as part of
  `npm test`): builds the real site and asserts the output makes **no
  third-party font, analytics, or tracker requests** (checked against a
  concrete denylist — Google Fonts, Google Analytics/Tag Manager, Plausible,
  Segment, Mixpanel, Sentry, Facebook, DoubleClick, Hotjar — plus a generic
  "no external `http(s)://` in `dist/index.html`" check), no inline
  cookie-setting or consent-banner code, that the AGPL license reference
  survives minification, and that the shared app shell keeps its
  skip-to-content link and semantic landmarks (`<header>`/`<main
  id="main-content">`/`<nav>`/`<footer>`). All checks run against the real
  built `dist/` output and source, not a mock.
- Fixed a real navigation regression: `AppShell.tsx`'s hardcoded "Roadmap"
  links (header nav + footer) still used the pre-W2 dash-based slug
  (`moon-roadmaps-documentation`); the current pipeline emits slash-based
  slugs (`moon/roadmaps/documentation`), so those links silently 404'd.
  Both fixed.
- Added `docs/website/RELEASE_CHECKLIST.md`: a concise manual pass to run
  alongside the automated gate — deep-link reload, search, theme toggle,
  reduced-motion, one Mermaid page, keyboard-only navigation, mobile
  width, and a clean browser console — plus a post-deploy verification and
  rollback reminder.
- Added Open Graph / Twitter card metadata to `index.html` (title,
  description, type, site_name) — previously entirely absent, so shared
  links rendered as a bare URL on every platform. `og:image`/`twitter:image`
  point at the existing self-hosted `favicon.svg`; a dedicated 1200×630
  raster social-card image is a non-blocking follow-up.
- `npx tsc --noEmit`, `npm test` (22 tests), and `npm run build` all clean.

### Grok — W4/W5 landing and navigation QA (#120, #121) (2026-08-13) [DRAFT]

- Replaced leftover cyan header, badge, active-nav, and footer hover classes
  with indigo/purple tokens on the landing and AppShell chrome.
- Theme-aware surfaces: shell, landing cards, Hub graphic, and search dialog
  now use `--bg-primary` / `--glass-*` / `--text-*` so light mode is readable.
- Mobile drawer reports `aria-expanded`, highlights the active route, and
  closes on navigation or Escape. Command palette closes on backdrop click and
  locks body scroll. Header and glass panels honor `prefers-reduced-motion`.
- Added a chrome-palette regression test that forbids cyan utilities in owned
  landing/navigation files.

### Chat / Codex — W6 Pages deployment and cutover (#122) (2026-08-13) [DRAFT]

- Replaced the MkDocs build in `.github/workflows/docs.yml` with the locked
  React website flow: Node 22, `npm ci`, `npm test`, `npm run build`, and the
  `docs/website/dist` GitHub Pages artifact. Pull requests now validate the
  same build without deploying; `main` deployments remain serialized.
- Documented the canonical Markdown/content-build boundary, required local
  verification, production acceptance checks, and a known-good-revision
  rollback path in `docs/DOCUMENTATION_STANDARDS.md` and the website README.
- The public Pages acceptance pass completed successfully, so removed the
  obsolete `docs/mkdocs.yml` and `docs/website/generate_docs_json.py`.
  Recovery remains available through the known-good Git history rather than
  retaining an inactive second documentation build path.
- Verified the exact CI command sequence from a clean Git archive: `npm ci`,
  `npm test`, and `npm run build` all pass with generated content absent at
  checkout and recreated by the `pretest` hook.

### Claude — W2 documentation content pipeline (#118) (2026-08-13) [DRAFT]

- `scripts/build-content.ts` now enumerates the roadmap's exact curated
  corpus (`docs/*.md`, `docs/adr/**`, `docs/moon/ROADMAP.md`,
  `docs/moon/CHANGELOG.md`, `docs/moon/roadmaps/*.md`) explicitly, instead
  of walking all of `docs/` — archive/research/reports are excluded by
  construction, not by convention.
- Parses optional YAML frontmatter (`gray-matter`): `title`, `description`,
  `nav_group`, `order`, `draft` — each overriding the existing
  path-heuristic default. Draft pages fail the build, with every offending
  file listed.
- Generates GitHub-style heading anchors (`github-slugger`), replacing a
  hand-rolled regex — correct duplicate-heading disambiguation
  (`overview`, `overview-1`, ...).
- Validates and rewrites internal Markdown links directly in the stored
  page content: an in-corpus `.md` link becomes a `/#/docs/<slug>`
  HashRouter path (validating any heading anchor along with it); a link to
  a real file outside the curated corpus is recorded in a new
  `manifest.unpublishedLinks` array rather than failing the build; a link
  to nothing real fails the build with every offender listed.
- Fixed the slug format to match the roadmap's own locked example
  (`moon/roadmaps/ui`, not `moon-roadmaps-ui`) — required `DocsLayout.tsx`
  to read the `/docs/*` splat param for multi-segment slugs.
- Running this for real against the actual `docs/` tree caught and fixed
  genuinely broken pre-existing links: `ARCHITECTURE.md`/`SECURITY.md`/
  `TROUBLESHOOTING.md` linked `ROADMAP.md` instead of `moon/ROADMAP.md`;
  `TUTORIAL.md` linked `android/README.md` instead of
  `../android/README.md`; `SECURITY.md`'s license badge linked `LICENSE`
  instead of `../LICENSE`.
- `src/content/*.json` were accidentally committed instead of gitignored
  as the roadmap requires — untracked them and added them to
  `docs/website/.gitignore`.
- 15/15 tests (`tsx --test tests/*.test.ts`, matching the `node:test`
  convention already established by `tests/search-rank.test.ts`).
  `npx tsc --noEmit` and `npm run build` (prebuild → build-content →
  Vite) both clean against the real 26-document corpus.
- Known limitation: the link scanner is a single-pass regex, not a full
  Markdown AST — nested `[![badge](img)](target)` links (image-in-link,
  used for README/status badges) are not currently validated or rewritten;
  they're silently left as-is rather than misclassified. A future pass
  could move to a proper `remark`/`unist` walk if that coverage matters.

### Chat / Codex — W1 documentation website foundation (#117) (2026-08-13) [DRAFT]

- Replaced the website's Vue entrypoints and configuration with an isolated
  React 19 + TypeScript + Vite + `HashRouter` foundation, keeping the desktop
  app package boundary intact.
- Configured Tailwind with the locked desktop design tokens: slate field
  `#020617`, indigo `#6366f1`, purple `#a855f7`, 16px glass cards, and 20px
  blur; the superseded cyan palette is not part of the foundation contract.
- Added local `@fontsource` Inter and JetBrains Mono bundles plus an inline
  before-paint theme initializer. The website no longer requests Google Fonts.
- Replaced the legacy Vue README/configuration/assets with React website
  guidance. `npm run build` now completes the content build, type check, and
  static Vite build successfully.

### Grok — W4 landing + W5 search/theme (#120, #121) (2026-08-13) [DRAFT]

- **Product landing (`/#/`).** Product-forward hero, capability grid, v1
  workflow, local quick-start snippet, docs + GitHub CTAs. Abstract Hub
  graphic (Grok / Claude / Codex / Gemini satellites) — not a desktop
  screenshot. Interlocking-circles brand mark recolored to indigo `#6366f1`
  / purple `#a855f7`.
- **Offline search.** `Cmd+K` / `Ctrl+K` MiniSearch palette ranks title
  above summary/body, supports arrow/Enter, and reads the W2
  `search-index.json` artifact. No external search service.
- **Zero-flash theme.** Dark default; Dark / Light / System controls persist
  to `ca-website-theme`. Inline boot script + `ThemeProvider` apply the
  resolved class before paint. Self-hosted Inter / JetBrains Mono via
  `@fontsource` (no Google Fonts).

### Gemini — Documentation Website Foundation, Content Pipeline & Reader (#117, #118, #119) (2026-08-13) [DRAFT]

- **React 19 Website Foundation (W1 / #117)**: Migrated `docs/website` from Vue prototype to React 19 + TypeScript + Vite + Tailwind CSS + HashRouter (`/#/docs/...`). Configured the locked slate/indigo/purple glassmorphism tokens and self-hosted Inter & JetBrains Mono typography.
- **Markdown Content Pipeline (W2 / #118)**: Built `scripts/build-content.ts` for the curated canonical corpus, producing `docs-manifest.json` and `search-index.json` for 26 published documents across 4 categories.
- **Accessible Documentation Reader (W3 / #119)**: Built `DocsLayout`, `DocsSidebar` (grouped categories & active route highlight), `MarkdownArticle` (`react-markdown`, PrismJS syntax highlighting, stable code-copy feedback, Mermaid.js rendering with graceful fallback, and manifest-driven unpublished-link notices), `TableOfContents` (scroll-aware heading tracker), `PrevNextNav` (document pagination), and `CommandPalette` (`Cmd+K` / `Ctrl+K` search modal).

### Gemini — Documentation Website Roadmap Approval & Architecture Lock (2026-08-13) [DRAFT]

- **Approved Documentation Roadmap (`docs/moon/roadmaps/documentation.md`)**: Promoted roadmap status from Draft to Approved. Locked visual aesthetics to glassmorphism (indigo `#6366f1` & purple `#a855f7` accents, Inter & JetBrains Mono typography), content pipeline to Node/TypeScript script (`scripts/build-content.ts`), routing strategy to `HashRouter` for GitHub Pages subpath stability, and interactive feature scope (Product Landing Page, `Cmd+K` Command Palette, scroll-aware TOC, Code Copy, Mermaid.js).

### Changed

- **Hub/CLI source layout.** Split the Hub store and `ca` CLI implementation
  into responsibility-focused Rust modules, including independently compiled
  Hub test groups. Every Rust source file in `crates/hub/src` and
  `crates/cli/src` is now at or below 500 lines; the public `hub` exports and
  the installed `ca` command interface remain unchanged.
- **Frontend panel layout.** Split `App`, Config, Hub, and chat-panel code
  into focused React components and support modules; placed Hub under the
  panel directory, kept `TaskTab` as a general component, and grouped chat
  support under `panels/messager`. Every TypeScript/React source file in
  `src/` is now at or below 500 lines.
- **Messager naming.** Renamed the app's former chat-branded panel,
  navigation state, support modules, storage key, and first-party
  documentation to Messager/messager.

### Added

- **DeepSeek through OpenCode.** Orchestrate provider/model selection includes
  DeepSeek. Model IDs come from `opencode models` (`deepseek/*`); the run path
  is `opencode run <prompt> -m deepseek/<model> --dir <abs>`. Missing
  `opencode` returns a clear unavailable error. No API keys are hardcoded or
  read from OpenCode config.
- **Mistral through Vibe.** Orchestrate includes Mistral (Vibe). Programmatic
  runs use explicit argv (`vibe -p <prompt> --workdir <abs> --trust --output
  text --auto-approve`) after `vibe --help` confirms those flags. Missing
  install, unsupported vibe builds, and missing auth (`MISTRAL_API_KEY` or
  `~/.vibe/.env` presence only; `vibe --setup`) return unavailable. Selected
  model is passed as `VIBE_ACTIVE_MODEL` (vibe has no `--model` flag). Wake
  inject may spawn `opencode`/`vibe`; task-only inject still queues.
- **Tauri backend layout.** Reorganized `src-tauri/src` by responsibility:
  `agent/` contains orchestration, `client/` external model clients,
  `harness/` capture and delivery adapters, `hub/` desktop Hub commands,
  `server/` local TCP services, `core/` filesystem/process utilities, and
  `main/` the binary entry point. Module names and Tauri IPC command names are
  unchanged.
- **Hub command decomposition.** Split the former 1,875-line Tauri Hub
  command module into bounded store, memory, messaging, workflow, quota, and
  test modules (all at or below 500 lines). The registered IPC command names
  and their JSON payloads are unchanged.
- **C12-CLAUDE-BRIDGE.** Real, verified discovery of already-running Claude
  Code sessions for task delivery (`crates/hub/src/claude_bridge.rs`),
  wired into `inject_harness`. `claude agents --json` (documented in
  `claude --help`) lists every active interactive/background session with
  its pid, cwd, session id, and status — confirmed directly on a live
  machine: it lists the very session this code was written in. Each active
  session also listens on a real Unix socket at
  `$XDG_RUNTIME_DIR/cc-socks/<pid>.sock`, confirmed with `lsof -U` against
  that live pid. Unlike Codex's `app-server`, that socket's wire protocol
  is undocumented Claude Code internals, so this bridge does **not**
  attempt to connect to it — writing arbitrary bytes into a live
  interactive session's control channel with no documented protocol and no
  way to verify a safe outcome is not a responsible automated action.
  Delivery always resolves to a clearly explained `unavailable` (task
  stays queued), the same safety shape as a missing bridge socket, except
  every claim behind it is grounded in something actually observed rather
  than assumed. Never spawns a replacement `claude -p` process or writes
  to a PTY.
- **C12 bridge review.** Gemini/Antigravity capture remains available, but its
  published CLI has no supported active-session IPC/RPC transport. The former
  assumed bridge-socket implementation was removed: task delivery now reports
  `unavailable` and stays queued rather than claiming delivery through an
  undocumented interface. Claude likewise performs documented active-session
  discovery but safely leaves tasks queued because its live socket protocol is
  undocumented. Grok retains the provider-supported ACP leader path.
- **Crate names simplified.** Workspace packages/directories are now `hub`
  and `cli` (the CLI binary remains `ca`); Rust imports, build commands, and
  project documentation use the new names.
- **C12-GROK-BRIDGE.** An explicitly registered Grok session can receive a
  queued Hub **task** through Grok Build's documented ACP leader path
  (`grok agent --leader stdio` → `session/load` + `session/prompt` on
  `~/.grok/leader.sock`). That is an ACP client of the existing leader, not
  a replacement TUI. If the leader socket is missing, delivery is
  `unavailable` and the task stays queued. Register with
  `hub_register_harness_session` (or let Grok capture auto-register the
  latest on-disk session). `hub_inject_harness` / `ca msg tag --dispatch`
  use this path for Grok tasks.

- The app header now continuously shows the selected **Workspace root** and
  **Active team chat**. Orchestrate places the editable Workspace Root control
  at the top, before team/session configuration.
- Chat & Memory can **create and delete** durable sidebar channels. Built-in
  `#general`, `#team-coordination`, `#agent-memory`, and `#wakes-alerts`
  stay; custom names persist in the hub store and can be removed from the
  sidebar (messages remain). Tracked as U13.

- **V1 hub-native orchestration (U11–U12, C10–C12), foundation delivered.**
  Orchestrate can **Create** or **Load** a named team chat and focus Chat &
  Memory on that session (`localStorage` keeps the choice across hub polls).
  The composer addresses all / a subset / one session member and can mark
  posts **task** and/or **wake**. Task-tagged sends to non-members are
  refused; wake-tagged sends may enroll a new identity. Tagged delivery
  goes through `hub_send_tagged_message` / `ca msg tag` and, when accepted,
  `hub_inject_harness` or `ca msg tag --dispatch` (explicit argv, no shell,
  no TUI attach). Active-session execution remains gated on provider-supported
  bridge transports.
- **C12 four-harness capture.** Disk adapters record assistant text from
  Grok (`~/.grok/sessions/<pct-workspace>/<id>/chat_history.jsonl`), Claude
  Code (`~/.claude/projects/...jsonl`), Codex (`~/.codex/sessions/...`), and
  Gemini/Antigravity (`transcript.jsonl`). Each adapter keeps a **disk
  session id** separate from the **hub work-session id**. Duplicate polls
  are content-hash deduplicated. Chat & Memory refresh polls all four and
  reloads the transcript when new rows appear. Headless: `ca harness capture
  --harness grok|claude|chat|gemini --workspace PATH`.
- Live named-session check on this checkout (throwaway HubStore, no write
  to `~/.coding-assistants`, no spawn): task-tagged send to enrolled `grok`
  accepted, outsider rejected; disk capture found grok 11 / claude 52 /
  chat 25 / gemini 247 assistant rows. C13 is Harbinger running that loop
  in the app; the markdown bus stays as fallback until then.
- C13 migration gate is specified (C12 live acceptance, named-session
  assign/review/capture, reconstruct from hub records only, #113 evidence).
- `infra/supabase/supabase_config.js` scaffold for later S11 Auth + Storage
  (no sync implementation).
- Website docs data (`docs/website/src/data/docs.json`) regenerated from
  the moon roadmaps (U11–U12 / C10–C13).

### Fixed

- **Orchestrate continuity.** The selected Workspace Root is now persisted in
  browser storage, and the Orchestrate roster rehydrates persisted Hub team
  membership after an app restart. **Detect running agents** is now a visible
  toggle: after scanning it becomes purple **Hide detected agents** and hides
  the discovery panel when clicked again. Process detection explicitly
  distinguishes finding a local process from having a supported live-delivery
  bridge.
- **Tagged task delivery and Gemini adapter:** task-only posts no longer
  launch an unexpected replacement Grok/Chat/Claude/Gemini CLI process; they
  remain in the durable session inbox and explicitly report that an active
  harness adapter must consume them. Explicit wake posts retain spawn
  behavior. Gemini/Antigravity now launches the installed `agy` executable,
  rather than the nonexistent `gemini` binary.
- **Explicit team enrollment:** new hubs now start with the human owner only.
  An untouched legacy default roster is migrated the same way, so agents must
  be deliberately added to the team/session before task delivery is accepted.
- **C12 partial injection failure visibility:** Chat & Memory now waits for
  every selected harness injection and reports each rejected/unavailable
  delivery in the existing owner alert. A durable session post is not hidden
  behind one rejected IPC call.
- **C10–C12 Tauri session-send payloads:** nested tagged and ordinary session
  arguments now deserialize their Chat & Memory camelCase fields, preventing a
  tagged send from failing with a missing `is_task` field before dispatch.

### Gemini — v1 hub-native orchestration UI & recipient tag controls (2026-08-13)

- **U11 Create and Load Team Chat Entry Points**: Added dedicated Create Team Chat and Load Existing Team Chat controls to Orchestrate (`ConfigPanel.tsx`), which automatically set active work session and focus the Chat & Memory window (`App.tsx`).
- **U12 / C10 Recipient Selection & Intent Tags**: Added Recipient Mode controls to Chat & Memory composer (`MessagerPanel.tsx`) supporting `🌐 All Team Members`, `👥 Subset` (interactive agent checkboxes), and `🎯 Single Agent` (dropdown), along with `⚡ [TASK]` and `🔔 WAKE` intent tag toggles.
- **C11 Task Tag Team Member Validation**: Implemented pre-flight validation preventing task-tagged messages from targeting non-enrolled agents, ensuring tasks target existing team members while wake-tagged messages can trigger or spawn new agent instances.
- **Transcript Intent & Recipient Badges**: Added visual badges to transcript message bubbles displaying `⚡ TASK`, `🔔 WAKE`, and `To: <recipient>` header metadata.

### Chat / Codex — v1 orchestration work-review draft (2026-08-13)

- Reviewed Grok's U11–U12 / C10–C13 hub-native orchestration plan and filed
  the six implementation issues (#108–#113), each linked to its roadmap row
  and the v1 gate. Implementation of U11–C12 has since landed; C13 is the
  owner live loop.

### Grok — v1 hub-native orchestration spec (2026-08-13)

Specified the remaining work to run the whole team from the CA app instead
of per-repo `.agent` markdown. New roadmap slices:

- **U11** Orchestrate **Create** and **Load** team chat (create already exists)
- **U12** Composer: all / subset / one, plus optional task and wake tags
- **C10** Same addressing from human and enrolled agents
- **C11** Wake may spawn a new instance that joins the team; task must target
  an existing member
- **C12** Capture harness-side messages and inject tagged hub messages
- **C13** Retire `AGENT_BUS.md` as the live protocol once C10–C12 work

Chat files the GitHub issues. Implementation order is U11, then C10+U12,
then C11, then C12, then C13.

### Claude session (2026-08-13)

- Usage tab no longer labels every successful provider quota fetch as **live
  quota**. Only Codex (`chat`, via `codex app-server` rate limits) and Grok
  (`grok`, via its billing snapshot) genuinely re-query a live process/API on
  every call, so they keep the "live quota" badge. Every other provider
  (Claude, Gemini/Antigravity, and any future harness lacking an official
  usage-budget command) now shows **last refreshed `<date-time>`** derived
  from `ProviderQuota.fetched_at`, plus a per-provider **Refresh** button, so
  the displayed numbers don't silently go stale between window opens.
- Added a **Refresh all stale quotas** button that re-fetches every non-live
  provider (everything except `chat`/`grok`) in one action.
- New backend command `hub_refresh_provider_quota(agent_id)` dispatches to
  the matching per-provider adapter so the frontend can refresh one provider
  without re-fetching the rest.
- **Disclosure**: while wiring this up, found `gemini_quota()` currently
  returns **hardcoded/fabricated window data** (fixed 66%/0%/100%/100% used
  percentages) — only its reset countdowns are computed relative to "now". It
  is not a one-time-stale snapshot, it was never live at all. The refresh
  button now at least lets you re-fetch it, but a genuine reverse-engineered
  Antigravity CLI usage-budget adapter (mirroring the Claude Code work below)
  remains open work, tracked under #86.

### Gemini session (2026-08-13)

- Added support for **Google Antigravity CLI** usage limit plots in Shared Hub (`Usage` tab) with dedicated sub-groups for **Gemini Model Family** (weekly limit & 5-hour limit remaining) and **Other Model Families** (Claude & GPT models in Antigravity).
- Expanded provider quota charts to group and display harness titles (`Anthropic Claude Code`, `xAI Grok Build`, `OpenAI Codex`, `Google Antigravity CLI`, `Anomaly Opencode`, `Local Llama.cpp`, `Local Ollama`) with model family subtitles (`Claude Model Family`, `Grok Model Family`, `Chat Model Family`, `Gemini Model Family`, `Other Model Families`).
- Endorsed `cloud_sync.md` architecture design with zero-trust local-first encrypted replica model, mutation-only Hub lock during sync runs, and 30-day manual conflict retention.

### Shared Hub consolidation (2026-08-13)

- Renamed the desktop conversation surface to **Chat & Memory**. It is now the
  single user-facing location for team messages, agentic memory, and
  `#wakes-alerts`; the duplicate Shared Hub **Inbox**, **Memory**, and
  **Wakes** navigation entries were retired.
- Wake-policy controls now apply their selected value optimistically while the
  persisted policy update is in flight, reverting only on an error. Their
  custom checkbox treatment uses a bright checked state, tick, border, and
  focus halo so selected and unselected policies are plainly distinguishable.

### Grok session (2026-08-12 → 2026-08-13)

Lead-orchestrator pass on the Messager-like hub after Harbinger's GO (M6 first,
then prove the team loop). Commits authored or co-authored in this stretch:

| Commit | What |
| --- | --- |
| `525f07c` | Persisted `agents.team_member` roster; team send includes Harbinger and excludes PID identities |
| `9655e7d` | Messager/Orchestrate team send wakes every enrolled member, not only `chat` |
| `c92accf` | `HubStore::request_team_wakes` |
| `f16e862` | Messager thread no longer creeps down while reading older messages |
| `0dc2f1b` | `ca agent enroll\|unenroll\|team`, `hub_set_team_member`, header **Local hub online** pill |
| `947a43d` | Enter-to-send, Shift+Enter newline, **Jump to latest** |
| `2ab31c7` | Messager DMs send only to that agent |
| `f9e255b` | Shared Hub Usage plots Grok's weekly pool (`creditUsagePercent`) and extra-usage credits from the TUI `/usage` billing snapshot |

Delegated (not claimed as Grok implementation): CA-106/109/110/111 to Claude,
CA-102 channel queries to Chat. M6 live seed is in `~/.coding-assistants`
(`ca memory search M6-20260812`); Claude ACKed. Board: #82 and #80 closed;
U10 follow-through tracked as #90 (closed after CA-106/109/110/111). Grok
Messager spine commits for #90: `525f07c`, `9655e7d`, `c92accf`, `f16e862`,
`0dc2f1b`, `947a43d`, `2ab31c7`. Wakes increment: #81 (still open).

**Cloud sync (2026-08-13):** Grok wrote the first `cloud_sync.md` draft from
the owner Q&A; Claude's second Q&A and `743000a` finalized it as the
approved S1–S13 plan. Grok review: agree, with S6 rebase-test and S10
no-key-envelope caveats recorded in that file. Issues #91–#103.

**Claude, 2026-08-13:** #82 and #80 are now closed. CA-106/109/110/111
(`2064a59`, `09d3533`, `bec7454`, `ca40e46`) shipped and are tracked/closed
as issue #90, since U10 had no prior issue of its own.

**U8 quota (2026-08-13):** Shared Hub Usage now plots live provider quota
windows for Codex, Claude, Grok, Gemini/Antigravity, and the remaining
configured harness families. This completes the agreed usage-limit scope for
#86.

### Fixed

- Messager channel badges now count **unread** posts only, using the same
  membership as the thread (including unprefixed `#general` history).
  Opening a channel marks those posts read. Existing history is not treated
  as unread on first launch, which is why `#general` had no number while
  `#team-coordination` showed a stale total.

- CA-106's Edit/Delete menu only opened via right-click, with no visible
  affordance a first-time owner would discover. Added a hover-revealed
  **⋯** actions button on the owner's own message bubbles (opens the same
  menu as right-click, at the click point); the bubble the open menu targets
  gets a highlight ring. Also fixed opening a *new* message's menu while
  another's was already open immediately self-closing — the click bubbled to
  the still-mounted `window` listener left over from the previous menu.
  `e.stopPropagation()` in `openMessageMenu` fixes both the new button and
  the pre-existing right-click path.

- Messager **Direct Messages** now send only to that agent. Opening a DM no
  longer keeps "Broadcast to Team" as the recipient, so a private thread
  cannot fan out to the whole roster.

- Messager composer sends on **Enter**; **Shift+Enter** inserts a newline.
  While reading older messages, a **Jump to latest** chip appears instead of
  yanking the viewport.

- Header chrome no longer shows a second Messager-looking control. The purple
  **Messager Multi-Agent Hub** badge is now a green **Local hub online** status
  pill so it cannot be mistaken for another Messager tab.

- Simplified the **Orchestrate** window to team/role configuration (including
  workspace and MCP settings) plus Remote Control. Its duplicate composer,
  Team Chat, and Messages feed were removed; Messager Chat & Memory is now the
  single desktop surface for human/agent communication.

- Messager `#general` no longer creeps downward while Harbinger reads older
  messages. The 1.5s hub poll was calling `scrollIntoView({ behavior:
  "smooth" })` on every refresh. The thread now stays put unless the view
  is already near the bottom, the channel changed, or Harbinger sent a
  message. Team fan-out copies of one post are shown once.

- Messager/Orchestrate team sends now wake every persisted roster member
  (`hub_list_agents` + `hub_request_wake`) instead of only `chat`. Direct
  messages still wake the selected recipient. The Messager DM list follows
  `team_member` enrollment and role labels match the Grok-lead / Chat-co-lead
  split.

- Team broadcasts (`ca msg send --to team` and `hub_send_message` with
  `to: "team"`) now fan out only to agents with persisted `team_member = 1`.
  The default roster is Harbinger (`human`) plus `claude`, `chat`, `gemini`,
  and `grok`. Process-discovered PID identities and local model runtimes stay
  privately addressable. `cargo test -p hub` (12 passed).

- Fixed the Messager Chat window going blank a few hundred milliseconds after
  first paint. `MessagerPanel.tsx` declared its own `DetectedProcess` shape
  (`agent_id`, `executable`) that never matched what `detect_agent_processes`
  actually serializes (`agent`, `provider`, `model`, `command`, `pid`) —
  `ConfigPanel.tsx` already had the correct shape. As soon as the 4s presence
  poll found a real running agent process, `p.agent_id.toLowerCase()` threw
  on `undefined` inside `getAgentInfo` (called during render for every
  message and the DM roster), and with no error boundary React 18 unmounted
  the whole tree. Aligned the interface and the running-process match logic
  with the real backend shape.

### Added

- Claude Code now reports **live session/weekly/monthly-credit quota
  windows** in the Usage tab, matching Chat's Codex quota bar. Anthropic
  publishes no stable API for a subscription's usage-limit percentages
  (`anthropic-ratelimit-*` headers are a separate, per-API-key billing
  concept); found the endpoint the official `claude` CLI itself calls to
  render `/usage` by driving an interactive `claude --debug` session and
  reading its debug log, then verified directly against it —
  `GET api.anthropic.com/api/oauth/usage` (Bearer-authenticated with the
  OAuth token from `~/.claude/.credentials.json`). It is undocumented and can
  change on any Claude Code update with no notice; every failure path
  (not logged in, expired token, network error, unrecognized response
  shape) degrades to the existing "unavailable" state rather than
  crashing. The "Usage credits" monthly window has no `resets_at` in the
  response, so its reset date is computed locally as the 1st of next
  month UTC, matching the desktop `/usage` UI's own wording.

- Work sessions: Orchestrate can create a named durable work-session chat.
  It starts with the current persisted team and adding an eligible agent to the
  team enrolls it in the active work session. Messager Chat & Memory lists each
  session, renders human and agent-harness messages in its
  `channel:session:<id>` scope, and sends only to that session's members.
  Per-member checkboxes decide which offline agents receive a wake for the
  next session message without changing durable delivery.

- Cloud Drive sync capability roadmap (`docs/moon/roadmaps/cloud_sync.md`,
  `743000a`): Google Drive first, then Firebase Auth+Storage, then Supabase,
  then OneDrive/Dropbox. Encrypted replica, journal-integrity merge, hashed
  remote names, confirm-only deletes, CLI `ca sync`, Hub mutation lock.
  Implementation issues S1–S13 (#91–#103).

- CA-114: channel messages can now be replied to in context. Replies retain a
  stable root message in the existing `channel:<name>:thread:<root>:<id>`
  subject namespace, so they remain isolated to the channel and keep normal
  team fan-out/wake behavior. The Messager composer shows the selected parent
  with a cancel control, while rendered replies identify their parent without
  requiring a schema migration.

- CA-106: right-click **Edit** / **Delete** on Messager message bubbles
  (`MessagerPanel.tsx`). Only Harbinger's own posts (`from_agent ===
  "human"`) show the menu; `hub_update_message` / `hub_delete_message`
  enforce the same rule server-side. Team/channel broadcasts are N SQLite
  rows sharing a subject, so both commands resolve and mutate every sibling
  copy via `hub::update_broadcast` / `delete_broadcast` — new posts group
  by the exact `channel:<name>:<uuid>` subject, legacy posts fall back to
  `(from_agent, body, subject, created-at-to-the-second)`. Delete is a soft
  cancel (`status = cancelled`); the Messager view hides cancelled rows while
  the audit trail (`hub_list_messages`) still returns them. `cargo test -p
  hub` (15 passed).

- CA-109: CLI parity for CA-106 — `ca msg edit --id <uuid> --from human
  "body"` and `ca msg delete --id <uuid> --from human`. Rejects any `--from`
  other than `human`, and independently verifies the target message's
  `from_agent` is `human` before mutating, matching the desktop
  `require_human_authored` check. Both commands resolve every sibling copy
  of a team/channel broadcast via `update_broadcast`/`delete_broadcast`.

- CA-110: Orchestrate role/process cards and the detected-process list now
  show **Remove from team** once enrolled, calling `hub_set_team_member`
  with `enrolled: false` for the same stable ids (`chat`, `claude`,
  `gemini`, `grok`) `Add to team` persists. `human` is never unenrolled, and
  removal never invents a PID-based roster row.

- CA-111: added a desktop **Journal** tab (`HubPanel.tsx`) surfacing pending
  audit events (`ca audit watch` output) at the owner checkpoint — fetched on
  first load (tab badge shows the pending count) and every time the tab
  opens, with **Approve** / **Quarantine** actions. New Tauri commands
  `hub_list_audit_events`, `hub_approve_audit`, `hub_quarantine_audit` wrap
  the existing `hub::HubStore` audit API; no new privileged adapter.

- Persisted Orchestrate **Add to team** onto the Messager roster for stable
  harness ids (`chat`, `claude`, `gemini`, `grok`). CLI: `ca agent team`,
  `ca agent enroll --id`, `ca agent unenroll --id`. Tauri:
  `hub_set_team_member`, `hub_list_team_members`, `hub_request_team_wakes`.

- Added CA-102 channel and memory-reference queries across the shared Hub:
  `ca msg channel <name> [--limit N]`, `ca msg memories <message-id>`, and
  Tauri `hub_list_channel_messages` / `hub_list_message_memories`. Channel
  lookup is exact (`channel:<name>`, with colon-delimited thread metadata)
  and bounded; `[Memory #<full-id-or-unique-prefix>]` tags resolve only when
  they identify one durable memory, preventing ambiguous cross-references.
  Store and Tauri command coverage verifies both channel isolation and linked
  memory resolution.

- Added dedicated **Messager-like Multi-Agent Chat Interface & Agentic Memory Hub** (`MessagerPanel.tsx`). Features channel sidebar (`#general`, `#team-coordination`, `#agent-memory`, `#wakes-alerts`, DM channels), agent status indicators, real-time message stream with Messager formatting, and an expandable Agentic Memory Hub side drawer.
- Established Lead Orchestration task allocation across Gemini (Lead Orchestrator), Grok (Build), Chat/Codex (Chat), and Claude (Code) on `.agent/cache/AGENT_BUS.md` and per-agent delegation files in `.agent/messages/`.

- Private hub messages addressed to Harbinger now display their contents in
  the Messages feed; private messages addressed to another participant remain
  redacted in the shared team view.

- Fixed Orchestrate message delivery to use stable harness identities (`chat`
  for Codex/ChatGPT) instead of detected process IDs, and added Team/private
  recipient routing. Each sent message now also creates a linked hub wake so
  the receiving harness has an explicit signal to poll its inbox. Team
  broadcasts fan out with a shared marker; private messages remain visible as
  sender/recipient metadata without their bodies in the team chat.

- Centralized Tauri invocation behind a runtime guard. Browser/Vite mode now
  reports a clear desktop-runtime requirement instead of throwing an undefined
  bridge error, and Tauri event listeners are skipped outside the desktop app.

### Added

- Shared Hub Usage now plots Grok's weekly subscription pool the same way
  it plots Codex windows. The adapter reads the Grok CLI session token from
  `~/.grok/auth.json` (never logged) and fetches the same
  `GET /v1/billing?format=credits` snapshot the TUI `/usage` command uses,
  then maps `creditUsagePercent` / `billingPeriodEnd` onto a Weekly bar.
  Extra usage credits (`onDemandUsed` / `onDemandCap`) appear as a second
  bar when present. Quota rows are limited to the four harnesses so PID
  identities no longer clutter the chart.

- Added provider-quota plots to the Shared Hub Usage tab. Codex is queried
  through its local app-server account rate-limit endpoint and displays each
  reported window's used/remaining percentage and reset time; providers that
  do not expose a local quota snapshot are explicitly marked unavailable
  instead of being confused with local Shared Hub budgets.

- Added the first agent-session bridge boundary: `ca inbox watch --agent
  <id>` emits JSONL `ready`/`message` records, polls the durable inbox,
  acknowledges delivered messages, and resolves linked wakes. Human-gated
  wakes require explicit `--accept-gated`; `--forward PROGRAM` can pipe the
  same stream to a long-lived provider adapter. Added a Codex adapter using
  the installed app-server `thread/resume` + `turn/start` protocol rather than
  terminal injection, plus `--list-threads` discovery for selecting a
  persisted session. Completed assistant messages are now published back to
  the hub as replies from `chat`; attaching to an already-running interactive
  TUI remains intentionally unsupported because it has no exposed app-server
  control channel.

- Connected Orchestrate to the shared harness inbox: messages sent by Harbinger
  use `hub_send_message` directly instead of launching an OpenCode task, while
  persisted agent and private messages are refreshed into the chat with sender,
  recipient, kind, and status labels.

- Reframed Orchestrate as a session team chat: **Execute Task** is now
  **Write Message**, **Launch Sequence** is **Send Message**, agent events are
  displayed as sender-attributed messages, and configured spawned agents have
  explicit **Add to team** controls. Enrolling a detected existing process now
  immediately adds its participant and a join message to the chat.

- Added the audit integrity MVP to `hub` and `cli`: recursive filesystem
  observation via `ca audit watch`, durable pending change records, owner
  approve/quarantine actions, and SHA-256 chain verification via `ca audit
  verify`. User-space observation records the watcher context and documents
  that originating external-writer PID attribution requires a privileged
  adapter.

- Added per-role existing-process endpoint configuration. When an
  OpenAI-compatible endpoint is supplied, orchestration sends requests to the
  already-running model service and does not spawn or terminate a child
  process; blank endpoints preserve the existing provider-managed behavior.
- Added a **Detect running agents** control to Orchestrate. It discovers local
  Grok, Claude, Codex/ChatGPT, and Gemini/Antigravity command processes and
  lets the user add selected identities to the configured team without taking
  ownership of or terminating those processes.
- Tightened process discovery to match executable basenames only, excluding
  desktop helpers, Chromium/Node utility services, and agent runtime helpers.
- Refined Gemini detection to recognize `agy` and the legacy `gemini` CLI while
  excluding the `antigravity` IDE executable.
- Improved maximized-window scrolling by removing permanent compositor-layer
  promotion from full-page panels and allowing offscreen sections to be skipped.
- Added a large-window performance profile that reduces full-surface gradients,
  card/button shadows, and header backdrop filtering while preserving layout and
  colors.
- Renamed the Shared Hub Budget tab to Usage and added per-agent used/available
  budget charts.
- Made startup resource discovery read-only so the default workspace is not
  created until the user explicitly initializes or runs a task.

- **Dashboard telemetry slice:** added persisted `agent_metrics` counters for
  provider calls, output lines/chars, estimated tokens used, and cached tokens;
  added Shared Hub Dashboard cards with per-agent budget progress. Exact
  provider token/cache/cost/latency reporting remains follow-up work.
- Dashboard now includes a collaboration overview sourced from existing task,
  message, and wake records, including pending-wake counts and recent tasks.

- **C6 done:** `agent_budgets` table + `HubStore::set_agent_budget` /
  `record_budget_usage` / `resume_agent` / `pause_for_budget`. Crossing a
  budget's `limit_units` flips `paused` (caller-defined units — call count,
  USD, tokens, whatever the caller tracks); `pause_for_budget` writes a
  Markdown handoff summary (objective/completed/missing) under
  `markdown/handoffs/`, sends a durable `Handoff` message to the delegate
  (default `"human"`), and `request_wake` then rejects the paused agent until
  a human calls `resume_agent`. Wired through `ca budget
  set|status|spend|pause|resume` and Tauri `hub_set_agent_budget` /
  `hub_get_budget` / `hub_record_budget_usage` / `hub_resume_agent` /
  `hub_pause_for_budget` (desktop UI wiring still open). Covered by
  `c6_budget_exhaustion_pauses_writes_handoff_and_blocks_wakes`.

- **2026-08-11 memory/communication hub slice (M1–M5, C1–C5):**
  - `hub`: promote/compact/delete, purge-stale, age-out short-term; wake
    pending **dedup**; wake resolve; standing `WakePolicy` (human-gate defaults);
    message status updates; Markdown export includes handoffs;
    **`export_markdown_git`** (M3); **`tasks`** with sequential stages,
    bounded-parallel groups (`parallel_group` + `max_parallel`), and
    per-stage **retries** (`max_retries` / `retry_task`) (C5).
  - `ca` CLI: `memory promote|delete|compact|purge-stale|age-out`,
    `msg status`, `wake resolve|policy`, `export-markdown --commit`,
    `task create|advance|complete|retry|list|get|cancel`.
  - C6 budget controls: `ca budget set|status|spend|pause|resume`; exhausted
    agents are blocked from new wakes and produce durable Markdown handoffs.
  - Tauri `AgentSystem` now checks configured budgets before provider calls and
    records one call unit after successful completions, invoking the handoff
    boundary when a role exhausts its budget.
  - Active-run cancellation now records a durable shutdown handoff and
    delegation message before the Tauri task exits.
  - Shared Hub now includes a Usage tab for configuring limits, recording
    usage, inspecting paused agents, and resuming them.
  - C4 task-level `require_human_approval` is persisted and exposed through
    CLI/Tauri workflow creation, with coverage for ungated task wakes.
  - Added atomic pre-call budget reservation via `ca budget consume` and
    Tauri `hub_consume_budget`; over-limit provider calls are rejected before
    they start.
  - Tauri `hub_*` IPC + React **Shared Hub** panel; Orchestrate UI split into
    `ConfigPanel`/`ActivityPanel`/`RemotePanel`/`ApprovalPanel`.
  - Shared Hub **Policy** tab added for managing standing `WakePolicy` (human gate defaults);
    Wakes panel resolves pending wakes as delivered.
  - **C7 done:** Implemented A2A-compatible discovery and horizontal delegation. `AgentCard` schema and storage were added to `hub`. `ca agent register-card` was added to `cli`. The Tauri API exposes `hub_upsert_agent_card` and the TCP server now handles `GetAgentCards` payloads, enabling local workflows to interoperate with A2A peers.
  - **U3 done:** Implemented `update_memory` in `hub` store and added inline editing
    along with color-coded scope indicators to the Shared Hub Memory tab.
  - **U2 done:** Added Task Browser tab to Shared Hub, allowing users to view task history,
    metadata, and message/handoff transcripts.
  - **U5 done:** Added DashboardScreen to Android app for viewing events and approving/rejecting wakes via TCP.
  - **U6 done:** Implemented Project Creation Wizard via a `bootstrap_workspace` Tauri command
    and a button in the ConfigPanel to initialize `.agent/` skeletons for new workspaces.
  - **C4 done:** Implemented per-task delegation policies via `require_human_approval` on
    `TaskRecord`, enabling configurability for automatic wakes during task dispatch, accessible
    through both the `cli` (`--require-approval`) and the Tauri API (`CreateTaskArgs`).
  - **C6 done:** Exposed shutdown hooks via `ca shutdown` in the CLI and `hub_record_shutdown`
    in the Tauri API. This completes the budget exhaustion and shutdown delegation milestone,
    allowing external adapters to properly persist handoff states upon cancellation or limit reach.
  - Install: `just install-ca` / `~/.local/bin/ca` documented in `crates/README.md`.
  - Unit tests: promote/compact, wake dedup/policy, M3 git export, M6 handoff
    acceptance, and C5 sequential plus bounded-parallel/retry workflows.
  - **U1 done:** Refactored `App.tsx` into decoupled components (`ConfigPanel`,
    `ActivityPanel`, `RemotePanel`, `ApprovalPanel`) and overhauled the UI with
    a stunning glassmorphism design and micro-animations.
- Added the first executable M6 acceptance flow covering a durable handoff,
  provenance-linked memory, cross-agent inbox retrieval, wake resolution, and
  Markdown export; a real multi-agent repository run remains.
- **M3 done:** `HubStore::export_markdown_git` runs `git add` + `git commit`
  on the Markdown export when its directory is inside a Git work tree; outside
  a repo, with a failed `git add`, or with nothing to commit, it returns a
  `GitExportOutcome { committed: false, detail }` instead of erroring. Wired
  through `ca export-markdown --commit [--message ...]`, the Tauri command
  `hub_export_markdown_git`, and a desktop "Export MD + Commit" button. Covered
  by `m3_export_markdown_git_commits_inside_a_work_tree` (spins up a real
  temporary Git repo).
- PMF VS10 pivot recorded in the agent coordination bus; baseline frontend and
  Rust workspace checks passed before this implementation began.

- Hub spine crates (`crates/hub`, `crates/cli` binary `ca`): SQLite
  agents/memories/messages/wakes, private journals, wake JSON side-channel,
  Markdown export, CLI commands for init/memory/msg/wake/journal/export;
  unit test covering M1/C1–C3 smoke path.

- Replaced language-oriented roadmap files with capability roadmaps for memory,
  communication, UI, dashboards, platform, and infrastructure. Added a Mermaid
  Gantt index, made private agent journals part of the first memory milestone,
  retained LAN and Firebase prototyping, promoted A2A to the next major
  milestone, and removed obsolete deployment scaffolding. Deleted the duplicate
  `docs/ROADMAP.md` and the superseded per-language roadmap files.

- Reoriented the roadmap around the owner-confirmed product identity: a
  personal, local-first collaboration hub. Added the priority
  the initial shared memory and coordination roadmap for
  SQLite/Markdown hybrid memory, durable handoffs, CLI access, wake signals,
  configurable policies, and external-agent adapters. Folded the former root
  feature checklist into the moon roadmaps, kept `docs/ROADMAP.md` as a pointer
  stub at that stage, demoted TUI/3D/GraphQL-first/early actors to
  Someday/Maybe, and
  recorded provider, security, testing, and infrastructure-hygiene follow-up.

- Roadmap implementation, batch 2 (the former daemon-extraction spike):
  completed the daemon-extraction spike as [ADR 0003](../adr/0003-daemon-extraction-spike.md).
  Measured the actual `tauri::AppHandle` coupling across the backend
  (`file_tools.rs`: none; `agents.rs`/`llm_client.rs`: event emission only;
  `tcp_server.rs`: event emission + listening) and decided against
  extracting a separate daemon crate yet — recommends decoupling event
  emission into an internal broadcast channel first (tracked as new item
  `P1`), deferring the physical crate split until the API boundary is clear.
- Roadmap implementation, batch 1 (rate limiting, async file I/O, and shell
  audit): per-provider token-bucket rate limiting on
  outbound LLM calls (`governor`, burst 3 / 1 per second) in
  `llm_client.rs`; converted `FileTools` and the remaining Tauri-command
  file I/O to `tokio::fs` so it no longer blocks async worker threads;
  audited the tool layer for raw shell execution (none found — confirmed
  compliant with `AGENTS.md`'s Security Notes). Also removed `lib.rs`'s
  dead `start_remote_server` command (superseded by `TcpServer`, and it
  hardcoded a machine-specific absolute path that doesn't exist in this
  repo).

- Synced repo scaffolding/tooling from the Tauri-App-Template layout (excluding
  its backend/frontend/middleware/notebooks directories, which don't apply to
  this repo's existing `src/`/`src-tauri/`/`android/` layout): `.agent/`
  cross-agent delegation docs, `.devcontainer/`, `.forgejo/`/`.gitea/`/`.gitlab/`
  CI mirrors, expanded `.github/` automation, `git/` repo-process tooling
  (hooks, backlog sync, label taxonomy), the original `infra/` scaffolding,
  `tools/*/justfile` + root `justfile`, `settings/` editor configs, and
  `docs/` additions (ADRs, `docs/moon/`, Structurizr C4 model, `docs/website/`).
- Moved root-level docs (`ARCHITECTURE.md`, `DEPENDENCIES.md`, `DEVELOPMENT.md`,
  `TESTING.md`, `TROUBLESHOOTING.md`, `TUTORIAL.md`, `ROADMAP.md`,
  `SECURITY.md`, `CHANGELOG.md`) into `docs/`.
- Moved `codecov.yaml`/`CONTRIBUTING.md` from `.github/` into the new `git/`
  directory.
- Expanded the former per-area roadmaps with target-architecture work items
  synthesized from
  `docs/moon/research/Multi-Agent AI App Architecture.md` and
  `docs/moon/reports/AI Coding Tools Feature Report.md`: a headless Tokio
  actor-model daemon, a GraphQL-over-WebSockets API, MCP + A2A protocol
  support, rate limiting + affine-typed budget guardrails, two-tier
  persistent memory, human-in-the-loop security gates, a 2D telemetry
  dashboard, 3D force-graph visualization, and a new Ratatui TUI
  (now reorganized under the capability roadmaps). Added a capability-order
  index and Mermaid Gantt to `docs/moon/ROADMAP.md`.

## [0.1.0] — 2026-07-30

### Added

- Repository created from scratch.
