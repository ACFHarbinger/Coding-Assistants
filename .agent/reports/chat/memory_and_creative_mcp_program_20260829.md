# Program design — Memory upgrade + Creative-Tool MCP servers

**Status:** design, PR #1 of the sequence (not a substitute for implementation).
**Date:** 2026-08-29. **Author:** Claude (Sonnet 5), session `session_01RrZbBjc6u8x5yrEhdis6Zx`.
**Owner decisions already taken (AskUserQuestion):**
- Memory: do all four — semantic/vector retrieval, better link graph + auto-recall,
  consolidation/summarization, cross-tool memory scope.
- MCP: **CA hosts one MCP server per tool**.
- Scope: all 7 targets, sequenced across many PRs.
- Runtime: whatever fits each tool (Rust core where possible; native scripting per app).

This is **two independent tracks** that share almost nothing. They must not share
PRs. Track M (memory) is pure `crates/hub` + `src-tauri` + frontend. Track C
(creative-tool MCP) is new crates + native plugin code + config plumbing.

---

## Delivery layer — how a CA-hosted MCP server actually reaches an agent (verified)

The **proven** mechanism is `<workspace>/.mcp.json` merging, already implemented
for the Claude Channel bridge:

- `crates/hub/src/bridge/channels/claude/workspaces.rs` owns per-workspace MCP
  config files under `<hub_home>/channel-servers/<key>.mcp.json` plus a
  `global.mcp.json` base layer. `merge_mcp_configs(global, workspace_specific)`
  produces the *effective* config, which the caller writes into
  `<workspace>/.mcp.json` — the file Claude Code natively reads. A server entry is
  `{ "command": "<abs binary path>", "args": [...] }`.
- `crates/claude` is the **only working MCP server in the repo** and therefore the
  template: a Rust `[[bin]]` (`coding-assistants-claude-channel`), deps `hub` +
  `serde_json`, hand-rolled stdio JSON-RPC in `src/main/protocol.rs` +
  `src/main/server.rs`, a `--setup <workspace>` subcommand in `src/main/cli.rs`
  that does the `.mcp.json` merge.

**Do NOT build on `MCP_CONFIG_FILE`.** `src-tauri/src/client/llm.rs:330` sets that
env var, but only for `opencode` spawns and only when `work_dir` is set. It is not
a standard MCP client convention and there is no evidence in this repo that any
client consumes it. `src-tauri/src/lib.rs:166` writes a third file
(`mcp_config.json`) that also appears unused. Track C's delivery layer is
`.mcp.json` merge, generalized (see C-1).

**Client coverage caveat:** `.mcp.json` is Claude Code's convention. Codex, Gemini
CLI, opencode, grok each have their own MCP config location/flag. C-1 must
generalize the `channels/claude/workspaces.rs` registry into a client-agnostic
"app-managed MCP servers" layer that can render per-client config
(`.mcp.json`, `~/.codex/config.toml` `[mcp_servers]`, Gemini's `settings.json`,
`opencode.json`), not just Claude's file. This is the first real design problem
of the track and C-1's main deliverable.

---

## Track M — Memory upgrade

### Current state

`crates/hub/src/store/models/{memories,memory_links}.rs`, SQLite via `rusqlite`.
Schema in `crates/hub/src/store/policies/audit.rs::migrate()` — one idempotent
`CREATE TABLE IF NOT EXISTS` batch, no versioned migration framework, a `meta`
key/value table available for a schema version. Retrieval today:
`search_memories` = `LIKE` on `body`/`title`/`tags_json`; `suggest_links_for_memory`
= tag/token overlap heuristic with a conservative score. No embeddings, no FTS5.
Tiers: `short_term` / `episodic` / `semantic`. Scopes: `workspace` / `global`.
Surfaced via `crates/cli/src/command/memory.rs`, `src-tauri/src/commands/messager/memory.rs`,
and `MemoryTab.tsx` / `MemoryDrawer.tsx` / `MemoryLinksSection.tsx`.

### Dependency order (these are NOT peers)

```
M1 semantic/vector retrieval   ← foundational; schema + embedding dependency
      │
      ├── M2 auto-recall / RAG injection   ← changes every agent's context
      │
      └── M3 consolidation / summarization ← background LLM job
              │
              └── M4 cross-tool memory scope ← trivial extension, needs Track C first
```

### M1 — semantic / vector retrieval

**Decision required before coding: embedding source.**
- **Local (recommended default):** a small ONNX/GGUF sentence-embedding model
  (e.g. `bge-small-en` / `all-MiniLM-L6-v2`, ~90–130 MB) run via `fastembed-rs`
  or `candle`. Keeps CA offline-capable (a stated v1.0 value), no per-call cost,
  no key. Cost: bundle size + a cold-start model load.
- **API:** OpenAI / Voyage / Cohere embeddings. Smaller binary, better quality,
  but adds a network dependency and a key to the memory write path, and breaks
  offline use.
- Recommendation: **local, with an optional API override in Settings.** Store the
  model choice in `meta`.

**Storage:** `sqlite-vec` (the maintained successor to `sqlite-vss`, a single
loadable extension, `rusqlite` `load_extension`) — a `vec0` virtual table
`memory_vectors(memory_id, embedding float[384])`. Fallback if the extension
won't load on a target: a plain `BLOB` column + brute-force cosine in Rust
(fine to ~10⁴ memories). Add `schema_version` to `meta`; gate the new table on it.

**API surface (`crates/hub`):**
- `write_memory*` → also compute + upsert the embedding (feature-flagged; a
  write must still succeed if embedding fails — log and continue).
- `search_memories_semantic(query, k, scope, tier_filter) -> Vec<(MemoryRecord, f32)>`.
- `search_memories` stays; a new `search_memories_hybrid` blends LIKE + vector
  (reciprocal-rank fusion) and becomes the default for the UI/CLI/commands.
- Backfill command: `ca memory reindex` embeds all rows lacking a vector.

**Frontend:** `MemoryTab` / `MemoryDrawer` search switches to hybrid; show a
relevance score; no schema knowledge in the UI.

**PRs:** M1a schema + `sqlite-vec` load + `meta.schema_version` + brute-force
fallback. M1b embedding provider (local model, `fastembed-rs`) behind a
`hub` feature. M1c wire into `write_memory*` + `search_*` + `reindex`. M1d
CLI/commands/UI switch to hybrid.

### M2 — auto-recall / RAG injection

Once M1 lands: before an agent turn, retrieve top-`k` semantically-relevant
memories for the prompt + workspace and inject them as context.

- **Where:** the agent spawn path (`src-tauri/src/agent/orchestrator.rs`,
  `src-tauri/src/client/llm.rs`) and/or the Channel bridge's `check_inbox`-style
  tools. Least invasive: a new MCP tool `recall(query, k)` the *agent* calls
  on its own initiative (mirrors the existing `check_inbox` design philosophy in
  `crates/claude/README.md` — push only what's worth interrupting for, let the
  agent pull the rest). Recommended over always-on injection.
- **Always-on option:** prepend a "Relevant memories" block to the first user
  message of a session. Costs tokens every session; make it a Settings toggle,
  default off.
- Budget: cap injected memory chars; dedup against what's already in context is
  out of scope (the agent tolerates mild redundancy — same call made for #163's
  pending-state work).

**PRs:** M2a a `recall` MCP tool (in the Track-C shared server, or a dedicated
`coding-assistants-memory` MCP server — decide in C-1). M2b optional always-on
injection + Settings toggle.

### M3 — consolidation / summarization

Background job: roll `short_term` → `episodic` → `semantic` with LLM
summarization, dedup near-identical memories, tune decay.

- Extends the existing `hub_compact_short_term` / `hub_age_out_short_term` /
  `hub_purge_stale_memories` commands (already in `messager/messaging.rs`).
- Needs an LLM call from a background context — reuse `src-tauri/src/client/llm.rs`
  with a cheap model; must run on `spawn_blocking` / a Tokio task, never the IPC
  thread (see #163).
- Dedup uses M1's embeddings (cosine > threshold → merge, keep the richer body,
  union tags, relink).
- Trigger: on a timer, on app idle, or an explicit "Consolidate" button. Start
  with the button; add the timer later.

**PRs:** M3a summarization helper + merge/dedup on embeddings. M3b promotion
pipeline (`short_term`→`episodic`→`semantic` with summaries). M3c scheduling.

### M4 — cross-tool memory scope

**Blocked on Track C.** Add `tool` (nullable) + keep `workspace_path` to
`memories`; a memory can be scoped to a creative-tool session ("this Blender
rig", "this Krita brush setup"). The per-tool MCP servers (Track C) get
`remember(body, tags)` / `recall(query)` tools that write/read with
`tool = "blender"` etc. Small once C exists; do not attempt before.

---

## Track C — Creative-tool MCP servers

### C-1 — the framework (do this before any tool)

1. **Generalize the app-managed MCP registry.** Lift
   `crates/hub/src/bridge/channels/claude/workspaces.rs`'s per-workspace +
   global config store out of the `claude` channel namespace into a
   `hub::mcp` module: a registry of app-managed MCP servers, each with
   `{ key, display_name, command, args, scope: global|workspace, enabled }`,
   and a **renderer per client** (`.mcp.json`, Codex `config.toml`
   `[mcp_servers]`, Gemini `settings.json`, `opencode.json`). The Claude Channel
   server becomes the first registry entry, not a special case.
2. **Shared MCP server crate.** `crates/mcp-core` — extract the stdio JSON-RPC
   loop + `initialize`/`tools/list`/`tools/call` handling from
   `crates/claude/src/main/{protocol,server}.rs` into a reusable lib. Each tool
   server is then `crates/mcp-<tool>` depending on `mcp-core`, implementing a
   `ToolProvider` trait (`fn tools() -> Vec<ToolSchema>`,
   `fn call(name, args) -> Result<Value>`).
3. **Bridge transport contract.** Every tool server talks to a *running instance*
   of its app. Standardize the app-side transport: a localhost TCP or Unix-socket
   line-JSON channel the native plugin opens, the MCP server connects to.
   `mcp-core` ships the client half; each native plugin ships the server half in
   its own language. One contract, N implementations.
4. **Lifecycle + discovery.** CA detects whether the app is installed / running
   (reuse `core::process_detector` — already used by `detect_agent_processes`),
   shows per-tool status in a new Settings "Creative Tools" tab, installs the
   native plugin on request, and adds/removes the registry entry.
5. **Reference doc** `crates/mcp-core/CONTRACT.md`: the app-side line-JSON schema,
   the `ToolProvider` trait, the config-render matrix.

**C-1 acceptance:** the Claude Channel server is migrated onto the registry with
no behavior change (all its existing tests green), and `mcp-core` has a trivial
`echo` tool server wired end-to-end into a `.mcp.json` and loaded by a real
`claude` session.

### Tool tiering — honest difficulty and risk

| Tier | Tool | Bridge | Risk |
|---|---|---|---|
| 1 | **Blender** | in-process `bpy`; addon opens the socket, `mcp-blender` connects | Low. `bpy` is complete and stable. Reference implementation. |
| 1 | **Krita** | PyKrita plugin (`Extension` + `QThread` socket) | Low–med. Python API is real but thinner than `bpy`; some ops need Qt-main-thread marshalling. |
| 2 | **Godot** | editor plugin (`EditorPlugin` in GDScript) or GDExtension; no Python | Med. GDScript socket + JSON is fine; the API surface for editor automation is smaller and version-sensitive (4.x only). |
| 2 | **Aseprite** (the pixel-art target) | Lua script + CLI (`aseprite -b -script`) | Med. Lua API is capable but has **no long-lived socket** — likely a request/response model driving `aseprite -b` per call, not a live session. Slower, stateless. |
| 3 | **Unreal Engine** | in-editor Python (`unreal` module) | High. Editor-only, version-pinned (5.x), Python is an opt-in plugin, remote-execution needs the "Python Editor Script Plugin" + remote exec enabled. Large surface, brittle across versions. |
| 3 | **Unity** | C# editor package + external transport | High. Needs a shipped `.unitypackage`/UPM package with an `EditorWindow`/`[InitializeOnLoad]` socket server; C# build + Unity version matrix; no headless story for editor ops. |
| 4 | **OpenToonz** | C++ plugin SDK (`toonz_plugin`) | **Possibly not viable in this model.** The plugin SDK targets image *effects*, not app automation/scripting; there is no general scripting/IPC surface. May be limited to a thin "open file / render" CLI wrapper, or dropped. Decide after Tier 1–2 ship; do not promise it.

**"Any pixel art editor" is not implementable as stated.** Aseprite is the only
pixel editor with a real automation API and is the named target. Others
(LibreSprite — Aseprite fork, partial API compat; Pixelorama — Godot-based, no
external API; GraphicsGale, Pyxel Edit — none) are out unless a specific one is
requested with a viable API.

### Sequencing

```
C-1  framework (mcp-core, registry generalization, contract doc, echo server)
 └── C-2  Blender          (Tier 1 — reference; proves the framework)
      └── C-3  Krita       (Tier 1 — second consumer; shakes out wrong abstractions)
           │   ── review the framework here; refactor mcp-core if C-2/C-3 fought it ──
           ├── C-4  Godot        (Tier 2)
           ├── C-5  Aseprite     (Tier 2 — stateless variant; may force a `ToolProvider` variant)
           ├── C-6  Unreal       (Tier 3)
           ├── C-7  Unity        (Tier 3)
           └── C-8  OpenToonz    (Tier 4 — viability spike FIRST; may become "wontfix")
```

Build **two** tools (Blender, Krita) before generalizing further — that is what
surfaces the wrong abstractions in `mcp-core`.

### Per-tool PR shape (repeat for C-2..C-8)

1. `crates/mcp-<tool>` — `ToolProvider` impl + tool schemas + unit tests with a
   mock socket peer.
2. `plugins/<tool>/` — the native plugin (Python/GDScript/C#/Lua/C++), its own
   README with install steps, a smoke script.
3. `hub::mcp` registry entry + `core::process_detector` signature for the app.
4. Settings "Creative Tools" tab: status row, install button, enable toggle.
5. Docs: `plugins/<tool>/README.md`, a line in `docs/moon/`.

Each tool server binary: `coding-assistants-mcp-<tool>`. Keep tool schemas
small and task-oriented (`create_primitive`, `run_script`, `export`,
`get_scene_summary`) — not a 1:1 API mirror.

---

## Cross-cutting

- **Security:** each tool server executes arbitrary operations in a creative app
  (including `run_script`). Gate behind the same enrolled-identity model the
  Channel bridge uses; default the `run_script`/eval tool to **off**, opt-in per
  workspace in Settings. Never expose a raw-eval tool without that gate.
- **Offline:** Track M's default (local embeddings) and all of Track C keep CA
  offline-capable. Only M1's optional API override and M3's summarization LLM
  need network.
- **`cargo-audit` / `pip-audit`:** already red on `main`; `fastembed-rs` /
  `sqlite-vec` and any Python plugin deps must not make it worse. Add plugin
  Python deps to `git/pyproject.toml`'s audit scope or a per-plugin lock.
- **CI:** new crates join the workspace `cargo test` / clippy automatically.
  Native plugins need their own lightweight lint/smoke jobs (ruff for Python,
  `gdscript` check, etc.) — add per plugin, don't block the first PRs on it.

## Delegation (this is a multi-session program)

Slices are large and parallelizable **after C-1 + M1 land**. When delegating:
- One `git worktree` per slice under `~/Repositories/Repo/.ca-worktrees/`.
- An explicit file-ownership boundary per slice, **recorded in
  `.agent/cache/AGENT_BUS.md`** where the other sessions read it — the shared
  checkout caused two collisions on 2026-08-29 (`relaunch/mod.rs`, `HubPanel.tsx`).
- Track M and Track C never in the same slice.
- Copy gitignored build output out of a worktree before removing it.

## First three PRs (recommended order)

1. **This doc** (PR #1).
2. **C-1** framework — highest architectural risk, unblocks all of Track C.
3. **M1a** schema + `sqlite-vec` load + `meta.schema_version` — unblocks all of Track M.

C-1 and M1a are independent and can run in parallel in two worktrees.

---

## Progress log

**2026-08-29 (session `session_01RrZbBjc6u8x5yrEhdis6Zx`)**
- **#176** design doc merged.
- **C-1a #177** merged — `crates/mcp-core` (stdio JSON-RPC server, `ToolProvider`,
  `Emitter`); `crates/claude` Channel bridge migrated onto it, 15 tests unchanged;
  `crates/mcp-echo` reference server.
- **C-1b #178** merged — `hub::mcp` client-agnostic config rendering
  (`render_merged(ClientKind, &[McpServerEntry], existing)`) for Claude `.mcp.json`,
  Gemini `.gemini/settings.json`, opencode `opencode.json`, Codex `~/.codex/config.toml`.
  Idempotent, preserves hand-added servers. Existing `channels/claude/workspaces`
  registry left untouched.
- **C-2 #179** merged — `crates/mcp-blender` (`coding-assistants-mcp-blender`,
  port 9765) + `plugins/blender/coding_assistants_bridge.py`. 7 tools
  (`run_python` gated behind `--allow-run-python`). bpy addon marshals to main
  thread via `bpy.app.timers`.
- **C-3 #180** merged — `crates/mcp-krita` (port 9766) + `plugins/krita/`
  (PyKrita `.py` + `.desktop`, QTimer pump). **Also** extracted the shared socket
  transport into `mcp_core::app_link` (`AppLink` trait, `TcpAppLink`,
  `parse_response`, `result_to_text`); `mcp-blender` shrank ~120 lines.

**Framework validated across 2 consumers — abstraction held, no mcp-core refactor
needed beyond the app_link extraction.** Per-tool code is now `tools()` + the
`--allow-run-python` gate string only.

**Not yet done:** C-4 Godot, C-5 Aseprite, C-6 Unreal, C-7 Unity, C-8 OpenToonz
(viability spike). Track M (memory) entirely unstarted — M1 vector retrieval is
the foundational piece. `hub::mcp` is not yet wired into a Settings "Creative
Tools" tab or invoked anywhere (C-1b is the library; the UI/registration is
follow-up). Pre-existing `crates/hub` fmt drift keeps resurfacing (CI only
fmt-checks `src-tauri`) — worth a one-shot `cargo fmt` cleanup PR.

**2026-08-30**
- **C-4 #182** merged — Godot bridge (`crates/mcp-godot` port 9767 + `plugins/godot/` EditorPlugin, GDScript, no thread marshalling — `_process` poll).
- **C-5 #183** merged — Aseprite bridge (`crates/mcp-aseprite` + `plugins/aseprite/dispatch.lua`). First **non-socket** transport: `CliAsepriteLink` spawns `aseprite -b --script` per call. `AppLink` trait held unchanged.
- **C-6 #184** merged — Unreal bridge (`crates/mcp-unreal` port 9768 + `plugins/unreal/init_unreal.py`, slate-post-tick pump). Tier 3, uncompiled against a real editor.
- **C-7 #185** — Unity bridge (`crates/mcp-unity` port 9769 + `plugins/unity/` C# Editor package w/ embedded MiniJSON, `EditorApplication.update` pump). Tier 3, not compiler-verified.
- **C-8** — OpenToonz **viability spike → confirmed does not fit** the socket-bridge model (no scripting API, no plugin IPC, `toonz_plugin` SDK is image-effects only, no mainline headless render). Shipped a deliberately minimal `crates/mcp-opentoonz`: `scene_info` (parses `.tnz` XML directly, no binary) + gated best-effort `render` passthrough. `plugins/opentoonz/README.md` is the finding. No `plugins/opentoonz/` code.

**Track C is now feature-complete for all 7 targets** (Blender/Krita/Godot fully, Aseprite via CLI, Unreal/Unity untested against real editors, OpenToonz minimal-by-necessity). `AppLink` held across socket + subprocess + file-read transports with no changes. **Still not wired anywhere**: no Settings "Creative Tools" tab, no registration flow — `hub::mcp` (C-1b) is library-only. Every gated tool (`run_python`/`run_gdscript`/`execute_menu_item`/`render`/`apply_script`) is off by default.

**Track M (memory) still entirely unstarted.**

- **C-9a** (this session, backend only) — the registration/wiring layer:
  - `hub::mcp::render_replacing(client, owned_keys, entries, existing)` — like
    `render_merged` but drops any `owned_keys` entry not in `entries`, so a
    toggle-off actually removes the server. `render_merged` now delegates to it
    with `&[]`. Codex TOML tables pruned via `toml_edit`.
  - `hub::mcp::creative` — `CATALOG` (7 `CreativeTool`s: key = `[[bin]]` name,
    display name, default args, `Transport` {socket|subprocess|file-parse}, port,
    `--allow-*` flag string, app process names). Per-workspace **enabled set**
    persisted under `servers_dir(store)/<workspace_server_name>.creative.json`
    (same storage shape as the Channel registry). `apply_to_workspace(ws, entries)`
    rewrites `.mcp.json` / `.gemini/settings.json` / `opencode.json` via
    `render_replacing`; idempotent, spares hand-added servers, won't create empty
    config files. **Codex deliberately excluded** — global `~/.codex/config.toml`,
    never written by a per-workspace toggle.
  - `hub` re-exports `servers_dir` + `workspace_server_name`.
  - Tauri commands (`src-tauri/.../settings/creative_tools.rs`, registered in
    `lib.rs`): `creative_tools_status(workspace)`, `creative_tools_set_enabled(
    workspace, key, enabled)`, `creative_tools_reapply(workspace)`,
    `creative_tools_codex_snippet(workspace)`. Bridge binary resolved against the
    app dir then `$PATH`; **`binaryFound` / `binaryPath` surfaced per tool** so a
    missing bridge is visible rather than a silent MCP start failure. Enabled keys
    with an unresolved binary stay in the set but are not written to a config.
    Best-effort `app_running` via one `ps` sweep.
  - 26 new tests (hub 19 in `mcp::`, tauri-app 3, + advisor-mandated
    enable→disable-removes-key / hand-added-survives at both the renderer and the
    `apply_to_workspace` level). `cargo clippy -p hub -p tauri-app --all-targets
    -D warnings` clean; `src-tauri` fmt clean.

**C-9a open question flagged to Harbinger:** the `mcp-<tool>` bridge binaries are
built into `target/` in dev but **not declared as sidecars/extra binaries in
`tauri.conf.json`** (that config predates these crates) — a packaged v1.0.0 build
ships none of them, so `binaryFound` will be false everywhere until bundling is
decided. C-9a surfaces the state; it does not solve bundling.

**C-9b (next): the Settings "Creative Tools" tab** — `CreativeToolsTab.tsx` +
`"creative-tools"` TabId, per-tool rows (status chips: installed / running /
registered, enable toggle, Codex snippet copy). Depends on the C-9a command
shapes above.
