# Crate refactor handoff — Chat

## Objective

Finish organizing `crates/cli/src` and `crates/hub/src` so no Rust source
file exceeds 500 lines, while preserving the `ca` CLI interface and the
public `hub` API.

## Current verified baseline

- `9656bd4 refactor(hub): group bridge and harness adapters` moved Hub bridge
  adapters to `crates/hub/src/bridge/` and its harness API to
  `crates/hub/src/harness/mod.rs`.
- `cargo test --workspace` passed: 81 tests; one intentional real-home smoke
  test ignored.
- The user created empty target directories under `crates/cli/src/` and
  `crates/hub/src/`; retain them.

## Required implementation approach

Do **not** split Rust `impl` blocks or functions by textual `include!`
fragments. Rust requires each included source to parse as a complete item.

1. Split `crates/hub/src/store.rs` by complete responsibility-aligned
   `impl HubStore` blocks. Define shared types and `HubStore` in
   `store/mod.rs`; make only required internals `pub(super)` and import the
   exact shared types in each child module. Move tests into independently
   compiled test modules with explicit imports (they do not inherit parent
   imports).
2. Split `crates/cli/src/main.rs` by complete command dispatch functions and
   self-contained helper groups. Keep `[[bin]] name = "ca"` stable; changing
   its source path is fine only after the whole new entry module compiles.
3. Keep every physical `.rs` file at 500 lines or below, including tests.
4. Update stale source-path references in docs/AGENTS, changelogs and relevant
   roadmaps. Regenerate `docs/website/src/data/docs.json`.
5. Verify `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`,
   `cargo test --workspace`, `npm run build`, docs website build, and
   `git diff --check` before committing focused refactor commits.

## Safety notes

- Preserve public `hub` re-exports and all Tauri IPC command names/payloads.
- Do not use PTY/terminal-key injection for harness delivery.
- Do not delete user-created empty directories; remove only files created by
  a failed refactor attempt if necessary.
