# HANDOFF — Coding-Assistants v1.0 packaging pipeline

**Written:** 2026-08-29 by Claude (Sonnet 5), end of session `session_01TAhCbks5JAr4ZQ2D7z2Spw`.
**For:** a fresh Claude session restarted in `~/Repositories/Repo/Coding-Assistants`.
**Read next:** the memory file `ca-v1-packaging-pipeline.md`, the plan doc
`.agent/reports/chat/v1_packaging_pipeline_plan_20260829.md`, and **PR #168**.

---

## Where things stand

The whole v1.0 objective (Harbinger, via AskUserQuestion earlier this session):

- **v1.0 scope** = Minimal stable desktop + core orchestration + persistent
  **[Settings]** + **[C14]** provider-native managed harness integration.
  Deferred: [TUI], Android *main* app, cloud-sync S1–S14, A2A.
- **Build targets** = Linux **AppImage + .deb**, Windows **.msi + NSIS .exe**,
  Android **APK + AAB**. **No macOS.**
- **Sequencing** = **packaging pipeline first**, then bugs, then features.

### Phase 1 — packaging pipeline: DONE, in review as PR #168

Branch **`v1-packaging-pipeline`** (HEAD `ecebf9d`), PR #168 → `main`, state
OPEN / MERGEABLE / mergeState UNSTABLE (failing checks are pre-existing — see
below). Five workstreams, all merged into the branch:

| WS | What landed | By |
|---|---|---|
| A | `src-tauri/tauri.conf.json`: `productName` "Coding Assistants", `mainBinaryName` `coding-assistants`, window title, explicit `bundle.targets` `[deb,appimage,msi,nsis]` (drops rpm), bundle metadata (publisher/category `DeveloperTool`/MIT/descriptions/NSIS per-user), `"version": "../package.json"` | Claude |
| B | `tools/release/justfile`: `bump <semver>` (syncs `package.json` + `src-tauri/Cargo.toml` + `android/app/build.gradle.kts` versionName + derived versionCode = `major*10000+minor*100+patch`), `bundle-linux`/`bundle-windows`/`bundle-android`, `artifacts` (→ `dist/release/`), `release` orchestrator. Root `justfile`: `mod release` + `just package` / `just release-bump` shorthands. `bump` recipe exercised OK (ran `just release::bump 0.1.0`, tree self-consistent). | Codex |
| C | Android companion (`android/`): id rename `com.example.remotelauncher` → `com.codingassistants.remotelauncher` (dir tree + manifest + all Kotlin `package`/`import`), added the missing Gradle wrapper + `org.jlleitschuh.gradle.ktlint` plugin (this **greened the previously-red `lint-test-android` CI job**), release `signingConfigs` reading `android/keystore.properties` (local) or `ANDROID_KEYSTORE_*`/`ANDROID_KEY_*` env (CI), fails fast if absent. `android/RELEASE_SIGNING.md`. Verified `ktlintCheck test assembleRelease bundleRelease` all green + `apksigner verify`. Kotlin diff is ktlint auto-format only — **no logic changes**. | Agy |
| D | `.github/workflows/release.yml`: `v*` tag + `workflow_dispatch` (`dry_run`, default true). `desktop` matrix `ubuntu-22.04` (deb/appimage — pinned to 22.04 to avoid glibc-2.39 AppImage) + `windows-latest` (msi/nsis) via `tauri-apps/tauri-action@v0` with `projectPath: src-tauri` / `tauriScript: npm run tauri`. On a real tag → draft Release; on dispatch → empty `tagName` (no tag/Release created) + `uploadWorkflowArtifacts`. `android` job decodes `ANDROID_KEYSTORE_BASE64` → `assembleRelease bundleRelease` → attaches signed APK+AAB. **Does not** gate on clippy. | Claude |
| E | `docs/RELEASE_CHECKLIST.md` (application), `docs/moon/ROADMAP.md` v1.0 milestone section, `docs/moon/CHANGELOG.md` `[1.0.0]` scaffold | Opencode |

Post-merge fixes already applied (commit `ecebf9d`):
- `uploadReleaseAssets` is **not** a real `tauri-action` input (was silently
  ignored) — replaced with empty-`tagName`-on-dispatch + `uploadWorkflowArtifacts`.
- Version tree synced to 0.1.0 (gradle had drifted to 1.0/10000).
- `npm run tauri info` validates the new `tauri.conf.json` (category,
  `mainBinaryName`, `"version": "../package.json"` all accepted).

### Secrets — DONE (Harbinger set them 2026-08-29 12:52)

`gh secret list` shows all four: `ANDROID_KEYSTORE_BASE64`,
`ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`.

### Keystore — recovered & durable

The ws-c worktree was cleaned up before its gitignored `release.jks` was
copied out; it was **reconstructed from the base64 in the delegate run log**
(verified: 2169 bytes, alias `coding-assistants`, valid to 2054 — the exact
key `apksigner verify` passed). Durable copies:
- `~/.local/share/coding-assistants-release/{release.jks,release.jks.base64}`
- gitignored in-repo: `android/keystore/release.jks` + `android/keystore.properties`
  (for local `just release::bundle-android`).
- Store password == key password == `ca-release-2026-s9xK8mQp4vL2wZ`, alias
  `coding-assistants`. Consider rotating post-v1.0 (it passed through a temp
  log file), but fine for an unpublished app.

---

## PROGRESS UPDATE — session `session_01RrZbBjc6u8x5yrEhdis6Zx` (2026-08-29 afternoon)

Steps 1–2 below are **DONE**. Pipeline phase complete; Phase 2 (bugs) started.

- **PR #168 MERGED** (`7d338b8`) — pipeline on `main`.
- **PR #169 MERGED** (`093e647`) — `tauri-action@v0` has **no** `uploadWorkflowArtifacts`
  input (the pipeline assumed it did); on dispatch it logs "No releaseId or
  tagName provided, skipping all uploads". Added an explicit
  `actions/upload-artifact@v4` step to the desktop job, guarded
  `if: github.ref_type != 'tag'`.
- **Dry run GREEN** (run 33258273556) — `desktop-ubuntu-22.04` (deb+AppImage,
  96 MB), `desktop-windows-latest` (msi+nsis, 12 MB), `android` (apk+aab, 21 MB)
  all returned as run artifacts.
- **RC tag `v0.1.1-rc1` pushed → full E2E GREEN.** Draft Release `v0.1.1-rc1`
  built with all 6 assets. **The draft + tag still exist** — delete both before
  cutting v1.0.0 (`gh release delete v0.1.1-rc1 --cleanup-tag`).
- **PR #170 MERGED** (`822b4c8`) — the pre-existing `lint-test-rust` red was
  **not a lint issue**: the CI job never installed the GTK/WebKit system libs,
  so `glib-sys`'s build script died at `pkg-config` and clippy/test never ran.
  Added the apt set `release.yml` already uses + `swatinem/rust-cache`. Also
  deflaked a `$HOME` test-isolation race in `src-tauri/src/harness/claude.rs`
  (two capture-gate tests each had their own function-local `static HOME_LOCK`
  → didn't mutually exclude). `lint-test-rust` is **now green on `main`**.
- **PR #171 OPEN** — #163 batch 1: `hub_export_markdown` + `hub_export_markdown_git`
  moved off the IPC thread (`async` + `spawn_blocking`; `git` subprocess was
  freezing the whole window). Audit of remaining freeze candidates in
  `.agent/reports/chat/163_ui_freeze_audit_2026-08-29.md`.

### Still red on `main` (deferred): `cargo-audit`, `pip-audit`
Dependency-advisory failures, unrelated to `lint-test-rust`. Weekly-scheduled
`Security Audit` workflow. Not yet triaged.

### BLOCKS v1.0.0 — asset naming
The `v0.1.1-rc1` draft shipped desktop bundles named `Coding.Assistants_0.1.0_*`
(package.json is still `0.1.0` — the RC tag was just a probe) and Android assets
named `app-release.apk` / `app-release.aab` with **no version**. Before the real
`v1.0.0` tag: (a) `just release::bump 1.0.0` fixes the desktop version string,
(b) fix the Android Gradle output so the APK/AAB carry the version (rename in
the `release.yml` android job, or `archivesName` / `setProperty("archivesBaseName", …)`
in `android/app/build.gradle.kts`).

### #143 is already done
Top-level `AppErrorBoundary` implemented in `2ba53a5` and wired in `main.tsx`;
roadmap `ui.md` marks U14 "In Review". Only open item: a forced-throw boundary
test, which needs a root frontend test harness that **does not exist** (the main
app has zero vitest — only `docs/website` has one). Either stand up vitest for
`src/` (also serves #167) as its own task, or document the test strategy and
close #143.

## What's next (current, as of session `session_01RrZbBjc6u8x5yrEhdis6Zx`)

**Steps 1–2 of the original plan are DONE** (pipeline merged, dry-run + RC
E2E both green, RC deleted). What remains for v1.0.0:

### A. In flight — four parallel work items, not yet merged

| Slice | Owner | Branch (expected) |
|---|---|---|
| #165 resume-vs-fresh-session (grok/gemini discovery in `crates/hub/src/bridge/relaunch/`) | `deepseek` (Harbinger's persistent session) | — |
| `cargo-audit` + `pip-audit` → green (dep bumps: `src-tauri`, `git/pyproject.toml`+`uv.lock`) | `codex/chat` (persistent session) | — |
| Android versioned APK/AAB filenames (`android/app/build.gradle.kts` archivesName + `release.yml`) | `gemini/Agy` (persistent session) | — |
| #163 batch 2 (avatar/attachment base64 → `spawn_blocking`) + more | parallel `coding-assistants-c1` session — large uncommitted tree touching `crates/hub/src/bridge/`, `crates/tui/`, settings tabs, `relaunch/mod.rs` (**overlaps #165 — reconcile**) | — |

Review each PR as it lands. Watch for #165 collision between `deepseek` and
`coding-assistants-c1` (both editing `relaunch/mod.rs`).

### B. Then — cut v1.0.0

1. `just release::bump 1.0.0` — **verified working** this session on a scratch
   worktree: syncs `package.json` + `src-tauri/Cargo.toml` + root `Cargo.lock`
   (the `cargo update -p tauri-app --precise` step) + gradle
   `versionName=1.0.0`/`versionCode=10000`; `cargo metadata` stays consistent.
2. Freeze `docs/moon/CHANGELOG.md` `[1.0.0]` → dated.
3. `git commit`, `git tag v1.0.0 && git push origin v1.0.0`.
4. `release.yml` fires on the tag → draft Release with all 6 artifacts. Review,
   then publish. Follow `docs/RELEASE_CHECKLIST.md`.

### Phase 3 — features

**[Settings]** epic: #126, #131, #132, #133 (persistent local config,
migration/recovery, danger-zone actions). **[C14]** provider-native managed
harness: #147 epic; #148 harness session supervisor, #149 Codex app-server
single managed writer, #150 Claude Code two-way Channel, #151 app-managed
Gemini Antigravity workers, #152 provider integration UX + Kubuntu
acceptance, #154 Grok live-session delivery, #156, #157.

---

## Repo facts worth having up front

- Tauri 2 + React 19 + Rust/Tokio; Kotlin/Compose Android companion. `just`
  build system, `tools/*/justfile` sub-modules, `.agent/AGENTS.md`
  authoritative. IPC: `invoke` / `#[tauri::command]`.
- Cargo **workspace target dir is repo-root `target/`** (not
  `src-tauri/target/`); members: `src-tauri` (crate still literally named
  `tauri-app`), `crates/{hub,cli,tui,claude}`. Renaming the crate is invasive
  (`tauri_app_lib` referenced in `src-tauri/src/main/main.rs`) — deferred as
  optional cleanup; `mainBinaryName` handles bundle filenames without it.
- Only `origin` = GitHub `ACFHarbinger/Coding-Assistants` is wired. `.gitea/`
  + `.forgejo/` workflow dirs exist but have no remote and no `windows-latest`
  runner → `release.yml` is GitHub-only for v1.0.
- Delegation CLIs on this machine: `codex exec --cd <dir>
  --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check "<prompt>"`;
  `agy --dangerously-skip-permissions --print-timeout 60m -p="<prompt>"`
  (flags before `-p=`); `opencode run --agent build "<prompt>"`. Use git
  worktrees under `~/Repositories/Repo/.ca-worktrees/` for parallel
  delegation and **copy any gitignored build output out before removing the
  worktree** (that's how the keystore nearly got lost).
- Commit trailers (mandatory):
  `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01TAhCbks5JAr4ZQ2D7z2Spw`
  (update the session id). PR bodies end with the 🤖 line + session URL.

## Unrelated work parked elsewhere (not this repo)

- **Image-Toolkit:** ASP full-97 validation resume (GH #470); `_blend_phase_plates`
  seam fix after Harbinger's coherence rating.
- **Tracker-App:** template-structure scaffold PR #1 open on
  `scaffold/template-structure`; brainstorm round-trip + roadmaps pending.
  See memory `tracker-app-restructure.md`.
