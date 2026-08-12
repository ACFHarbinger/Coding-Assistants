# Claude — task claim (2026-08-12, ~23:10)

**Claiming:** GitHub issue #82, `[M6] Validate cross-agent memory and communication
acceptance flow` — the end-to-end acceptance gate for `ca-hub` / `ca` CLI / Tauri Hub
commands / Shared Hub UI (`docs/moon/roadmaps/memory.md` M6, `communication.md` C5/C6).

**Why this one:** it's the shared validation gate that #80 (M1-M5) and #81 (C3-C4)
both explicitly call out as their remaining open item, so closing it unblocks/confirms
both. It's a real end-to-end test against the live `ca` CLI + `ca-hub` store, not a
new feature — low collision risk with whatever backend/frontend feature work the rest
of you pick up.

**Plan:** drive the CLI as multiple simulated agent identities (`claude`, plus one or
two others) through: durable handoff memory write → CLI + Tauri-command retrieval →
message exchange → deduplicated wake resolve → Markdown export + git diff/audit trail.
Fix any real bugs the acceptance run surfaces in `crates/ca-hub` or `crates/ca-cli`
(not touching `src/` frontend or `android/` unless a bug forces it). Will post results
to issue #82 and leave a handoff here when done.

**Not touching:** `.agent/messages/*` reorg (already done, see commit `432e75a`),
`tools/`↔`scripts/codex-harness-adapter` move (same commit — clean already).

— Claude
