# Release 1.0.0 — BLOCKER: candidate is not a cut release

Recorded: 2026-09-01 by Claude (team lead / issue truth). Release acceptance
is **PAUSED** by owner decision until section D below is done.

## Finding (blocker-class, belongs on #199 and #192)

The 1.0.0 artifacts in `release/` were **not built from a tagged release
commit**, and the existing `v1.0.0` tag points at the wrong tree.

| Check | Command | Result |
|---|---|---|
| Tag vs candidate distance | `git describe --tags 844b5d1` | `v1.0.0-18-g844b5d1` — tag is **18 commits behind** |
| Candidate on `main`? | `git branch --contains 844b5d1` | **No.** Only `feat/model-effort-selection` |
| `main` ancestor of candidate? | `git merge-base --is-ancestor main 844b5d1` | Yes — linear, **fast-forwardable** (no divergence) |
| `origin/main` ancestor of candidate? | `git merge-base --is-ancestor origin/main 844b5d1` | Yes |
| What `v1.0.0` points at | `git rev-parse v1.0.0^{}` | `41c47cf` (PR #175 merge, 2026-08-29) — pre-dates the creative-tool MCP suite |
| GitHub release state | `gh release view v1.0.0` | **Draft** (not public), author `github-actions[bot]`, `untagged-…` URL, 6 assets from the 2026-08-29 CI run |
| Draft asset naming | — | Draft has `…_amd64.AppImage`; local `release/` has `…_amd64.appimage` (different case) → **local artifacts are not the CI draft assets; provenance unclear** |
| Build tree cleanliness | `git status` at inventory | `-dirty` — `.gitignore` +3 lines (`release/` ignore, no trailing newline). Cosmetic; does not affect build output |
| Changelog freeze | `docs/moon/CHANGELOG.md` | `## [Unreleased]` still holds Fixed / Performance / **Packaging** ("bundle all seven Creative Tools MCP sidecars") — i.e. the candidate's own headline change is **not under `## [1.0.0]`** |
| Version strings | `package.json`, `src-tauri/Cargo.toml`, `android/app/build.gradle.kts` | All `1.0.0` / `versionCode 10000` — **consistent, OK** |

### The 18 commits between the tag and the candidate are real source

Not noise — the entire creative-tool MCP bridge suite and more:

```
844b5d1 feat(settings): add harness model and effort defaults      <- candidate
768fee0 feat(release): bundle creative MCP sidecars
f8d8dac feat(settings): responsive creative MCP controls (#187)
fed446e chore(fmt-and-ci-hygiene): workspace fmt+CI widening + versioned Android artifacts (#188)
29f67bb fix(presence): observed sessions show present (#165) (#189)
340dd95 feat(mcp): C-9a per-workspace registration for creative bridges (#187)
cc07383 feat(mcp): OpenToonz viability spike (#186)
aed234c feat(mcp): Unity bridge (#185)
40744b7 feat(mcp): Unreal Engine bridge (#184)
46a34b9 feat(mcp): Aseprite bridge (#183)
06921ba feat(mcp): Godot bridge (#182)
0d75fc4 feat(mcp): Krita bridge + shared app_link (#180)
5bac287 feat(mcp): Blender bridge (#179)
c7f451e feat(mcp): hub::mcp client-agnostic MCP config rendering (#178)
e8f3ea8 feat(mcp): extract mcp-core stdio server (#177)
... (+ f8d8dac/768fee0/e1d9a9b merge already listed)
```

`d998f94` sits one commit past the candidate on the branch tip and is
docs-only (the #192–#199 bus record).

## Why this blocks

`docs/RELEASE_CHECKLIST.md` §2 requires an **annotated `vX.Y.Z` tag on
`main`**; §1 requires the changelog frozen under the version heading before
tagging. Neither holds. Publishing `v1.0.0` as-is would ship a tree that is
missing every creative-tool sidecar the 1.0.0 changelog advertises, and the
local `release/` binaries cannot be tied to any commit.

## Remediation (owner executes git/tag/build; Claude updates issues + checklist)

**A. Land the candidate on `main`**
- Option 1 (governance-consistent, changes SHAs): push
  `feat/model-effort-selection`, open PR → `main`, review-lead (Codex)
  reviews, merge. Tag the resulting merge.
- Option 2 (fastest, stable SHA): `git checkout main && git merge --ff-only d998f94`
  (or `844b5d1` if the docs commit is excluded), then `git push origin main`.
- Recommend Option 1 — every one of the 18 prior commits landed via a
  numbered reviewed PR; the release commit should not be the exception.

**B. Freeze the changelog** (commit on `main` before tagging)
- Fold `## [Unreleased]` (Fixed / Performance / Packaging) into
  `## [1.0.0] - 2026-09-01` (re-date from 2026-08-29). Keep history.

**C. Retag** (safe — the GitHub release is still a draft, no public consumers)
- `git tag -d v1.0.0 && git push origin :refs/tags/v1.0.0`
- Delete the stale draft release: `gh release delete v1.0.0 --yes` (leaves the tag namespace clean for the workflow).
- `git tag -a v1.0.0 -m "v1.0.0" <main HEAD> && git push origin v1.0.0`

**D. Rebuild + re-verify artifacts**
- Tag push triggers `.github/workflows/release.yml` → fresh 6 artifacts →
  new draft release. (Or run `just release::artifacts` locally from the
  clean tag for the Linux set.)
- Replace `release/*` with the rebuilt artifacts.
- Recompute all 6 SHA-256. Update: #192 artifact table, checklist §1
  (`Journal/Personal/Journals/RELEASE_CHECKLIST_CA.md` — outside this repo),
  and #199.
- New candidate commit = the tagged `main` HEAD. Update #192 "Candidate
  commit" and every child's "candidate commit" field.

**E. Only then** clear the pause and start acceptance from checklist §3.

## Not in scope of this fix
- The `.gitignore` `release/` change: legitimate, but its owner said not to
  touch it without direction. Fold it into a real commit on `main` during
  step A (and add the missing trailing newline) so the release tree is clean.
- Windows unsigned / Android signing caveats — already documented in
  `docs/RELEASE_CHECKLIST.md` §7; carry into #199 known-caveats, not part of
  this blocker.
