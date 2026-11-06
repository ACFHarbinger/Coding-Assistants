# Kimi `/swarm` + `/goal` task toggle (#363) — design, mock verification, live-verification owed

Status: implemented + mock-verified. **Not live-CLI verified** — the
authoring sandbox has no `kimi` binary and no Moonshot credentials. The
owner must run the live steps in §5 before enabling the toggle for real
work.

## 1. Design

Kimi Code CLI documents two in-session slash commands the app can drive for
free from the existing one-shot delivery path
(`kimi -p <body> --session <id> --output-format stream-json`):

- `/swarm <task>` — swarm mode for one completion on Kimi's built-in
  coder/explore/plan subagents; auto-disables after.
- `/goal <objective>` — persistent objective across auto-continuing turns;
  documented one-shot exit codes 0 complete / 3 blocked / 6 paused.

The toggle is a string prefix applied to `request.body` inside
`deliver_kimi_task_with` (`crates/hub/src/bridge/kimi.rs`) — the spawn argv
is unchanged and no new bridge function exists.

- **Type:** `KimiTaskMode { Swarm, Goal }` (`crates/hub/src/bridge/kimi_mode.rs`).
- **Per-task:** `HarnessInjectRequest.kimi_mode` (serde-default `None`).
  CLI surface: `ca msg tag --dispatch --kimi-mode swarm|goal`.
- **Per-workspace:** `kimi_mode = "swarm"|"goal"` under `[[workspace]]` in
  `settings.toml` (`WorkspaceOverride.kimi_mode`,
  `SettingsStore::set_workspace_kimi_mode`), surfaced as
  `EffectiveHarnessSettings.selected_kimi_mode` (kimi row only) and
  resolved into the request at the same dispatch boundaries that already
  resolve `model`/`effort` (`hub_inject_harness` in
  `src-tauri/src/harness/commands/mod.rs`, `ca msg tag` in
  `crates/cli/src/command/msg.rs`).
- **Precedence:** per-task request field wins over the workspace override;
  both absent = OFF (default).

### Prefix semantics (`apply_kimi_task_mode`)

- Mode `Swarm` → body becomes `/swarm <body>`; mode `Goal` → `/goal <body>`
  (body trimmed).
- A body whose first non-whitespace character is `/` passes through
  **unchanged**, even when a mode is set. This is the deliberate escape
  hatch: `/goal` control subcommands (`status` / `pause` / `resume` /
  `cancel` / `replace`) are driven by sending the explicit slash command as
  the task body (e.g. body `/goal status`) — they are **not** a toggle
  surface and the toggle never rewrites them.
- `/goal` exit-code caveat: the bridge treats any non-zero exit as a
  delivery failure and re-queues (`Queued`). Documented `/goal` exit codes
  3 (blocked) and 6 (paused) therefore currently surface as
  `Kimi worker failed: kimi task worker exited with status ...`. Mapping
  3/6 to structured outcomes is follow-up work, not part of this toggle.

## 2. How the app consumes a managed run's output (traced)

- `kimi_managed_spawn_args` adds `--output-format stream-json`, but
  **nothing in the app reads that stdout**: both spawn paths
  (`run_kimi_managed_task` in `crates/hub/src/bridge/kimi.rs`,
  `spawn_explicit` in `crates/hub/src/harness/spawn.rs`) use
  `Stdio::null()` for stdout. Unlike Cursor/Gemini, the Kimi bridge never
  parses stream-json.
- The only output consumer is the on-disk transcript poller:
  `hub_capture_kimi_session` → `capture_kimi_session(_from)` →
  `agents/main/wire.jsonl` → `record_harness_capture`.
- Consequence: the flagged risk "stream-json's event shape may change under
  swarm mode" cannot break the app directly — that stdout is discarded.
  The real exposure is the **wire.jsonl layout**: swarm subagents plausibly
  write sibling `agents/<worker>/wire.jsonl` transcripts and/or multiplex
  `agentId`-tagged events into the main wire.

## 3. What the pre-#363 code did wrong (mock-demonstrated)

Simulated a `/swarm` run with a mock `kimi` (three agent wires +
multiplexed `agentId` events + unknown event types + a truncated final
line). Old behavior:

- only `agents/main/wire.jsonl` was read → **sibling sub-agent transcripts
  were dropped entirely** (`coder own transcript text`, `explore findings`
  never captured);
- every extracted text recorded as agent `kimi` → **multiplexed sub-agent
  events misattributed** (`coder multiplexed status` posted as `kimi`).

## 4. Hardening (all unit-tested in `crates/hub/src/bridge/kimi_wire.rs`)

- `kimi_session_wire_logs` enumerates `agents/*/wire.jsonl` (main first,
  siblings sorted); legacy single-wire layout unchanged.
- `recent_kimi_wire_texts` attributes per event: an event's own `agentId`
  wins over the wire's directory name.
- `kimi_capture_agent_id`: `main` → `kimi` (unchanged, so existing dedup
  hashes still match); workers → `kimi-<worker>` (sanitized).
- Unknown event types and malformed lines are skipped — an undocumented
  swarm event shape cannot error capture.
- Parsing/capture moved from `src-tauri/src/harness/kimi.rs` into hub so it
  is testable under `cargo test -p hub --lib` (the `tauri-app` crate cannot
  link in the sandbox: missing GTK/WebKit system libs). The Tauri adapter
  is now a thin delegation; **its edit is compile-unverified here**.

## 5. Empirical record — mock-verified vs. live-owed

Mock-verified here (`cargo test -p hub --lib swarm_mock`, log in
`verifier/runs/`): prefix reaches the mock argv exactly
(`/swarm …`, `/goal …`, `/goal status` passthrough); multiplexed stdout is
safely discarded; capture records 5 texts across `kimi` / `kimi-coder` /
`kimi-explore` with dedup on re-poll; unknown event types tolerated.

**Still owed by the owner with a real `kimi` CLI + credentials:**

1. Run one real `/swarm <task>` delivery with the toggle on. Confirm the
   session's on-disk layout matches the mock: does each subagent get its own
   `agents/<worker>/wire.jsonl`, or are subagent events only multiplexed
   into `agents/main/wire.jsonl`? If the shapes differ from §3's
   assumptions, adjust `kimi_session_wire_logs` /
   `recent_kimi_wire_texts` accordingly.
2. Confirm `--output-format stream-json` stdout under swarm mode is
   really ignorable (no sideband control channel the app would need).
3. Run one real `/goal <objective>` one-shot; observe exit codes
   0/3/6 in practice and decide whether 3/6 deserve structured
   `HarnessInjectResult` statuses instead of the generic failure path
   (§1 caveat).
4. Re-run `cargo test -p cli` on the build machine — it cannot link in
   the authoring sandbox (pre-existing: shared `$CARGO_TARGET_DIR`
   ort-sys objects vs glibc 2.36; reproduced at clean HEAD).
5. Compile-check `src-tauri` (`cargo check -p tauri-app`) — impossible in
   the sandbox (GTK/WebKit), and this change touched
   `src-tauri/src/harness/kimi.rs` + `src-tauri/src/harness/commands/mod.rs`.
