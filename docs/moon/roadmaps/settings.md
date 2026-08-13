# Persistent Settings Roadmap

> **Status:** Approved implementation plan
> **Date:** 2026-08-13
> **Scope:** local-first desktop settings and durable configuration for Coding-Assistants.

## Product model

Settings opens as one reusable, resizable desktop window. Reopening Settings focuses that window instead of creating a duplicate. It remains independently closable while Orchestrate, Chat & Memory, and Shared Hub stay usable; closing the main application also closes Settings.

The window uses the existing dark glass-morphism language and a WAI-ARIA tab interface.

| Tab | Settings | Scope |
| --- | --- | --- |
| General | theme, window behavior, startup/default view | global; workspace override only where meaningful |
| Workspace & sessions | workspace-open behavior, default team roster, selected/default chat, and workspace overrides | workspace |
| Agents & harnesses | named provider profiles, workspace default profile per harness, executable/workdir, capture polling, inject permission | global profile plus workspace selection |
| Orchestration | task/wake confirmation, auto-enrollment, budgets, tool/sandbox policy | global with workspace, session, and agent overrides in Advanced |
| Memory & storage | retention, non-destructive export, backup policy and number of retained settings backups | global with workspace override where applicable |
| Diagnostics | log level, configuration health, redacted diagnostics export | global |
| ⚠ Danger zone | confirmed reset, removal, and purge operations | explicit target scope only |

Every inherited field displays an **Inherited** or **Workspace Override** status pill. An override can be reset to global in one action. Provider profiles show a non-sensitive source badge such as **Stored in System Keychain** or **Env Var $NAME**; Settings never renders or accepts a raw secret value.

The ordinary controls are theme, startup/session behavior, team defaults, default provider profile, confirmation mode, auto-enrollment, retention, log level, and retained-backup count. Advanced controls hold per-session and per-agent overrides, budgets, tool/sandbox permissions, harness polling and injection permission, and detailed export/backup policy.

## Persistence, scope, and security

Tauri owns a versioned `settings.toml` under `CA_HOME` when set, otherwise `~/.coding-assistants`. The format is TOML and the writer uses `toml_edit` so hand-authored comments and formatting survive app-initiated saves. React only receives typed, JSON-serializable, redacted snapshots through Tauri commands; it never reads the configuration path or file directly.

Writes validate the complete candidate, use a sibling temporary file and atomic replacement, fsync where practical, and retain timestamped last-known-good copies. The default is **three** retained backups, and the user can change that number in Memory & storage within a documented bounded range. A malformed file loads safe defaults without blanking the app, leaves the original untouched, and offers a one-click restore from a selected last-known-good backup.

Global defaults apply first. A workspace override changes only its selected fields. Workspace identity is the user-selected path string normalized for safe storage but deliberately does **not** resolve symlinks, so distinct paths to the same repository can keep separate configuration on one or multiple devices.

Provider profiles are global named records containing non-secret configuration such as provider, model, base URL, and a secret reference. A workspace selects one named default profile per harness; it never copies profile fields. Credentials, OAuth tokens, passwords, raw environment values, and harness session credentials are excluded from TOML, IPC payloads, logs, diagnostics, and exports. OS-specific keychain/secret-service support and its encrypted-vault fallback are a later modular platform decision; the profile reference contract must allow those backends without changing the settings schema.

All configuration changes append to a dedicated, redacted settings audit stream and fan out a compatible redacted event to the Hub audit stream. Executable/workdir settings remain subject to existing validation and explicit-argument/no-shell process rules.

## Orchestration and destructive-action policy

Settings is the sole editor for standing policies; the Shared Hub Policy tab moves into Settings → Orchestration. Orchestrate and Chat & Memory retain only operational controls such as workspace/team selection, Create/Load, and per-message task/wake tags.

Task and wake remain distinct: a task never spawns a harness instance. A wake may auto-enroll any supported harness identity, subject to settings policy; when it starts an instance, it uses that workspace's default profile for the harness. The safe default requires confirmation for wakes, new enrollment, and broadcast delivery. Task delivery to an existing targeted team member can proceed without a standing confirmation unless an override requires one.

Danger-zone controls use red/amber warning badges and high-contrast warning containers. Each confirmation names the exact profile, workspace, or data set affected, describes recoverability, focuses **Cancel** by default, and adds an audit record. Irreversible transcript or memory purges require a second, target-name typing confirmation. The first release exposes retention, non-destructive export, and backup configuration; destructive transcript and memory purges are delivered only through the danger-action slice.

## Delivery slices

| # | Deliverable | Acceptance criteria |
| --- | --- | --- |
| S1 | Versioned settings store | Resolve `CA_HOME`/home safely; validate TOML through `toml_edit`; atomically save; preserve comments; retain three timestamped backups by default with configurable bounded retention; recover from malformed or interrupted writes without a startup failure. | ✅ **Done** · #127 |
| S2 | Typed IPC, effective settings, and audit | Expose redacted get/update/reset commands; deterministically resolve global and path-preserving workspace overrides; write dedicated redacted settings audit entries and Hub-compatible audit events. | ✅ **Done** · #128 |
| S3 | Standalone Settings window | Open/focus one independently closable, resizable settings window; close it with the app; implement the tab interface, inheritance status/reset UI, dark glass styling, and General/Workspace & sessions controls. | ✅ **Done** · #129 |
| S4 | Global profiles and harness settings | Manage global named profiles and per-workspace/per-harness default selection; expose source-status badges but no secret inputs; persist and validate harness executable/workdir, capture-polling, and injection-permission settings. | ✅ **Done** · #130 |
| S5 | Orchestration and storage policy | Relocate standing Policy controls; persist ordinary and Advanced policy scopes, safe confirmation defaults, auto-enrollment, budgets, tool/sandbox settings, retention, non-destructive export, and backup settings; current hub flows honor them. | 🚧 **Partial** · #131 policy is persisted, exposed, and enforced by the live auto-enrollment/export/sandbox paths; Settings-window controls remain |
| S6 | Confirmed dangerous actions | Build the shared red/amber, target-aware confirmation framework with Cancel-first focus and audit events; use typed target confirmation for irreversible transcript/memory purge operations. |
| S7 | Migration and acceptance | Test no-file, legacy/migration, malformed input, interrupted write, permission failure, backup retention/restore, comments preserved, distinct symlink-path overrides, profile defaults, no-secret exposure, window lifecycle, policy enforcement, and destructive-action cancellation. |

## Delivery tracking

- Epic: [#126](https://github.com/ACFHarbinger/Coding-Assistants/issues/126)
- S1: [#127](https://github.com/ACFHarbinger/Coding-Assistants/issues/127) · implemented in `crates/hub` (`SettingsStore`, `default_hub_home`); awaiting Chat/Codex review
- S2: [#128](https://github.com/ACFHarbinger/Coding-Assistants/issues/128) · implemented: workspace-override resolution in `crates/hub::settings`, redacted Tauri commands in `src-tauri/src/hub/commands/settings.rs`, audit fan-out via `HubStore::record_settings_audit_event`; awaiting Chat/Codex review
- S3: [#129](https://github.com/ACFHarbinger/Coding-Assistants/issues/129) · implemented: standalone reusable Tauri window (`src/lib/settingsWindow.ts`, `src/components/settings/SettingsApp.tsx`) with real General (`default_workspace`) and Workspace & sessions (`default_session`) controls, plus a Memory & storage bonus tab (`backup_retention`); awaiting Chat/Codex review
- S4: [#130](https://github.com/ACFHarbinger/Coding-Assistants/issues/130) · storage + typed redacted commands (`settings_list_profiles`, upsert/rename/remove, workspace default-profile, harness list/update); awaiting Chat/Codex review
- S5: [#131](https://github.com/ACFHarbinger/Coding-Assistants/issues/131) · `OrchestrationPolicy`/`OrchestrationOverride`, typed commands, the composed wake-policy editor, and budget commands are implemented, and the three returned enforcement points now consume the persisted policy: `HubStore::send_tagged_message` refuses to auto-enroll a new identity when `auto_enrollment_allowed` is false (an already-known team member can still be added to a session); `hub_export_markdown`/`hub_export_markdown_git` refuse when `export_enabled` is false; `hub_start_harness`/`hub_inject_harness` refuse `vibe` (the one harness that unconditionally passes `--trust`/`--auto-approve`) under a `Strict` sandbox policy. Wiring an Orchestration/Advanced tab into the Settings window remains follow-up UI work; awaiting Chat/Codex review.
- S6: [#132](https://github.com/ACFHarbinger/Coding-Assistants/issues/132)
- S7: [#133](https://github.com/ACFHarbinger/Coding-Assistants/issues/133)

## Dependencies and completion gate

C10–C13 in [`communication.md`](communication.md) own session, task, wake, transcript, and harness-delivery semantics. Settings configures those flows but does not change the task-never-spawns rule. S1 should centralize the existing `CA_HOME`/`~/.coding-assistants` resolution before introducing the new file.

Settings is complete when a user can safely change a global default and a path-distinct workspace override, choose a named profile per workspace/harness, restart without losing the effective configuration, recover from malformed configuration using a retained backup, open/close Settings independently, decline every dangerous action without data change, and verify that no secret value is exposed anywhere in the settings surface or its diagnostics.
