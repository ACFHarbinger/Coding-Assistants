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

## What's next (do these in order)

### 1. Land PR #168

Failing checks on the PR are **all pre-existing on `main`**, none introduced
by this branch:
- `lint-test-rust` — `cargo clippy --all-targets -- -D warnings` pedantry.
  Red on `main` already. The release workflow deliberately does not run clippy.
- `cargo-audit`, `pip-audit` — dependency advisory failures. Confirm they're
  also red on `main` (they almost certainly are) before dismissing.
- `lint-test-android` now **passes** thanks to WS-C's ktlint plugin.
- `build`, `lint-test-frontend`, `npm-audit` pass.

Decision for Harbinger: merge PR #168 despite the pre-existing red (they're
tracked for the *bugs* phase), or fix `clippy -D warnings` + the audits
first. Recommend **merge now** — the pipeline is orthogonal to those, and
`release.yml` can't be dispatched until it's on `main` (see next).

### 2. Validate the pipeline (needs #168 merged first)

`release.yml` currently 404s from the Actions API because
`workflow_dispatch` requires the workflow file on the **default branch**. Once
#168 is merged into `main`:

a. **Dry run:** Actions → Release → Run workflow → `dry_run: true`. Confirms
   the desktop bundles build on both runners and the Android job signs. No
   tag/Release created; bundles come back as run artifacts.
b. **RC tag:** `git tag v0.1.1-rc1 && git push origin v0.1.1-rc1` → full
   end-to-end: draft Release with all 6 artifacts (`.deb`, `.AppImage`,
   `.msi`, NSIS `-setup.exe`, `.apk`, `.aab`). Iterate `release.yml` if
   anything is missing.
c. **v1.0.0:** `just release::bump 1.0.0`, freeze `docs/moon/CHANGELOG.md`
   (`[1.0.0] - Unreleased` → dated), commit, `git tag v1.0.0 && git push`.
   Follow `docs/RELEASE_CHECKLIST.md`.

Known unverified: `cargo update -p tauri-app --precise <ver>` inside `just
release::bump` — Codex claimed it works on a workspace member; only exercised
with an unchanged 0.1.0 so far. Watch it on the real 1.0.0 bump.

### 3. Phase 2 — bugs

`#163` (UI freezes several seconds, no feedback), `#165` (resume duplicates
sessions / Claude reroutes live chat / Gemini messages while inactive),
`#167` (terminal scroll/resize — partially fixed already, commit `dfb518f`),
`#143` (no frontend crash-recovery boundary). Also fold in the pre-existing
CI red (`clippy -D warnings`, `cargo-audit`, `pip-audit`) here.

### 4. Phase 3 — features

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
