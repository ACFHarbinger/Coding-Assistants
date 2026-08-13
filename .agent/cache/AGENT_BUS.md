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
  W1 and W2 are implemented, W3–W5 are in review or active hardening, and W6/W7
  remain active.
- The documentation site uses the curated build-content pipeline. Local verification
  currently passes with `npm test` and `npm run build` in `docs/website`.

## Active task board

| Owner | Issue / workstream | Current task | Coordination boundary |
| --- | --- | --- | --- |
| Chat / Codex | #122 — W6 deployment and cutover | Replace the MkDocs Pages workflow with the verified React-site build and deployment flow; update contributor guidance and retain a safe rollback path. Remove legacy MkDocs assets only after the deployed Pages site is verified. | Own `.github/workflows/docs.yml`, documentation deployment guidance, and cutover validation. |
| Gemini | #119 — W3 documentation reader | Replace the legacy `marked`/direct-HTML rendering path with `react-markdown`, locked GFM and heading-slug plugins; render a clear public “not published” notice from the generated manifest. Run website tests and build. | Own `docs/website/src/features/docs/` reader rendering. Do not change deployment workflow. |
| Grok | #120/#121 — W4 landing and W5 navigation | Complete visual and interaction QA for landing, navigation, command palette, theme controls, mobile drawer, and reduced-motion behavior. Remove remaining off-palette cyan styling where it conflicts with the indigo/purple design system. | Own landing/navigation presentation. Avoid Markdown reader and workflow files. |
| Claude | #123 — W7 polish and release confidence | Add a focused static privacy/accessibility regression check and a concise manual release checklist. Confirm the built site makes no runtime font, analytics, or tracking requests. Keep changes to tests, release guidance, and public metadata only. | Do not alter route components, Markdown rendering, or the Pages workflow without a new bus assignment. |

### Shared completion rules

- Re-read this file immediately before editing and claim a task in a dated update.
- Keep issue #116 and its linked subissues accurate; comment with verification results
  when a task reaches review.
- Run the scoped tests and build before handoff. Report blockers, changed files, and
  verification in the next dated update.
- Do not close an issue solely because code exists: meet its acceptance criteria and
  obtain any required owner or deployment verification first.

## 2026-08-13 updates

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
