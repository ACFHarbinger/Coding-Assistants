# Coding Assistants 1.0.0 acceptance-fix batch review

Date: 2026-09-01  
Reviewer: Codex  
Baseline: `main` at `51e26fc`; #216 merge `527c6d1`; local/remote branch tips #214 `9c6885c`, #205 `96f171c`, #213 `705e5ab`

## Verdict

**Changes requested. Do not merge #214 or #205, and do not open/merge #213 as ready, until the blockers below are resolved.** #216 compiles and fixes the originally empty desktop provider list, but it still needs a changelog entry and a focused regression test under the batch's stated ready-for-review rule.

## Findings

### Blocker — #214 does not contain the reconciled #206/#207/#208 scope, and #209 endpoint parity is absent

The tip of `feat/android-task-config` is a single commit, `9c6885c`, whose functional diff is the #209 role/resource slice. It does not contain:

- #206's structured/plain-language approval context or gate preview. `DashboardScreen.kt` remains the raw `Agent`, `Reason`, and wake-id card.
- #207's persisted server IP or revised connection UX. `MainViewModel` is still a plain `ViewModel`, and `ConnectionScreen.kt` still initializes `ipAddress` to `"192.168.1."`; there is no `SharedPreferences` use.
- #208's provider/model fallbacks, editable model field, expanded provider names, or related icon/color updates. The provider menu is still driven only by `availableModels.keys`, so it remains empty when the server returns no discovered models.
- The endpoint portion of the reconciled #209 claim. Android `ModelConfig` at `TcpClient.kt:78-85` has provider/model and four resource fields but no `endpoint`, and `ModelSelectionScreen.kt` has no endpoint control.

This is not merely a comparison artifact: neither current `main` nor the branch contains the missing #206/#207/#208 symbols/behavior. The older branch-local bus correctly describes `9c6885c` as isolated #209 work; the later reconciled bus incorrectly treats PR #214 as carrying all four issues. The PR/batch scope and changelog must be corrected, or the missing work must actually be added and device-tested.

### High — all three open branches are pre-#216 and are not ready in their current ancestry

- `feat/android-task-config` and `fix/213-maximized-scroll` merge-base at `30d6ab5`, before #216.
- `ci/sidecar-composite-action` merge-base is even older, `70726f4`.

All must rebase onto post-#216 `main`. A replay of `9c6885c` onto current `main` produced only an `AGENT_BUS.md` conflict; `ModelSelect.tsx` auto-merged correctly, retaining #216's provider logic and #214's separate skill-picker hunk. #205 also has bus-history overlap. #213's CSS change merges cleanly.

The reconciled bus overstates the stale-source-hunk problem: at the reviewed tips, only #214 changes `ModelSelect.tsx`, and none of #214/#205/#213 changes `hubState.ts`; #205 and #213 have no `ModelSelect.tsx` diff at all. Therefore:

- Preserve #216's `hubState.ts` and provider-option block as the baseline.
- Retain #214's skill picker/type additions after rebase; they are not stale provider logic.
- If a rebase exposes historical #205/#213 provider hunks, drop them, but there are no such hunks in the current tip commits.
- Resolve coordination-file conflicts without restoring stale status claims.

### Medium — #216 offers providers that the backend did not report, then clears the model

`ModelSelect.tsx:38-40` unions every key in the static label map with the keys returned by `get_available_models`. That makes all twelve known labels selectable even when their model list is absent. `ConfigPanel.tsx:163-173` then assigns `model: ""` when one of those labels is selected. The comment that the selectable list is driven by what the backend “actually offers” is therefore inaccurate, and the UI can lead a user into an invalid empty-model configuration.

The original empty-dropdown defect is fixed, and the rebased frontend builds, but the fallback policy should be explicit: either show only discovered providers plus the currently saved provider, or provide real fallback model catalogs/validation for static providers. A focused provider-selection regression test is also missing; the #216 commit message itself notes that no test covered the failure.

### Medium — #214 reintroduces synchronous filesystem/settings I/O inside async handlers

`src-tauri/src/core/agent_resources.rs:18-55` opens/snapshots settings and uses `std::fs::read_dir`. Both the async Tauri `get_agent_resources` command and async TCP `GetAgentResources` handler call this helper directly. This replaces the prior `tokio::fs::read_dir` implementation and can block an async runtime worker. Use async I/O or a bounded `spawn_blocking` boundary, consistent with the repository's responsiveness rule. The new test covers listing semantics, not the async boundary.

### Medium — changelog/readiness evidence is incomplete

- #216 has no `[Unreleased]` entry for the desktop provider-dropdown fix.
- #213 has no `[Unreleased]` entry for removing `content-visibility`; the old 1.0.0 changelog text describes the earlier optimization that introduced the behavior, not this corrective change.
- #214's entry is accurate for the role/resource code actually present, but it does not establish completion of #206/#207/#208 or endpoint parity.
- #205's entry accurately describes the shared action and v5 action bumps.
- #205's branch-local evidence says only YAML/static validation was run. The later reconciled table's broader “CI rust/frontend/android green” statement is not supported by that branch record, and GitHub check status was unavailable from this restricted environment. Do not present the broader claim as locally verified without a check-run link/run id.
- #213 has a clean production build here, but no targeted regression or live maximized-WebKit evidence. The CSS diagnosis is plausible and directly removes the unstable intrinsic-size substitution at `scroll-performance.css:8-12`; acceptance still requires the planned desktop live retest.

## Per-change assessment

### #216 / `527c6d1` — desktop provider dropdown

- Correctly restores labels and makes the formerly empty select render options.
- `npm run build` passes in the post-#216/#214 replay (`tsc` + Vite; 103 modules).
- No focused test was added; root has no application test script.
- Static-undiscovered provider behavior needs the policy/validation correction above.
- Missing changelog entry.

### #214 / `feat/android-task-config` at `9c6885c`

- Resource protocol, editable role names, prompt/rule/workflow/skill pickers, desktop skill field, and prompt construction are internally coherent.
- `GetAgentResources` returns the desktop default workspace and the Android client applies that work directory, aligning resource selection with task execution. A late response can still overwrite a just-edited Android work-dir field; recheck this interaction during device acceptance.
- Missing reconciled #206/#207/#208 work and missing endpoint parity are release blockers.
- Post-#216 replay verification performed here:
  - `npm run build`: pass.
  - `cargo fmt --all -- --check`: pass.
  - `cargo test -p tauri-app core::agent_resources -- --test-threads=1`: 1 passed.
  - `cargo clippy -p tauri-app --all-targets -- -D warnings`: pass.
  - Android Gradle could not be independently rerun in this sandbox because Gradle's local daemon/coordination socket fails with `java.net.SocketException: Operation not permitted`, including with `--no-daemon`. The author's recorded JDK-21 `compileDebugKotlin ktlintCheck` pass is therefore not contradicted, but was not independently reproduced here.

### #205 / `ci/sidecar-composite-action` at `96f171c`

- The composite action correctly builds the same seven crates as `tools/release/justfile` and invokes the existing staging script.
- Both GitHub `ci.yml` Rust and `release.yml` desktop jobs call the shared action exactly once; neither retains an inline sidecar build.
- All seven relevant YAML files (action plus six edited workflows) parse successfully with PyYAML.
- Requested GitHub action bumps are consistently applied in the edited GitHub workflows. The untouched Gitea/Forgejo mirrors are outside the explicitly GitHub-only v1.0 release path documented in the packaging handoff.
- No source `ModelSelect.tsx`/`hubState.ts` hunk exists at this tip. Rebase is still required for ancestry and bus conflict cleanup.
- Static validation supports the branch's own claim; it does not independently prove Linux/Windows release execution or the later broad green-CI claim.

### #213 / `fix/213-maximized-scroll` at `705e5ab`

- Removing `content-visibility: auto` and `contain-intrinsic-size: auto 600px` from `.main-content` descendants is a technically credible fix for WebKit scroll-range correction/jumps.
- `npm run build`: pass.
- No source conflict with #216 exists at this tip; rebase is still required.
- Missing changelog, missing PR, and missing live maximized-window regression evidence. Do not close #213 on the build alone.

## 500-LoC rule

All changed hand-authored code/config files are within the hard cap at their reviewed tips. Largest relevant files:

- #214 `src-tauri/src/agent/orchestrator.rs`: 496 lines.
- #214 `src-tauri/src/lib.rs`: 446 lines.
- #214 `src-tauri/src/client/llm.rs`: 430 lines.
- #214 Android files: 109-314 lines; `ModelSelect.tsx`: 211 lines.
- #216 `ModelSelect.tsx`: 204 lines; `hubState.ts`: 80 lines.
- #205 largest edited workflow: 158 lines; composite action: 36 lines.
- #213 CSS: 34 lines.

No line-cap violation found, though #214 leaves `orchestrator.rs` only four lines below the limit.

## Required disposition

1. Rebase #214, #205, and #213 onto current post-#216 `main`; preserve #216 provider logic and only retain #214's non-provider skill UI hunk.
2. Correct PR #214's scope/state: add the actual #206/#207/#208 and endpoint work, or split/relabel it as #209 resource parity and keep the other issues open.
3. Resolve #216's undiscovered-provider/empty-model behavior and add a focused regression test.
4. Restore nonblocking resource enumeration in #214 and add protocol/prompt coverage beyond the single directory-list test.
5. Add accurate `[Unreleased]` entries for #216 and #213; preserve the accurate #214/#205 entries during rebase.
6. Obtain green PR checks and perform the required Android/device and maximized-desktop live acceptance before release sign-off.

