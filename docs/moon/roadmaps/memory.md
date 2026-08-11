# Shared Memory and Private Journals Roadmap

Top-priority capability for the local-first collaboration hub.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| M1 | SQLite schema: `agents` (seeded with the PC2 roster), `memories`, `messages`, `wake_requests` | Migrations run on a clean clone; records are queryable by scope and agent | ✅ **Done** · `HubStore` migrate + seed agents; verified `cargo test -p ca-hub` |
| M2 | Recent raw memory plus episodic and semantic long-term tiers | Compaction preserves important decisions and links each compressed memory to source events | ✅ **Done** · tiers + search + `promote_memory` / `compact_short_term` (`source_event_id`); 2026-08-11 |
| M3 | Git-tracked Markdown exports for high-priority tasks, handoffs, and architectural decisions | Exported files are human-editable and reproducible from SQLite | 🚧 **Partial** · `export_markdown` writes episodic + semantic + **handoffs** to `markdown/shared_memory.md`; not auto-git-commit |
| M4 | Private per-agent journals under a separate, non-shared directory, with optional **opt-in, owner-permissioned** per-agent encryption | Each agent can write without overwriting another agent; private data never enters shared exports by default; the shared/durable store (M1–M3) is **never** encrypted | 🚧 **Partial** · `journals/<agent>/journal.md` + isolation tests; encryption still open |
| M5 | Memory review/edit/delete/stale workflows and bounded transcript retention | Human can correct or remove memories; retention policy is tested | 🚧 **Partial** · stale/delete/purge-stale/age-out + desktop Hub review; full TTL scheduler not automated |
| M6 | Acceptance gate: owner and two external agents retrieve and use a prior handoff on a real repository task | End-to-end transcript, memory, Git changes, and provenance are reviewable | 🚧 **In Progress** · `m6_cross_agent_handoff_acceptance_flow` covers durable handoff, provenance, wake dedup/resolve, and Markdown export; real multi-agent repository run remains |

SQLite is the source of truth for structured memory. Markdown is the
human-readable synchronization and high-priority layer. Private journals are
separate from shared memory and must not be silently merged.

**2026-08-11:** Desktop UI and Tauri commands share the store with `ca` CLI.
See `docs/moon/CHANGELOG.md` Unreleased and `crates/README.md`.

**2026-08-11:** The first executable M6 acceptance flow now exercises a
cross-agent handoff, provenance-linked episodic memory, inbox acknowledgement,
deduplicated wake delivery, and Markdown export in one isolated Hub test.

**Implementation note (Claude, 2026-08-10):** an earlier pass of this file
described a different, incompatible `ca-hub` schema that briefly coexisted
after concurrent sessions; reconciled by wiring `store.rs` as the real
implementation.
