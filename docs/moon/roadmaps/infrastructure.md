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
| I8 | Keep Rust and TypeScript/React source units bounded to 500 lines, organized by responsibility, without changing their public API, CLI, or UI contracts ([#158](https://github.com/ACFHarbinger/Coding-Assistants/issues/158)) | 🚧 **Reopened · 2026-08-15** — 5 hand-authored files exceeded the cap post-#161/#162 churn; **split by DeepSeek** (see below). Re-verify the source-length inventory at review. |

## I8 — bounded source modules

- Split every production Rust, TypeScript, and React source unit above 500
  physical lines into responsibility-focused modules while preserving its public
  API, CLI behavior, IPC contract, and UI behavior.
- Use the organization directories already created for Hub settings/bridge,
  Claude, and TUI modules where they fit; add focused directories only where a
  clear boundary exists.
- Add a repeatable source-length inventory/check before closing the issue, and
  run the affected crate/frontend/docs test and build commands after each slice.
- Completed the inventory: Settings persistence and tests, Hub agents/audit/
  roles, CLI branches, Tauri quota tests, and frontend/provider slices are
  split by responsibility. The final source-length inventory has no Rust,
  TypeScript, or React unit above 500 physical lines.
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
- **DeepSeek's slice (I8 reopen) — done (2026-08-15):** five files that
  exceeded the 500-line cap after #161/#162 churn were split, contracts
  preserved exactly:
  - `crates/hub/src/store/mod.rs` (506) → `store/{mod,types}.rs`
    (largest 437) — the record enums/structs moved to `types.rs`;
    `HubStore` and the schema helpers stay in `mod.rs`, which keeps the
    imports the impl submodules glob-import.
  - `crates/hub/src/harness/mod.rs` (507) → `harness/{mod,spawn,inject}.rs`
    (largest 292) — per-harness spawn argv/start in `spawn.rs`, task/wake
    injection dispatch in `inject.rs`, `HarnessId` + request/result
    structs stay in `mod.rs`.
  - `crates/hub/src/store/tests/roster.rs` (598) → `roster.rs` (310) +
    `roster_audit.rs` (74) + `roster_memory.rs` (225) — audit tests and
    memory-tier/link tests extracted to sibling test modules.
  - `crates/cli/src/app/mod.rs` (517) → `app/{mod,commands}.rs` (largest
    391) — the subcommand payload enums moved to `commands.rs`; `Cli` +
    `Command` shells stay in `mod.rs`.
  - `crates/cli/src/command/mod.rs` (547) → `command/{mod,memory,msg}.rs`
    (largest 286) — the Memory and Msg dispatch arms moved to focused
    handler modules; the remaining dispatch stays in `mod.rs`.
  - Verification per the standing thermal constraint: `cargo build
    --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
    `cargo check -p hub -p cli --all-targets` all clean. No `cargo test`
    (owner go-ahead required). My changed files are rustfmt-clean.
