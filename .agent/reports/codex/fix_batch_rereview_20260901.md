# Acceptance fix-batch re-review

Reviewed by Codex on 2026-09-01. The repository owner explicitly authorized
Codex to complete both the Codex re-review assignment and DeepSeek's #199 §3
audit-disposition assignment.

## PR #221 — desktop UI

Disposition: **cleared**, subject to normal merge ordering with PR #223.

- The #216 zero-model state now renders an explicit fallback and validation
  hint, preserves custom models, and has focused Vitest/RTL coverage. Codex
  added a missing provider-selection callback assertion in `7ae819c`.
- All touched source files are below 500 lines; the `[Unreleased]` changelog
  covers #213, #215-A, and #216.
- The initial #213 fix removed scroll-range jumps but left residual
  maximized-window sluggishness. Codex pushed `7696c96` to remove the remaining
  WebKitGTK repaint path and constrain the flex scroller. The owner then
  live-tested it and reported: “Now the scroll is perfect.”
- Local `npm test`, `npx tsc --noEmit`, and `npm run build` passed. CI frontend,
  Rust, Android, and docs-build checks passed. The expected `pip-audit` tooling
  failure is covered by the separate #199 disposition.

## PR #223 — consolidated Android companion

Disposition: **changes requested**.

The required reconciliation held:

- `ModelSelectionScreen.kt` retains #208's live/static `ProviderCatalog` union,
  custom/free-text model handling, and fallback-safe provider changes together
  with #209's editable role names and prompt/rule/workflow/skill pickers.
- `MainViewModel.kt` retains merged provider catalogs and the
  `GetAgentResources`/workspace response together with #212 connection-loss and
  reconnect state.
- #214's title/body and the replayed slice are scoped to #209; no former
  #206/#207/#208 overclaim survived. PR #214 is still open, however, so the
  assigned supersession/closure has not completed.
- Every touched source file is below 500 lines (largest production source:
  `orchestrator.rs`, 496; `MainViewModel.kt`, 424), and the changelog covers all
  consolidated Android items.

Changes requested on PR #223:

1. `TaskExecutionScreen.kt` still enables Start Task after a connection dies
   while that screen is open; gate it on live connection state (#212).
2. `ConnectionScreen.kt` accepts `host:port`, but `MainViewModel.kt` passes the
   whole string as the host and still uses port 5555; parse it for initial and
   reconnect clients (#207).
3. Explicit disconnect resets to `AppState()` and clears the in-memory persisted
   host, so the connection screen loses its prefill until process restart
   (#207).
4. Close PR #214 in favor of #223.
5. PRs #221 and #223 conflict in `ConfigPanel.tsx` and the changelog. Preserve
   #221's manual config-path/Browse UI and #223's missing-only two-stage
   bootstrap confirmation when resolving.

Verification rerun by Codex on PR #223 head `c07f611`:

- JDK 21 `./gradlew compileDebugKotlin ktlintCheck assembleDebug`: passed.
- `cargo test -p tauri-app core::agent_resources`: passed (1 test).
- `cargo clippy -p tauri-app --all-targets -- -D warnings`: passed.
- CI frontend, Rust, Android, and docs build: passed.

## #199 §3 audit disposition (DeepSeek assignment)

Completed by Codex on owner authorization. The full per-advisory table is in
`issue_199_section_3_audit_disposition_20260901.md`.

- `cargo audit`: 11 vulnerabilities (all fix-now) plus 26 allowed warnings
  (20 unmaintained, 6 unsoundness entries), each dispositioned.
- Dependabot: 26 open alerts, separately counted as 9 high / 11 medium / 6 low.
- `pip-audit`: no audit result; `uv run pip-audit` cannot spawn the undeclared
  executable. This is a tooling failure, not evidence of Python vulnerabilities.
- #199 remains blocked pending fixes, written owner exceptions for every
  accept/defer row, a reproducible Python dependency audit, and a clean rerun.
