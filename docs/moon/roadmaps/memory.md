# Shared Memory and Private Journals Roadmap

Top-priority capability for the local-first collaboration hub.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| M1 | SQLite schema for global/workspace memories, sessions, messages, decisions, events, provenance, and identities | Migrations run on a clean clone; records are queryable by scope and agent | 📋 Pending |
| M2 | Recent raw memory plus episodic and semantic long-term tiers | Compaction preserves important decisions and links each compressed memory to source events | 📋 Pending |
| M3 | Git-tracked Markdown exports for high-priority tasks, handoffs, and architectural decisions | Exported files are human-editable and reproducible from SQLite | 📋 Pending |
| M4 | Private per-agent journals under a separate, non-shared directory | Each agent can write without overwriting another agent; private data never enters shared exports by default | 📋 Pending |
| M5 | Memory review/edit/delete/stale workflows and bounded transcript retention | Human can correct or remove memories; retention policy is tested | 📋 Pending |
| M6 | Acceptance gate: owner and two external agents retrieve and use a prior handoff on a real repository task | End-to-end transcript, memory, Git changes, and provenance are reviewable | 📋 Pending |

SQLite is the source of truth for structured memory. Markdown is the
human-readable synchronization and high-priority layer. Private journals are
separate from shared memory and must not be silently merged.
