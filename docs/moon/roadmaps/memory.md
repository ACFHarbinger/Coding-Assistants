# Shared Memory and Private Journals Roadmap

Top-priority capability for the local-first collaboration hub.

## Audit integrity MVP (in progress)

- [x] Persist filesystem observations as reviewable SQLite audit events with
  operation, path, observed time, content hash, process context, and status.
- [x] Chain events with SHA-256 links and expose `ca audit verify` so tampering
  or reordering is detectable.
- [x] Add `ca audit watch`, `pending`, `list`, `approve`, and `quarantine`.
- [x] Surface pending events at the owner checkpoint when a journal opens
  (CA-111): desktop **Journal** tab in `HubPanel.tsx` fetches pending audit
  events on mount (tab badge) and on open, with Approve/Quarantine actions
  (`hub_list_audit_events`, `hub_approve_audit`, `hub_quarantine_audit`).
- [ ] Add a privileged Linux auditd/fanotify adapter for originating-writer PID
  attribution; the current user-space watcher labels that attribution
  unavailable instead of guessing.
- [ ] Add append-only export and retention/backup policy for audit records.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| M1 | SQLite schema: `agents` (seeded with the PC2 roster), `memories`, `messages`, `wake_requests` | Migrations run on a clean clone; records are queryable by scope and agent | ✅ **Done** · `HubStore` migrate + seed agents; verified `cargo test -p hub` |
| M2 | Recent raw memory plus episodic and semantic long-term tiers | Compaction preserves important decisions and links each compressed memory to source events | ✅ **Done** · tiers + search + `promote_memory` / `compact_short_term` (`source_event_id`); 2026-08-11 |
| M3 | Git-tracked Markdown exports for high-priority tasks, handoffs, and architectural decisions | Exported files are human-editable and reproducible from SQLite | ✅ **Done** · `export_markdown` writes episodic + semantic + handoffs to `markdown/shared_memory.md`; `export_markdown_git` (`ca export-markdown --commit`, desktop "Export MD + Commit") runs `git add`/`git commit` when the export dir is inside a work tree, no-ops (not an error) otherwise |
| M4 | Private per-agent journals under a separate, non-shared directory, with optional **opt-in, owner-permissioned** per-agent encryption | Each agent can write without overwriting another agent; private data never enters shared exports by default; the shared/durable store (M1–M3) is **never** encrypted | 🚧 **Partial** · `journals/<agent>/journal.md` + isolation tests; encryption still open |
| M5 | Memory review/edit/delete/stale workflows and bounded transcript retention | Human can correct or remove memories; retention policy is tested | 🚧 **Partial** · stale/delete/purge-stale/age-out + desktop Hub review; full TTL scheduler not automated |
| M6 | Acceptance gate: owner and two external agents retrieve and use a prior handoff on a real repository task | End-to-end transcript, memory, Git changes, and provenance are reviewable | ✅ **Board-closed (#82)** · Isolated test + live seed `M6-20260812`; Claude retrieved and ACKed; private journal canary did not leak. Chat never posted a second `M6-ACK`. Grok does not reopen; the original two-harness wording is only partially met. |
| M7 | Memory-to-memory graph links (not just source-event provenance) plus a suggestion/auto-link matcher, gated by an `off`/`suggest`/`auto` policy | Edges are creatable/queryable/depth-walkable via CLI and IPC; a dependency-free scorer proposes candidates with a human-readable reason; `Auto` mode only draws edges above a real, measured threshold, attributed to the system, never an agent | ✅ **Done (#159)** · `memory_links` + recursive-CTE `related_memories` + tag/token-Jaccard `suggest_links_for_memory`/`apply_link_suggestions`; CLI + Tauri IPC exposed; desktop UI (`MemoryLinksSection`, `TopicBrowsePanel`) wired into the Chat & Memory drawer |

SQLite is the source of truth for structured memory. Markdown is the
human-readable synchronization and high-priority layer. Private journals are
separate from shared memory and must not be silently merged.

**2026-09-04 (M4a, #259):** Shared memories support an optional tool identity.
The Hub preserves legacy unscoped rows and offers tool-filtered exact, vector,
and hybrid retrieval for creative-tool consumers.

**2026-09-04 (Track M, #255):** Orchestrated tasks now retrieve a bounded,
relevance-ranked set of workspace and global shared memories and inject it
after `.agent/project_memory.md`. Recall is enabled by default, configurable
per workspace, and emitted as structured data for an owner-visible UI.

**2026-09-04 (Track M, #257):** The desktop memory drawer and Shared Hub
memory view gained a Smart (similarity) / Exact (text) search toggle with
per-result score badges, scope/tier filters, and a Reindex Vectors action,
over the M1 `hub_search_memories_hybrid` / `_semantic` commands.

**2026-09-04 (Track M, #256):** `hub_consolidate_memories` clusters related
live short-term memories by shared tag/token overlap within one scope,
summarizes each cluster into a single episodic memory via the provider, links
every source `consolidated_into` the summary, and marks the sources stale
(never deleted). A missing or empty provider summary is a non-destructive
skip. Periodic auto-trigger is not yet wired — the command is manual for now.

**2026-08-11:** Desktop UI and Tauri commands share the store with `ca` CLI.
See `docs/moon/CHANGELOG.md` Unreleased and `crates/README.md`.

**2026-08-11:** The first executable M6 acceptance flow now exercises a
cross-agent handoff, provenance-linked episodic memory, inbox acknowledgement,
deduplicated wake delivery, and Markdown export in one isolated Hub test.

**2026-08-12:** Live M6 seed written to the owner hub (`~/.coding-assistants`):
episodic workspace memory `M6-20260812 live handoff`, targeted handoff
messages to `chat` / `claude` / `gemini` / `human`, and linked wakes. Grok
private journal canary stayed out of shared search and `export-markdown`.
Claude ran the isolated CLI/Tauri acceptance and ACKed the live retrieve
(issue #82). Grok landed the team-roster hole that blocked Messager `#general`
fan-out (`525f07c`). Still waiting on a second harness ACK before calling
the product gate done.

**2026-08-13:** CA-111 (Claude) surfaces pending audit events on the desktop
Journal tab. Remaining audit MVP: privileged writer-PID adapter and
append-only export/retention.

**Implementation note (Claude, 2026-08-10):** an earlier pass of this file
described a different, incompatible `hub` schema that briefly coexisted
after concurrent sessions; reconciled by wiring `store.rs` as the real
implementation.

**2026-08-14 (M7, Claude + Grok + Codex, #159):** memory-to-memory graph
links landed — `memory_links`, depth-bounded traversal, and a dependency-free
tag/token-Jaccard matcher behind a new `off`/`suggest`/`auto`
`LinkSuggestionMode` setting. Backend (store, CLI, Tauri IPC) is done and
`cargo build --workspace` clean; no frontend UI calls the new commands yet.
Worth remembering: the auto-accept threshold started as a guessed 0.55 and
had to be recalibrated to 0.35 after a real smoke test showed genuinely
related memories scoring only 0.39-0.42 — a similarity threshold picked "by
feel" without a real example is not a real threshold. See
`docs/moon/CHANGELOG.md` for full detail.

**2026-08-14 (M7 closed, Gemini + Codex):** frontend UI landed —
`MemoryLinksSection` (per-memory expandable links panel with a "Find
related" suggest-and-link action) and `TopicBrowsePanel` (a new "Browse by
topic" mode in the Chat & Memory drawer, grouping search results by
`agent_id` side by side). `cargo build --workspace` + `npm run build`
(`tsc` + `vite build`) both clean. M7 is done end to end: schema, matcher,
CLI, Tauri IPC, and desktop UI.
