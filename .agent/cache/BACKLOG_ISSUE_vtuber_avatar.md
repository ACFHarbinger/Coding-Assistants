<!--
Draft backlog issue — NOT yet filed (gh token was invalid on 2026-09-10).
To file once re-authenticated (`gh auth login -h github.com`):

  gh issue create \
    --repo ACFHarbinger/Coding-Assistants \
    --title "[Feature]: Animated V-Tuber avatar presence for agents & humans (Open-LLM-VTuber bridge)" \
    --label enhancement \
    --body-file .agent/cache/BACKLOG_ISSUE_vtuber_avatar.md

Then move it to the project board's Backlog column and delete this file
(or leave it as the drafted-from record). Roadmap: ui.md U17.
-->

## What problem does this solve?

Agents and human devs in Chat & Memory are represented by a **static** profile
picture (`hub_set_agent_avatar`, `AgentAvatar.tsx`, `AgentRecord.avatar_attachment_id`).
This is an explicitly-labelled **fun gimmick** (owner, 2026-09-10), not a
capability gap: let an identity opt into a **live Live2D avatar** that reacts to
conversation — expression changes, lip-sync, optional TTS — instead of a still
image. It makes multi-agent sessions more legible and more fun to watch, and is
a natural showcase for the team-roles / profile-customization work.

## Proposed solution

- **Per-identity toggle** in profile settings: `off` ⇒ current static avatar,
  `on` ⇒ animated model. Stored next to `avatar_attachment_id`.
- **Bridge, not a drop-in MCP server.**
  [Open-LLM-VTuber](https://github.com/Open-LLM-VTuber/Open-LLM-VTuber) is a
  standalone app (Live2D rendering + TTS/ASR + its own LLM loop with a
  WebSocket/HTTP API and a web frontend). The integration is a thin adapter the
  CA app owns that:
  - spawns / connects to a local Open-LLM-VTuber instance (one per active
    animated identity, or one shared instance switched by model),
  - forwards each new assistant message for that identity to it (text →
    expression + optional speech), suppressing its built-in LLM loop so CA
    stays the source of truth,
  - embeds the rendered avatar in the message list / a presence strip
    (webview or `<canvas>` stream).
- Related to `platform.md` **P9** (MCP external-server integration) in spirit;
  if Open-LLM-VTuber gains a real MCP surface, fold the bridge into the P9
  registry instead of a bespoke adapter.

## Dependencies (cut these first)

1. **Profile customization** — humans *and* agents choose their own avatar
   (extends the existing avatar infra). Not yet roadmapped.
2. **Settings → team roles.** Not yet roadmapped.

Both are prerequisites; this issue stays in the backlog until they land.

## Scope / non-goals

- Not a 1.0 item. Backlog, post-1.0.
- No voice **input** (ASR) from Open-LLM-VTuber — CA drives it one-way.
- Keep it optional and default-off: a headless / CI / low-power run must be
  completely unaffected, and the static-avatar path stays the default.
- Dependency policy: Open-LLM-VTuber runs as an **external process**, not a
  bundled dependency — no new frontend npm packages for the Live2D runtime
  beyond what an embedded webview already provides.

## Affected area

Frontend (src/) + Backend (src-tauri/) — a new bridge adapter plus the
profile-settings toggle and the message-list render surface.

## Acceptance criteria

- [ ] Profile settings expose an "animated avatar" toggle per identity (human
      and agent), persisted.
- [ ] With it on, a local Open-LLM-VTuber instance is launched/connected and
      its built-in LLM loop is disabled.
- [ ] Each assistant message for that identity drives the avatar (expression;
      speech if TTS enabled) with no double-generation.
- [ ] With it off — the default — behaviour and resource use are byte-for-byte
      the current static-avatar path.
- [ ] Bridge failure degrades to the static avatar with a one-line notice,
      never a blocked message stream.
