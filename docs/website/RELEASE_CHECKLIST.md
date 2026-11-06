# Documentation Website Release Checklist

A short, manual pass before (or right after) a `docs/website` deployment to
GitHub Pages. Pair this with the automated checks in `tests/` — the
automation catches regressions in privacy/accessibility invariants and the
content pipeline; this checklist catches what only a human eye or a real
browser reasonably can.

## 1. Automated gate (must be green first)

```sh
cd docs/website
npm ci
npm test        # content pipeline + search ranking + privacy/a11y statics
npm run build   # prebuild (build-content.ts) + tsc --noEmit + vite build
```

If any of these fail, stop — do not proceed to the manual pass below.

## 2. Manual spot-check (5–10 minutes, real browser)

Serve the build locally (`npm run preview` from `docs/website`) and check:

- [ ] **Landing (`/#/`)** loads with the indigo/purple glass aesthetic, not
      a flash of unstyled or light content.
- [ ] **Docs (`/#/docs`)** shows the sidebar with all expected nav groups;
      clicking a nested roadmap entry (e.g. Capability Roadmaps → UI) loads
      that page, not a redirect to the fallback doc.
- [ ] **Direct deep link**: paste `/#/docs/moon/roadmaps/documentation`
      straight into the address bar and hard-reload. Must render the page,
      not 404 or bounce to `/`.
- [ ] **Search (`Cmd+K` / `Ctrl+K`)** opens, returns results for a known
      term (e.g. "roadmap"), and Enter navigates to the top result.
- [ ] **Theme toggle** switches Dark/Light with no flash on reload, and the
      choice survives a hard refresh.
- [ ] **`prefers-reduced-motion`** (OS or DevTools emulation): glow/blur
      animation stops; layout stays usable.
- [ ] **One Mermaid-bearing page** (e.g. a roadmap with a diagram) renders
      the diagram, or falls back to a formatted code block without a blank
      gap or a thrown error in the console.
- [ ] **Keyboard-only pass**: Tab from the top of the page reaches the
      skip-to-content link first, then the header controls, then the page
      body, in a sensible order. No focus trap.
- [ ] **Mobile width** (DevTools responsive mode, ~375px): nav collapses to
      the drawer, no horizontal scroll on the landing page or a long doc.
- [ ] **Browser console**: no errors on landing, a doc page, or after a
      search — warnings from third-party libraries (Mermaid, etc.) are
      acceptable, application errors are not.

## 3. Privacy confirmation

The `tests/privacy-a11y.test.ts` suite already asserts this at build time
(no `fonts.googleapis.com`/`gstatic.com`, no known analytics/tracker hosts,
no external `<link>`/`<script src>` in `dist/index.html`). As a manual
cross-check, open the browser Network tab on the deployed site and confirm
every request stays same-origin (the GitHub Pages host) — no requests to
`google-analytics.com`, `googletagmanager.com`, font CDNs, or any other
third party.

## 4. Post-deploy

- [ ] Confirm the deployed GitHub Pages URL matches the steps above (not
      just the local `npm run preview`) — subpath `base` misconfiguration
      only shows up once actually hosted at the repo's Pages path.
- [ ] Record the verification (date, commit SHA, who ran it) in the
      relevant delivery-phase tracking (issue #122 for the deployment
      itself, or the roadmap's W6/W7 rows).
- [ ] If verification fails, follow the rollback path documented in
      `docs/website/README.md` — do not leave a known-broken deployment
      live while investigating.
