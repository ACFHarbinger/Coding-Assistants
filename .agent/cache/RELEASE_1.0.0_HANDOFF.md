# Release 1.0.0 handoff — Claude and Codex

Updated: 2026-09-01 WEST. Read this with `AGENT_BUS.md` before beginning
release work. Claude remains the team lead and owner of GitHub issue truth;
Codex performs review/governance and reports verified results to Claude.

## Current state

- Candidate repository commit: `844b5d1c5990538940a2bfdbfd9f61572699e747`.
- Repository describe at inventory time: `v1.0.0-18-g844b5d1-dirty`.
- Do not alter the existing unstaged `.gitignore` change as part of release
  work without its owner's direction.
- No artifact has been installed or live-tested yet. Checklist preparation and
  fixture validation are complete, not release acceptance.

## Source of truth and paths

- Complete manual checklist:
  `$REPOSITORIES_ROOT/Journal/Personal/Journals/RELEASE_CHECKLIST_CA.md`.
  From this repository it is also reachable as
  `../../Journal/Personal/Journals/RELEASE_CHECKLIST_CA.md`. It has the host
  record, six artifact names/sizes/SHA-256 values, fixtures, acceptance
  criteria, and final sign-off fields.
- Release artifacts: `release/`.
  - `Coding.Assistants_1.0.0_amd64.appimage`
  - `Coding.Assistants_1.0.0_amd64.deb`
  - `Coding.Assistants_1.0.0_x64-setup.exe`
  - `Coding.Assistants_1.0.0_x64_en-US.msi`
  - `coding-assistants-companion-release.apk`
  - `coding-assistants-companion-release.aab`
- Fixtures: `release/fixtures/` (ignored by Git).
  - `workspace/`: disposable coding workspace and `.agent` resources.
  - `workspace/.agent/mcp_config.valid.json`: inert valid MCP config.
  - `workspace/.agent/mcp_config.invalid.json`: intentionally malformed JSON.
  - `attachments/`: plain-text and SVG attachment inputs.
  - `remote/invalid-requests.jsonl`: malformed/unknown TCP protocol inputs.
- Upstream release workflow/checklist: `.github/workflows/release.yml` and
  `docs/RELEASE_CHECKLIST.md`.
- Changelog: `docs/moon/CHANGELOG.md`.

## Required GitHub issue work — do this before declaring release readiness

Claude must create (or locate and update) a release-tracking parent issue for
the 1.0.0 candidate, then create/link focused issues for the following manual
verification groups. Add the candidate commit, artifact SHA-256 values, test
environment, checklist path, evidence, and pass/fail/blocker result to each.

1. Linux AppImage and Debian install/launch/upgrade/uninstall acceptance.
2. Windows MSI and NSIS install/launch/upgrade/uninstall acceptance.
3. Android APK/AAB install, signing, and remote-control acceptance.
4. Desktop task lifecycle, approvals, Hub/CLI persistence, and privacy
   acceptance.
5. Creative-tool MCP sidecar matrix (Blender, Krita, Godot, Aseprite, Unreal,
   Unity, OpenToonz); unavailable host applications are explicit N/A results.
6. Documentation website deployed-site and accessibility/privacy acceptance.
7. Publication/sign-off: artifact metadata, release notes, known caveats, and
   post-publication smoke test.

Do not close an issue because a build exists. Close it only after the matching
checklist section has live evidence. Link any defect found during testing to
the parent issue and leave the release blocked until disposition is recorded.

## Execution order

1. Claude creates/updates the issue set above and assigns owners according to
   `AGENT_BUS.md`.
2. Copy `release/fixtures/workspace` to a temporary writable directory. Never
   point an agent task at the checked fixture source or a production workspace.
3. Run the checklist's isolated desktop/Harness/Hub/CLI acceptance. Use only
   disposable keys and local data. Record errors without secrets or personal
   paths.
4. Test each creative bridge with a disposable host project; record N/A where
   the host/runtime is unavailable.
5. Test Android only on an actual Android 7.0+ device on the same LAN. At
   inventory time no `adb` command or device was available.
6. Test Windows artifacts on supported Windows machines or VMs. Do not claim
   Linux-side file inspection as Windows acceptance.
7. Update the checklist, GitHub issues, release notes/changelog, and only then
   request publication/sign-off.

## Guardrails

- The app uses local Hub data, providers, MCP bridges, terminals, and a TCP
  remote-control server. Treat credentials, private messages, workspaces, and
  network addresses as sensitive; redact them from evidence.
- Use targeted checks only unless the team lead explicitly authorizes a full
  suite. Do not run large test/benchmark workloads concurrently.
- Preserve installed prior releases and existing user profiles; perform tests
  with isolated profiles and disposable workspaces.
- The checklist intentionally records Wi-Fi only as connected, not its SSID.
- Report all scoped changes and verification evidence back to Claude before
  calling a release check complete.
