#!/usr/bin/env bash
# Comment memory/communication hub progress on related GitHub issues.
# Run from repo root after `gh auth login`.
set -euo pipefail
cd "$(dirname "$0")/.."

gh issue comment 47 --body "## Progress 2026-08-11 — memory compaction / wake dedup

Implemented on local hub (\`ca-hub\`):

- \`compact_short_term\` / \`promote_memory\` with \`source_event_id\` provenance
- Pending **wake deduplication** (same target+message+reason)
- \`memory purge-stale\` / \`age-out\` retention helpers

CLI: \`ca memory compact|promote|purge-stale|age-out\`. Desktop Shared Hub via Tauri \`hub_*\`.
Roadmap: memory.md M2 done, M5 partial; communication.md C3 done.
Commits: \`8903e88\`, \`7b972fc\`."

gh issue comment 45 --body "## Progress 2026-08-11 — shared memory store

SQLite hub at \`~/.coding-assistants/hub.db\` (or \$CA_HOME): tiers short_term/episodic/semantic; search/promote/compact/delete/stale/purge; private journals under \`journals/<agent>/\`.

Desktop: Shared Hub tab. CLI: \`ca memory …\`. See docs/moon/roadmaps/memory.md + CHANGELOG Unreleased."

gh issue comment 64 --body "## Progress 2026-08-11 — wake human-gate policy (C4 partial)

Standing \`WakePolicy\` in hub meta: \`default_requires_human_gate\` (default true), \`allow_auto_wake\`.

CLI: \`ca wake policy --set-default-gate true --set-allow-auto true\`
Tauri: \`hub_get_wake_policy\` / \`hub_set_wake_policy\`.

Per-task tool-approval UI still open (U4)."

gh issue comment 76 --body "## Note 2026-08-11 — hub events still Tauri-coupled

Memory/inbox/wakes are available via CLI without the GUI. Agent orchestration events remain AppHandle-coupled (this issue / P1). HubStore opens per-command; not yet an internal broadcast bus."

echo "Done commenting Coding-Assistants issues."
