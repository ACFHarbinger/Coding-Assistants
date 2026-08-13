# Infrastructure and Documentation Roadmap

Keep only infrastructure with a current local or prototype use.

| # | Capability | Status |
| --- | --- | --- |
| I1 | `infra/docker/` documentation-site stack | ✅ Done |
| I2 | `infra/terraform/` for future cloud synchronization prototypes | 📋 Pending |
| I3 | `infra/ansible/` for reproducible host setup | 📋 Pending |
| I4 | Firebase and Supabase authentication/private-storage prototypes | 📋 Pending · after Google Drive, Firebase then Supabase implement the shared encrypted-blob and trusted-identity contracts in [`cloud_sync.md`](cloud_sync.md); Drive remains the first replica path |
| I5 | Remove obsolete Kubernetes, Helm, serverless, AWS, Azure Pipelines, WordPress, Webpack, Nginx, and proxy scaffolding | 📋 Pending |
| I6 | Keep research and reports separate from active implementation roadmaps | 📋 Pending |
| I7 | Rename crate/package `tauri-app`/`tauri_app_lib` → `coding-assistants`/`ca` (`src-tauri/Cargo.toml`, root `package.json`, `tauri.conf.json`, capability configs, lockfiles) — owner-confirmed 2026-08-10; dropped from the roadmap during the capability-file restructure, re-added here (Claude verification pass) | 📋 Pending |
| I8 | Keep Rust and TypeScript/React source units bounded to 500 lines, organized by responsibility, without changing their public API, CLI, or UI contracts ([#158](https://github.com/ACFHarbinger/Coding-Assistants/issues/158)) | 🚧 In progress · 2026-08-13 |

## I8 — bounded source modules

- Split every production Rust, TypeScript, and React source unit above 500
  physical lines into responsibility-focused modules while preserving its public
  API, CLI behavior, IPC contract, and UI behavior.
- Use the organization directories already created for Hub settings/bridge,
  Claude, and TUI modules where they fit; add focused directories only where a
  clear boundary exists.
- Add a repeatable source-length inventory/check before closing the issue, and
  run the affected crate/frontend/docs test and build commands after each slice.
- This is a refactor-only programme: behavior changes need their own roadmap
  entry and issue rather than being folded into a mechanical split.
- **Claude's slice — done:** `crates/hub/src/bridge/claude_channel.rs` (1,069
  LoC) split into `bridge/channels/claude/{mod,workspaces,events,reply,
  permissions,terminal}.rs` (largest: 394); `crates/claude/src/main.rs` (613
  LoC) split into `src/main.rs` (thin `#[path]` entry point) plus
  `src/main/{cli,protocol,server}.rs` (largest: 307);
  `src/components/settings/SettingsApp.tsx` (812 LoC) split by tab into
  `settings/tabs/{shared,GeneralTab,WorkspaceTab,MemoryTab,
  OrchestrationTab}.tsx` (largest: 242), with `SettingsApp.tsx` itself down
  to 457. Public API, MCP protocol, CLI subcommands, and Settings UI/state
  behavior all preserved — see the C14.3 roadmap entry and #150 for the
  Claude Channel side. All existing tests still pass; added a few
  module-boundary tests (`terminal_exec_prefix_*`, `handle_request_*`).
- **Grok's frontend/Grok slice — done:** `ConfigPanel.tsx` (513) split into
  `config/{types,WorkSessionSection}.tsx` (now 394); `MessagerPanel.tsx`
  (574) split into `messager/{sendTagged,useHarnessDelivery}.ts` (now 492);
  Channels UI extracted to `hub/ChannelsTab.tsx`. `bridge::grok` is the
  C12 adapter; C14 connect/spawn lives in `bridge::channels::grok`.
  Owned TS/TSX and Grok Rust files are ≤500 LoC.
