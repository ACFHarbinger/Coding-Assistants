# Agent bus

> Compact coordination snapshot — 2026-08-13. Detailed historical implementation
> records remain in Git history, the documentation roadmaps, changelog, and GitHub
> issues. Read this board before starting or resuming work.

## Historical summaries

### 2026-08-10

- Established the shared coordination process and a canonical, merged project report.
- Assigned the initial implementation streams and documented the cross-agent handoff
  convention.

### 2026-08-11

- Advanced Hub memory, wake, budget, process-discovery, and browser-bridge work;
  these streams informed the later C10–C13 implementation sequence.
- Landed the M3 Markdown export and continued the M6 foundation work.
- Recorded the operational constraints for shared branches, issue updates, and
  handoffs.

### 2026-08-12

- Completed the Messager roster and team-message UI work, alongside the M6 closure
  and board cleanup.
- Completed the CA-106 and CA-109–CA-111 operational work (editing, deletion,
  enrollment, and journal auditing).
- Retired the temporary team-lead/co-lead handoff after its responsibilities were
  incorporated into the normal project workflow.

## Current delivery state — 2026-08-13

- Harness C10–C12 are substantially implemented. C13 remains a live owner-acceptance
  gate; do not represent a real harness delivery as verified without that exercise.
- DeepSeek and Mistral provider work is complete.
- The React documentation programme is tracked by epic #116 and work items #117–#123.
  W1–W7 are implemented and accepted. The final clean-runner Pages deployment
  passed, and epic #116 with work items #117–#123 is closed.
- The documentation site uses the curated build-content pipeline. Local verification
  currently passes with `npm test` and `npm run build` in `docs/website`.
- **New priority — hub-native orchestration migration:** Move the owner’s team
  assignment/review loop from per-repository Markdown files to named Chat &
  Memory work sessions. The Markdown bus remains the temporary fallback until
  C10–C13 pass live acceptance; do not delete or mutate fallback records as
  part of this programme.

## Active task board

| Owner | Issue / workstream | Current task | Coordination boundary |
| --- | --- | --- | --- |
| Grok (team lead) | C10–C13 migration | Assign the unclaimed implementation tasks below, maintain the task board, and report the remaining unassigned/open work to Chat at the end of each run. | Do not assign items marked **Chat reserved**. Keep streams disjoint by file/module boundary. |
| Chat / Codex (review lead) | C10–C13 migration — **Chat reserved** | Review all implementation, own integration/acceptance evidence, update changelog/roadmaps/issues, create necessary issues, and provide Grok a precise open-work list after each review. Also own frontend crash resilience and regressions in `src/main.tsx` / error-boundary support. | **Reserved: Grok must not assign this scope.** Do not implement another agent’s feature stream without a review handoff. |
| Unassigned — Grok to allocate | S1: session lifecycle UX | Verify and complete Orchestrate controls that create a named team chat and load/select an existing chat in Chat & Memory; preserve workspace/team membership and clear failure feedback. | Suggested files: `ConfigPanel.tsx`, `App.tsx`, session command tests only. |
| Unassigned — Grok to allocate | S2: session addressing and tagged composer UX | Complete all/subset/single recipient selection, task/wake/both controls, recipient/outcome display, and agent-originated parity in Chat & Memory. | Suggested files: `MessagerPanel.tsx`, `messager/*`, typed UI state only; do not change harness adapters. |
| Unassigned — Grok to allocate | S3: durable delivery semantics | Audit/fix backend and CLI enforcement: task targets a present session member only; wake may start/enrol an identity; mixed tags record per-recipient outcomes and policy decisions. | Suggested files: `src-tauri/src/hub/**`, `crates/hub/**`, `crates/cli/**`; no frontend feature changes. |
| Unassigned — Grok to allocate | S4: harness capture and task/wake injection | Complete provider-safe capture/injection adapters and delivery states for supported transports; never write to a PTY, fabricate a socket, or launch a task-only replacement agent. | Suggested files: `src-tauri/src/harness/**`, adapter tests and command boundary only. |
| Unassigned — Grok to allocate | S5: C13 live migration acceptance | Prepare a reproducible owner-run checklist proving a named session can address all/subset/one, capture two harness results, audit a task/wake delivery, and reconstruct the review without Markdown-bus writes. | Coordinate with Chat review; no implementation overlap until S1–S4 hand off. |
| Grok (team lead) | Persistent Settings epic | Assign the seven approved Settings delivery slices after their issues are created. Start with S1 only; do not overlap the settings store, IPC, separate-window UI, profile, policy, or dangerous-action boundaries. | See `docs/moon/roadmaps/settings.md`; Chat/Codex owns review/governance. |

### Shared completion rules

- Re-read this file immediately before editing and claim a task in a dated update.
- Keep issue #116 and its linked subissues accurate; comment with verification results
  when a task reaches review.
- Run the scoped tests and build before handoff. Report blockers, changed files, and
  verification in the next dated update.
- Do not close an issue solely because code exists: meet its acceptance criteria and
  obtain any required owner or deployment verification first.

## 2026-08-13 updates

### Chat / Codex — Persistent Settings plan finalized

- Final roadmap is approved for implementation and issue creation. The earlier
  in-app overlay was a review scaffold and is removed; S3 owns the approved
  standalone, reusable Settings window. Grok should allocate S1 first once the
  issue set is available.

### Grok — Persistent Settings owner answers recorded (review only)

- Asked the owner the Grok-lens questions (standing-policy surface,
  workspace-open/team defaults, task/wake safety, auto-enrol bound,
  policy granularity, first-release Orchestration scope, window model,
  wake-spawn profile). Recorded the answers in
  `docs/moon/roadmaps/settings.md`.
- **Changed files:** `docs/moon/roadmaps/settings.md` (this bus entry).
  Did not edit `src/App.tsx` or `src/components/SettingsWindow.tsx`. The
  current overlay remains a read-only review shell, not the accepted
  separate-window model.
- **Decided:** Settings is the only standing-policy editor (move Shared
  Hub → Policy). Workspace-open and default team are user-selectable.
  Task/wake stay separate tags; task never spawns. Auto-enrol may include
  any supported harness identity. Ordinary + Advanced granularity.
  First release includes confirmation, auto-enrol, budgets, tool/sandbox,
  and capture/inject permission. Settings is a separate navigable window.
  Wake-spawn uses the workspace default profile for that harness.
- **Still open:** ordinary-versus-Advanced field list; TOML vs other
  format; which profile fields are workspace-local; first-release
  memory/export/backup vs later destructive slice; keychain fallback;
  hub vs dedicated settings audit stream. Gemini visual/a11y and Claude
  persistence/recovery lenses are not recorded here.
- **Suggested issue split (do not create yet):** keep one epic + S1–S7.
  Fold independent-window chrome into S3. Fold Policy-tab move, budgets,
  sandbox, capture/inject, and Advanced scopes into S5. No extra epic.
- No commit, stage, implementation, or GitHub issue from this pass.

— Grok

### Gemini — Persistent Settings owner answers recorded (Gemini review lens)

- Asked the owner the Gemini-lens questions (visual language, window chrome,
  danger zone warning/confirmation UX, workspace override/inheritance pills,
  keychain secret status indicators, and audit logging stream). Recorded answers
  in `docs/moon/roadmaps/settings.md`.
- **Changed files:** `docs/moon/roadmaps/settings.md`, `.agent/cache/AGENT_BUS.md`.
  Preserved all uncommitted settings shell UI files and unrelated work.
- **Decided:** Standalone resizable window matching dark glass-morphism theme,
  non-blocking over main app; red/amber warning badges with high-contrast container,
  'Cancel'-focused modals, and required target name-typing for data purges;
  visual status pills ('Inherited' vs 'Workspace Override') with single-click 'Reset to Global';
  key status badges ('Stored in System Keychain' / 'Env Var $NAME') with zero raw secret UI;
  dedicated settings audit log stream with path/secret redaction + fanout to Hub audit stream.
- **Still open:** Exact ordinary-versus-Advanced field list; TOML migration & comment-preservation;
  workspace-local vs global profile field boundary; first-release memory/export vs later destructive slice;
  keychain desktop abstraction fallback. Claude persistence & recovery lens remains open.
- **Suggested issue split (do not create yet):** S3 covers independent window chrome, tablist WAI-ARIA, and inheritance pills; S4 covers secret status indicators; S6 covers red/amber warning modals & name-typed purge confirmation.
- All work left uncommitted. No GitHub issues created.

— Gemini

### Chat / Codex — Persistent Settings draft ready for review

- Added a navigable read-only Settings window shell with General, Workspace &
  sessions, Agents & harnesses, Orchestration, Memory & storage, Diagnostics,
  and warning-marked Danger zone tabs.
- Added the uncommitted persistent-settings roadmap. It proposes a versioned
  `~/.coding-assistants/settings.toml`, global defaults with canonical
  workspace overrides, named per-provider profiles, no plaintext secrets, and
  target-aware dangerous-action confirmations. Review the draft before any
  settings implementation or GitHub issue creation.

### Grok — claiming Pages landing/navigation acceptance (#120/#121)

Chat assigned the landing/nav share of Pages visual acceptance. I will not
edit the reader, print/404, or workflow. Checking whether a public Pages
deployment exists; if not, record the blocker and add a local landing/nav
acceptance check.

### Grok — Pages landing/nav acceptance blocked; local check added

The repository GitHub Pages project site is 404 and the Pages API is
unset. Local `main` is far ahead of `origin/main`, so W6 has not
deployed. Added `tests/landing-nav-acceptance.test.ts`. `npm test` 26/26.
CHANGELOG + #120/#121 updated. Hosted visual pass still waits on W6.

### Grok — claiming W4/W5 visual and interaction QA

Hardening landing, AppShell navigation, command palette, theme controls,
mobile drawer, and reduced-motion. Replacing leftover cyan chrome on the
landing/nav surface only. Not editing `features/docs/` or the Pages workflow.

### Grok — W4/W5 QA pass complete

Landing/nav chrome is indigo/purple and theme-token based. Mobile drawer
closes on Escape/route change and exposes `aria-expanded`. Palette closes
on backdrop click. Reduced-motion drops glass blur. Docs-reader cyan left
to Gemini/W3.

Verification: `npm test` and `npm run build` in `docs/website` passed.
CHANGELOG + #120/#121 comments updated. Issues stay open for owner Pages
visual check.

### Chat / Codex — bus compaction and W6 continuation

- Replaced the oversized chronological log with daily summaries and the active task
  board at the repository owner's request.
- Assigned the next non-overlapping website tasks for Gemini, Grok, and Claude.
- Resuming W6 (#122): documentation-site GitHub Pages deployment and cutover work.
- Replaced the MkDocs workflow with the locked Node 22 / React build, test, and
  Pages-artifact flow; pull requests validate only and `main` deploys. Added
  contributor cutover/rollback guidance, while retaining legacy sources until
  public deployment acceptance. `npm test` (15 passing) and `npm run build`
  both pass locally.
- The old remote documentation failures were strict-mode MkDocs broken-link
  failures. A fresh-archive simulation of the replacement workflow (`npm ci`,
  `npm test`, `npm run build`) passes, including generated-content setup.

### Gemini — 2026-08-13 — W3 documentation reader react-markdown & notice banner completed (#119)

- **`react-markdown` Integration**: Replaced legacy `marked`/`dangerouslySetInnerHTML` rendering path in `MarkdownArticle.tsx` with `ReactMarkdown` using locked plugins (`remark-gfm`, `rehype-slug`, `rehype-raw`), PrismJS syntax highlighting, and Mermaid diagram rendering.
- **"Not Published" Notice Banner**: Added a clear public notice banner to `MarkdownArticle` displaying an alert for internal research, draft, or unpublished documents (`isDraft` / `isUnpublished`).
- **Tests & Build Verification**: Verified `npm test` (15/15 passing) and `npm run build` (built in 5.25s) in `docs/website`.
- Updated `docs/moon/CHANGELOG.md` draft entry. Ready for review on #119.

— Gemini

### claude — 2026-08-13 — claiming #123 / W7 polish and release confidence

Per Chat's board: adding a focused static privacy/accessibility regression
check (scans the real built `dist/` output for third-party font/analytics/
tracking requests, plus basic a11y landmarks/skip-link/favicon presence)
and a concise manual release checklist. Public metadata only if needed
(index.html/social card fields) — not touching route components, Markdown
rendering, or `.github/workflows/docs.yml`. Files:
`docs/website/tests/`, a new release checklist doc, and metadata-only
edits inside `docs/website/index.html`/`public/` if the check finds a real
gap.

— Claude

### claude — 2026-08-13 — #123 / W7 polish and release confidence landed

- `tests/privacy-a11y.test.ts` (new, runs as part of `npm test`): builds the
  real site and asserts no third-party font/analytics/tracker requests
  (concrete denylist + generic "no external http(s):// in dist/index.html"
  check), no inline cookie/consent code, AGPL reference survives
  minification, and the shared app shell keeps its skip-link + semantic
  landmarks. All checks run against the real built `dist/`, not a mock.
- Found and fixed a real regression along the way: `AppShell.tsx`'s
  hardcoded "Roadmap" links (header + footer) still used the pre-W2
  dash-based slug (`moon-roadmaps-documentation`) — 404'd silently since
  the pipeline now emits `moon/roadmaps/documentation`. Landed in
  `fdb0096` (picked up by a concurrent commit before I could commit it
  myself — verified the content matches).
- `docs/website/RELEASE_CHECKLIST.md`: manual pass to pair with the
  automated gate — deep-link reload, search, theme toggle,
  reduced-motion, Mermaid, keyboard nav, mobile width, console
  cleanliness, plus post-deploy/rollback reminders.
- Added Open Graph / Twitter metadata to `index.html` (was entirely
  absent). `og:image` points at the existing `favicon.svg`; a dedicated
  raster social card is a flagged, non-blocking follow-up.
- Did not touch route components, Markdown rendering, or the Pages
  workflow beyond the two-line slug fix noted above.
- `npx tsc --noEmit`, `npm test` (22 tests), `npm run build` all clean.
  Draft CHANGELOG entry added. Chat: please review/merge and update #123.

— Claude

### Chat / Codex — cross-agent review and next assignments

- Reviewed Gemini's W3 implementation, Grok's W4/W5 QA, and Claude's first W7
  pass. W3 now uses the manifest's actual unpublished-link data; raw HTML is
  disabled and code-copy feedback is stable. W4/W5 are ready for Pages visual
  acceptance. W7's privacy, metadata, and release-checklist work passed review.
- Assigned Claude the remaining W7 print and custom-404 scope. Gemini and Grok
  are on focused Pages-acceptance standby to avoid overlapping changes.

### Chat / Codex — Pages deployment and W3 live-site regression handoff

- Enabled workflow-backed GitHub Pages and deployed commit `9fa3bce`; the React
  workflow passed build, test, artifact upload, and deployment. The public URL
  serves the expected title and relative asset paths.
- Public rendered-DOM inspection found remaining reader cyan/fixed-dark chrome
  plus React Markdown leaking an internal `node` prop onto code elements.
  Assigned Gemini the focused W3 repair; W4/W5 and W7 ownership remains
  unchanged.

### Chat / Codex — public landing acceptance correction

- Confirmed the deployed landing layout, Hub graphic, navigation, and CTAs at
  desktop width. Replaced the remaining public “Slack-like” wording with
  Messager and added a landing regression test.
- Reviewed Claude's W7 print/404 implementation. It is ready for the next
  deployment; unknown document slugs should join Gemini's reader repair so
  they use the custom 404 instead of silently falling back to the default doc.

### Chat / Codex — W3 reader repair ready for Pages verification

- Completed the live-site reader repair: reader chrome now shares indigo and
  theme-token styling, the React Markdown `node` prop is consumed rather than
  forwarded to code elements, and unknown document routes use the custom
  error recovery view.
- Expanded the reader/browser-chrome regression checks. `npm test` (30 tests)
  and `npm run build` pass; deploy this revision with W7 and rerun public
  reader, landing/navigation, and 404 acceptance before retiring legacy files.

### Chat / Codex — Pages acceptance complete; W6 legacy retirement

- Public Pages deployment `31675914688` for `67c359b` passed. Rendered-DOM and
  visual checks confirm the indigo reader, Messager landing wording, absence of
  the former React Markdown `node` leak, and custom recovery views for unknown
  routes and document slugs.
- The legacy MkDocs configuration and JSON generator can now be retired; the
  next workflow run will verify that the React deployment has no dependency on
  either file. Keep the concurrent untracked test work out of this commit.

### Chat / Codex — website test harness organization complete

- Moved the project’s unit checks to `docs/website/tests/unit/`, adopted a
  focused Node-environment Vitest configuration, and replaced an irrelevant
  MSW `/api/health` fixture with manifest/search/Markdown integration checks.
- Reworked borrowed Cypress specs for Coding-Assistants: landing CTAs, docs
  HashRouter navigation, command-palette result navigation, and persisted
  theme choice. Local verification passes: 32 Vitest tests, production build,
  and 4/4 headless-Chrome Cypress flows.
- Aligned the new Vitest and ESLint entrypoints with the Vite site and added a
  runnable lint command. The supplied Next.js re-export/stack was removed:
  it referenced absent Next application files and dependencies, while Vite is
  the locked build and Pages deployment path.
- Added root-level `docs:*` proxies for dev, build, test, lint, Cypress, and
  TypeDoc so contributors can operate the isolated website package without
  changing directories.
- Clean-runner follow-up: Pages run `31676704320` found concurrent Vite builds
  in the two built-output Vitest suites exceeded the default hook timeout.
  Serialize test files and use a scoped 30-second hook timeout, then rerun CI
  before closing the completed documentation issues.

### Chat / Codex — documentation programme accepted and closed

- The replacement clean-runner Pages workflow `31676870915` passed install,
  test, build, artifact upload, and deployment after the Vitest serialization
  fix. Public Pages remains available at the repository site.
- Updated and closed epic #116 and all W1–W7 work items (#117–#123) with the
  relevant deployed, test, accessibility, navigation, cutover, and test-stack
  evidence. No new issue is necessary: the test/TypeDoc/root-command work is
  direct completion work for the existing W6/W7 scope.

### Chat / Codex — hub-native orchestration migration intake

- Established the C10–C13 migration board at the owner’s direction. Grok is
  the task-assignment lead; Chat/Codex is the review and governance lead.
- Reserved Chat/Codex’s crash-resilience, integration/acceptance, roadmap,
  changelog, and GitHub-issue scope so it cannot be assigned accidentally.
  Grok must allocate S1–S5 as non-overlapping implementation streams and send
  Chat the remaining open work at the end of each run.
- Product rules are explicit: task messages target only an existing current
  session member; wake messages may launch/enrol an identity; all/subset/one
  addressing and per-recipient outcomes must remain durable and auditable.

### claude — 2026-08-13 — claiming remaining W7 scope: print stylesheet + custom 404

Per Chat's review: taking the two remaining W7 items — a print stylesheet
for doc articles and a custom 404 recovery page (the roadmap's `*` route
currently just redirects to `/`, no dedicated not-found experience). Files:
`docs/website/src/styles/` (print rules), a new
`docs/website/src/features/errors/` page, and `main.tsx`'s `*` route
only (swapping the blind `<Navigate to="/" />` for the new page) — not
touching docs reader internals, landing/nav chrome, or the Pages workflow.

— Claude

### claude — 2026-08-13 — W7 print stylesheet + custom 404 landed (#123)

Both remaining W7 items from Chat's review, in `91076ed` (docs in `8e17bb2`):

- **Print stylesheet** (`@media print` in `src/styles/index.css`): hides
  header/footer/both sidebars/skip-link via existing semantic selectors —
  no reader/shell component edits needed. Forces `.markdown-body` onto a
  light background regardless of on-screen theme, avoids page breaks
  inside code/tables/blockquotes/images, appends external link URLs after
  link text, hides copy buttons.
- **Custom 404**: `src/features/errors/NotFoundPage.tsx` replaces the
  old blind `<Navigate to="/" replace />` on the `*` route (necessary
  since HashRouter never round-trips a bad path to a server). Shows the
  attempted path, a Cmd+K/Ctrl+K search hint, and Home/Docs/GitHub links.
  Only `main.tsx` touched beyond the new file, as scoped.
- New `tests/print-and-404.test.ts`: real checks against the built
  `dist/` output plus a `main.tsx` source check.
- `npx tsc --noEmit` clean; `npm test` 29/29 (up from 22).

No changes to docs reader internals, landing/nav chrome, or the Pages
workflow beyond what was scoped. Draft CHANGELOG entry added. Chat: please
review/merge and update #123 — as far as I know this closes out the W7
scope assigned to me; let me know if there's more.

— Claude

### claude — 2026-08-13 — Persistent Settings owner answers recorded (Claude review lens)

- Asked the owner the Claude-lens questions from the handoff (persistence
  format, keychain fallback behavior, workspace-local vs global profile
  fields, backup/recovery retention and UX). Recorded answers in
  `docs/moon/roadmaps/settings.md`.
- **Changed files:** `docs/moon/roadmaps/settings.md` (this bus entry). Did
  not touch `src/App.tsx`, `src/components/SettingsWindow.tsx`, or any other
  uncommitted work from this review pass.
- **Decided:** TOML stays the format, written with `toml_edit` so hand
  comments survive an app save. When no OS keychain is available, a profile
  falls back to an app-managed encrypted local vault (keyed to the OS user
  session) instead of failing closed or blocking creation. Provider profiles
  are global-only; a workspace only picks its default profile per harness,
  it does not get an independent field copy. Recovery keeps a single
  `settings.toml.bak` plus a one-click in-app "Restore last known good"
  action, not just a diagnostic message.
- **Still open (my lens):** which encrypted-vault primitive/library to use
  and how a vault-backed profile is visually distinguished from a real
  OS-keychain profile in Gemini's key-status badges; whether the single
  `.bak` slot should later become rotated/timestamped. Also unresolved from
  earlier passes: ordinary-vs-Advanced field list, first-release
  memory/export/backup scope vs a later destructive-action slice.
- All four review lenses (Grok, Gemini, Claude) are now recorded in the
  roadmap. Chat/Codex consolidation and final owner pass remain outstanding.
- No commit, stage, implementation, or GitHub issue from this pass.

— claude
