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
  W1/W2 are complete; W4/W5 have passed public Pages inspection; W6 is deployed;
  W3 needs its focused reader repair and W7 is ready for its next deployment.
- The documentation site uses the curated build-content pipeline. Local verification
  currently passes with `npm test` and `npm run build` in `docs/website`.

## Active task board

| Owner | Issue / workstream | Current task | Coordination boundary |
| --- | --- | --- | --- |
| Chat / Codex | #122 — W6 deployment and cutover | Replace the MkDocs Pages workflow with the verified React-site build and deployment flow; update contributor guidance and retain a safe rollback path. Remove legacy MkDocs assets only after the deployed Pages site is verified. | Own `.github/workflows/docs.yml`, documentation deployment guidance, and cutover validation. |
| Gemini | #119 — W3 documentation reader | Fix the public-site reader regressions found in Pages acceptance: replace remaining cyan/fixed-dark reader chrome with locked indigo/purple and theme tokens, and ensure React Markdown's internal `node` prop is not emitted to DOM code elements. Add focused regression coverage and verify build. | Own `docs/website/src/features/docs/` and reader-focused tests only. Do not change deployment workflow, landing/navigation, or W7 print/404 files. |
| Grok | #120/#121 — W4 landing and W5 navigation | Review complete; perform the landing/navigation portion of the shared Pages visual acceptance once a deployment is available. | Do not alter reader, print/404, or workflow files without a new assignment. |
| Claude | #123 — W7 polish follow-up | Complete the remaining bounded W7 implementation: article print stylesheet and a custom HashRouter not-found view. Add targeted static checks where practical; keep the dedicated 1200×630 social-card asset explicitly optional for this pass. | Own print/404 code and related tests/guidance. Do not change reader rendering or the Pages workflow. |

### Shared completion rules

- Re-read this file immediately before editing and claim a task in a dated update.
- Keep issue #116 and its linked subissues accurate; comment with verification results
  when a task reaches review.
- Run the scoped tests and build before handoff. Report blockers, changed files, and
  verification in the next dated update.
- Do not close an issue solely because code exists: meet its acceptance criteria and
  obtain any required owner or deployment verification first.

## 2026-08-13 updates

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
