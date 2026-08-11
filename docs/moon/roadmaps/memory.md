# Shared Memory and Private Journals Roadmap

Top-priority capability for the local-first collaboration hub.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| M1 | SQLite schema: `agents` (seeded with the PC2 roster), `memories`, `messages`, `wake_requests` | Migrations run on a clean clone; records are queryable by scope and agent | 🚧 In Progress · `crates/ca-hub::store::HubStore` (verified 2026-08-10: `cargo test -p ca-hub` + `cargo check --workspace` green, end-to-end `ca` CLI smoke test of every subcommand in `crates/README.md`). **Not yet in schema**: dedicated decisions/events/provenance/session tables — currently folded into `memories`/`messages` generically |
| M2 | Recent raw memory plus episodic and semantic long-term tiers | Compaction preserves important decisions and links each compressed memory to source events | 🚧 **Partial** · tiers + search; `promote_memory` / `compact_short_term` with `source_event_id` provenance and tests 2026-08-11 |
| M3 | Git-tracked Markdown exports for high-priority tasks, handoffs, and architectural decisions | Exported files are human-editable and reproducible from SQLite | 🚧 In Progress · `ca export-markdown` writes episodic+semantic memories to `markdown/shared_memory.md`, verified 2026-08-10; not yet wired to auto-run or git-commit itself |
| M4 | Private per-agent journals under a separate, non-shared directory, with optional **opt-in, owner-permissioned** per-agent encryption | Each agent can write without overwriting another agent; private data never enters shared exports by default; the shared/durable store (M1–M3) is **never** encrypted (owner 2026-08-10, security/auditability) | 🚧 In Progress · `ca journal append` writes to `journals/<agent>/journal.md` (file-based, not a DB table — isolates agents by directory); isolation verified (`memory_message_wake_roundtrip` test asserts journal content never appears in `search_memories`); **encryption (RJ2) not implemented** |
| M5 | Memory review/edit/delete/stale workflows and bounded transcript retention | Human can correct or remove memories; retention policy is tested | 🚧 **Partial** · stale/delete/promote/compact + desktop Hub review; TTL retention still open |
| M6 | Acceptance gate: owner and two external agents retrieve and use a prior handoff on a real repository task | End-to-end transcript, memory, Git changes, and provenance are reviewable | 📋 Pending |

SQLite is the source of truth for structured memory. Markdown is the
human-readable synchronization and high-priority layer. Private journals are
separate from shared memory and must not be silently merged.

**Implementation note (2026-08-11):** `ca-hub` memory promotion, deletion, and
short-term compaction are available through both the `ca` CLI and the desktop
Shared Hub panel. Compaction keeps the newest records and promotes older
records to episodic memory while preserving `source_event_id` provenance.

**Implementation note (Claude, 2026-08-10):** an earlier pass of this file
described a different, incompatible `ca-hub` schema (`messages` +
`memory_entries` + `journal_entries` tables) that briefly coexisted on disk
alongside a separate, more complete `store.rs` module (this one) after two
concurrent sessions built the crate at the same time. Reconciled by wiring
`store.rs` in as the crate's real implementation (matches `crates/README.md`
and has the richer schema below) and rewriting `ca-cli` to match; the
earlier schema was discarded, not merged, since it was a strict subset.
