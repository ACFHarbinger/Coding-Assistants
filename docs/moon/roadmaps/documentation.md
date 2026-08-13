# Documentation & Website Roadmap

> **Status:** Approved
> **Date:** 2026-08-13
> **Product name:** Coding-Assistants
> **Target:** A coherent, modern documentation programme: canonical Markdown content,
> contributor standards, a static React 19 + TypeScript documentation/product
> website matching the desktop app's glassmorphism aesthetic, and its GitHub Pages publishing workflow.

## Locked direction

- **Framework & Tech Stack:** Replace the `docs/website` Vue/Vite prototype with a modular React 19 + TypeScript + Vite application in an **isolated** `docs/website` npm project. Do not retain a parallel Vue implementation. Do not import desktop `src/` into the website.
- **Branding:** Keep the public product name **Coding-Assistants** throughout the site, header, metadata, and social cards.
- **Design System & Aesthetics:** Dark-first glassmorphism that **matches the live desktop app** (`src/index.css`): slate field `#020617`, indigo `#6366f1`, purple `#a855f7`, glass cards (`rgba(15, 23, 42, 0.92)`, 16px radius, `blur(20px)`), two-corner radial glows, crisp low-contrast borders, generous whitespace. Light theme is a first-class toggle, not the default. Self-host Inter and JetBrains Mono (no Google Fonts request).
- **Styling implementation:** Tailwind CSS, configured with the desktop tokens (§ Design tokens) as `theme.extend` values (colors, border radius, blur), rather than hand-written CSS or CSS Modules. This is a deliberate divergence from the desktop app's plain-CSS approach, traded for build velocity across many small landing/docs components; token *values* still match the desktop exactly. The 500-line file bound still applies per-component; Tailwind's utility classes live in JSX/TSX, not in a growing global stylesheet.
- **Dual-Purpose Experience:** Product-forward landing page first (`/#/`), then an interactive documentation reader one click away (`/#/docs`).
- **Content Pipeline:** Preserve Markdown files under `docs/` as the single canonical source of truth. A deterministic TypeScript build-time content script (`scripts/build-content.ts`) parses a **curated** corpus, validates internal links/headings, builds a MiniSearch index, and emits typed JSON manifests (`docs-manifest.json` and `search-index.json`) as **gitignored build artifacts**.
- **Routing Strategy:** Use `HashRouter` (`/#/docs/...`) to guarantee 100% reliable direct links and page reloads under GitHub Pages subpath hosting without requiring complex server redirect fallbacks.
- **Interactive Suite:** Local full-text search command palette (`Cmd+K` / `Ctrl+K`) via MiniSearch, scroll-aware on-page TOC, code copy buttons, syntax highlighting via `react-syntax-highlighter` (Prism themes), client-side Mermaid.js diagram rendering, and dark/light theme selection with zero flash.
- **Landing hero visual:** An abstract multi-agent architecture graphic (radial glow behind connected nodes for Grok/Claude/Codex/Gemini around a central Hub), not a desktop app screenshot — stays visually fresh with no upkeep as the UI evolves and doubles as a preview of the architecture docs.
- **Brand mark:** Recolor the existing desktop app icon (`src-tauri/icons/`, an interlocking-circles mark, currently cyan/gold) into the site's indigo `#6366f1` / purple `#a855f7` tokens for the header logo and favicon, rather than designing a new mark or shipping a text-only wordmark.
- **Privacy & legal:** No analytics, no cookies, no third-party font or tracker requests. Footer states AGPL-3.0 and links to `LICENSE`.
- **Automated Deployment:** Deploy the static build artifact via GitHub Actions (`.github/workflows/docs.yml`) to GitHub Pages beneath the repository base path.

## Current-state findings

| Area | Current state | Required outcome |
| --- | --- | --- |
| Deployed docs | `.github/workflows/docs.yml` builds Material for MkDocs from `docs/mkdocs.yml` | GitHub Actions installs Node, runs the content generator and Vite build, and deploys static output to Pages. |
| Prototype | `docs/website` is a Vue 3/Vite single-component prototype using Plus Jakarta Sans and indigo-only tokens | Isolated React 19 + TypeScript + Vite application with modular feature components, HashRouter, and desktop-matched tokens. |
| Content | `docs/` Markdown is hand-authored; `generate_docs_json.py` builds legacy JSON | Markdown remains canonical; `scripts/build-content.ts` produces typed metadata, MiniSearch index, and navigation manifest from the curated corpus. |
| Branding | Prototype uses template-era "Polyglot Portal" metadata | Consistent Coding-Assistants name, tagline, metadata, favicon, and social graph preview cards. |
| Design tokens | Desktop app: indigo `#6366f1` / purple `#a855f7` / `#020617`. Prototype: Plus Jakarta + indigo. Earlier site lock used cyan `#24C8D8` / violet `#8B5CF6`. | Owner-locked: copy the **live desktop** tokens, not the Tauri-badge cyan pair. |
| Hosting | GitHub Pages is deployed by existing workflow | Vite `base` set for Pages subpath; HashRouter ensures zero 404s on page refresh or deep linking. |
| Documentation governance | `DOCUMENTATION_STANDARDS.md`, ADRs, Moon roadmaps, and changelogs exist | Single discoverable information architecture, clear contributor rules, and link validation script. |
| Infra overlap | `infra/docker/` documentation-site stack is I1 (done) for MkDocs-era local preview | Docker preview is optional later; W1–W6 do not depend on it. |

## Information architecture

### Public routes

| Route | Purpose | Content & Experience |
| --- | --- | --- |
| `/#/` | Product-forward landing | Hero with product narrative, install/quick-start snippet, capability grid, multi-agent architecture preview, workflow overview, docs CTA, and GitHub link. |
| `/#/docs` | Documentation overview | Getting-started quick links, featured guides, capability map, and recent updates. |
| `/#/docs/:slug*` | Rendered document route | Nested slugs (`architecture`, `moon/roadmaps/memory`). Markdown rendering, code copy, Mermaid, TOC. |
| `/#/docs/roadmap` | Product planning entry point | `docs/moon/ROADMAP.md` plus capability roadmap index. |
| `/#/docs/changelog` | Release/change history | Canonical `docs/moon/CHANGELOG.md` (and a pointer to historical entries). |
| `*` | Not-found page | Custom 404 recovery page with search shortcut and navigation links. |

### Navigation groups

- **Start Here:** Overview, Tutorial, Installation & Development, Troubleshooting.
- **Understand the Product:** Architecture, Security Policy, Glossary, Benchmarks.
- **Contribute:** Development Guide, Testing Strategy, Dependencies, Documentation Standards, ADRs.
- **Project Status:** Roadmap, Capability Roadmaps (Memory, Communication, UI, Dashboard, Platform, Infrastructure, Cloud Sync, Documentation), Changelog.

### Published content corpus

`scripts/build-content.ts` includes only:

- Top-level product docs: `docs/*.md`
- Architecture decisions: `docs/adr/**/*.md`
- Moon index and history: `docs/moon/ROADMAP.md`, `docs/moon/CHANGELOG.md`
- Capability roadmaps: `docs/moon/roadmaps/*.md`

It **excludes** `docs/moon/archive/`, `docs/moon/research/`, and `docs/moon/reports/` unless a published page links them (linked targets may be inlined or 404 with a clear "not published" message; they are not in the sidebar or default search corpus). New Markdown is unpublished until it matches a glob above or is added to this table.

Optional YAML frontmatter (all fields optional): `title`, `description`, `nav_group`, `order`, `draft`. Draft pages fail the production content build. Slug = path relative to `docs/` without the `.md` suffix (`docs/moon/roadmaps/ui.md` → `moon/roadmaps/ui`). Heading anchors use GitHub-style slugification so in-repo `#` links survive.

## Technical architecture

```text
curated docs/*.md + docs/adr/** + docs/moon/ROADMAP.md
+ CHANGELOG.md + docs/moon/roadmaps/*.md
          │
          ▼
scripts/build-content.ts (Build-time TypeScript script)
          │  validates frontmatter & links; emits typed manifest + MiniSearch index
          ▼
Isolated React 19 + TypeScript + Vite app (`docs/website/`)
 ├── app shell (glass top bar, theme toggle, mobile drawer)
 ├── landing feature (hero w/ architecture graphic, capability grid, CTA)
 ├── docs feature (sidebar, react-markdown article, TOC, prev/next)
 ├── search feature (Cmd+K palette, MiniSearch over titles/headings/body)
 ├── Markdown renderer (remark-gfm, rehype-slug, react-syntax-highlighter, Mermaid.js)
 └── design system (Tailwind config on desktop tokens, glass utility classes, self-hosted fonts, recolored brand mark)
          │
          ▼
static `dist/` deployed via GitHub Actions to GitHub Pages
```

### Implementation locks

| Decision | Lock |
| --- | --- |
| Package boundary | Isolated `docs/website` npm project. Root desktop `package.json` stays Tauri/React app-only. |
| Router | `react-router` `HashRouter`. Vite `base` = `/${github.event.repository.name}/` (or the repo's Pages path). |
| Styling | Tailwind CSS (`tailwind.config.ts` + `postcss.config.js`), `theme.extend` populated from the design tokens table below. No hand-written global stylesheet beyond Tailwind's base layer and `@font-face` rules. |
| Markdown | `react-markdown` + `remark-gfm` + `rehype-slug`. Replace prototype `marked` + Vue. |
| Syntax highlighting | `react-syntax-highlighter` (Prism themes), lazy-loaded per language used on a page to keep the initial bundle small. |
| Search | MiniSearch over title, headings, and body. No FlexSearch WASM, no Fuse.js. |
| Fonts | Self-hosted Inter and JetBrains Mono under `docs/website/src/assets/fonts`. No Google Fonts `@import`. |
| Theme | Dark default (matches desktop). Light toggle persisted in `localStorage`. Apply theme class before paint to avoid flash. Optional "System" item may follow `prefers-color-scheme` but must not override a stored choice. |
| Motion | Desktop-like radial glows and glass blur. `prefers-reduced-motion: reduce` drops glow animation and reduces blur to a solid translucent fill. |
| Hero visual | Abstract multi-agent architecture graphic (SVG/inline component: radial glow, Hub node, Grok/Claude/Codex/Gemini satellite nodes) — not a desktop screenshot. Reused/linked from the docs architecture page for consistency. |
| Brand mark | Recolored `src-tauri/icons/` interlocking-circles mark (indigo `#6366f1` / purple `#a855f7`) as the header logo and favicon set; source as SVG under `docs/website/src/assets/brand/` so it can be recolored again without re-exporting from the original design tool. |
| Generated JSON | Build artifacts in `docs/website/src/content/`; gitignore them. CI always runs `build-content.ts` before Vite. Retire `generate_docs_json.py`. |
| Tests | Vitest + React Testing Library in `docs/website/tests`. Smoke-render landing and one doc route; unit-test slug/link rewrite. |
| Privacy | No analytics, no cookies, no third-party requests at runtime except GitHub Pages itself. |
| License | Persistent footer: AGPL-3.0, link to repo `LICENSE`. |
| File size | Follow the repo 500-line bound for website `.ts` / `.tsx` / `.css` units — Tailwind keeps most styling as JSX utility classes rather than growing a global CSS file. |
| Desktop coupling | Token *values* are copied, not imported from `src/index.css`, so the desktop app can change without breaking the site build. |

## Source layout target

```text
docs/website/
├── tailwind.config.ts        # theme.extend populated from the desktop token table
├── postcss.config.js
├── scripts/                 # build-content.ts & link-checker
├── src/
│   ├── app/                 # router, theme provider, app shell
│   ├── features/
│   │   ├── landing/         # hero (architecture graphic), capability cards, CTA
│   │   ├── docs/            # sidebar, Markdown article, TOC, prev/next
│   │   └── search/          # Cmd+K MiniSearch palette
│   ├── components/          # glass cards, buttons, badges, theme toggle
│   ├── content/             # generated JSON (gitignored)
│   ├── styles/              # Tailwind entrypoint, @font-face rules, markdown prose overrides
│   └── assets/
│       ├── brand/           # recolored logo/favicon SVG source (indigo/purple)
│       └── fonts/           # self-hosted Inter / JetBrains Mono
├── public/                  # favicon, static assets, social metadata
└── tests/                   # Vitest + Testing Library
```

## Delivery phases

| # | Milestone | Scope | Exit criteria |
| --- | --- | --- | --- |
| W1 | Foundation and React setup | Replace Vue tooling with React 19 + TypeScript + Vite in isolated `docs/website`. HashRouter, Tailwind configured with desktop design tokens, self-hosted fonts, Vite Pages base. | `npm run build` succeeds; no Vue / `marked` / Google Fonts remain; dark shell renders with indigo/purple glass via Tailwind utilities. |
| W2 | Content inventory & build script | `scripts/build-content.ts` parses the curated corpus (not all of `docs/**`), validates internal links and headings, builds MiniSearch JSON, fails on draft-in-production or broken nav links. | Deterministic content build; unpublished trees stay out of the manifest. |
| W3 | Documentation reading experience | Glass sidebar, `react-markdown` article, scroll-aware TOC, heading anchors, prev/next, code copy, syntax highlighting, Mermaid.js. | All published docs render; keyboard nav and code copy work offline. |
| W4 | Product landing page | Product-forward landing: hero with the abstract multi-agent architecture graphic, quick-start snippet, capability grid, docs + GitHub CTAs. Docs reader stays one click away. | Landing matches desktop glass theme, is fluidly responsive, and links into `/#/docs`. |
| W5 | Search palette & theme toggle | `Cmd+K` / `Ctrl+K` MiniSearch palette; Dark/Light (optional System) toggle with persisted choice and zero theme flash. | Offline search ranks titles, headings, and body; theme works on every route; reduced-motion is honored. |
| W6 | CI/CD Pages deployment & cutover | Update `.github/workflows/docs.yml` to `npm ci` in `docs/website`, run content build + Vite, deploy `dist/`. Update contributor docs; retire MkDocs and `generate_docs_json.py`. | Site live on GitHub Pages; direct loads, refreshes, search, theme, and Mermaid verified. |
| W7 | Polish | Social cards, recolored-mark favicon set, skip-link audit, mobile drawer, print stylesheet for articles, 404, AGPL footer. Optional later: Docker preview aligned with I1. | WCAG AA spot-check on landing + one long doc; no third-party network on first load. |

## Design and accessibility requirements

- **Design system tokens (copied from the live desktop app):**

  | Token | Value | Role |
  | --- | --- | --- |
  | `--bg-dark` | `#020617` | Page field (dark default) |
  | `--primary` | `#6366f1` | Indigo accent, links, focus |
  | `--primary-hover` | `#4f46e5` | Hover |
  | `--accent` | `#a855f7` | Purple secondary accent / glow |
  | `--bg-card` | `rgba(15, 23, 42, 0.92)` | Glass card fill |
  | `--glass-bg` | `rgba(255, 255, 255, 0.03)` | Hairline glass tint |
  | `--glass-blur` | `blur(20px)` | Card/header blur (solid fill when reduced-motion) |
  | `--border-color` | `rgba(255, 255, 255, 0.08)` | Crisp borders |
  | `--text-main` | `#f8fafc` | Primary text |
  | `--text-muted` | `#94a3b8` | Secondary text |
  | `--radius-card` | `16px` | Card corners |

  Light theme inverts surfaces and text while keeping indigo/purple accents. Earlier cyan `#24C8D8` / violet `#8B5CF6` values are **not** the site palette.

- **Atmosphere:** Fixed two-corner radial glows (indigo at top-left, purple at bottom-right), same idea as `.app-container::before` in the desktop app.
- **Typography:** Self-hosted Inter for UI/body; JetBrains Mono for code and identifiers.
- **Accessibility:** WCAG AA contrast, visible keyboard focus rings, semantic landmarks (`<main>`, `<nav>`, `<aside>`), skip navigation link, `prefers-reduced-motion` support.

## Content rules and migration constraints

- `docs/` Markdown files remain the sole canonical source of truth. Generated website content files are build artifacts only and are gitignored.
- Only the curated corpus is published. Archive, research, and reports stay in git for agents/contributors, not in the public sidebar.
- Markdown links to `.md` files inside the corpus become HashRouter paths (`/#/docs/...`). Links that escape the corpus fail the content build.
- Client-side Mermaid rendering degrades gracefully to formatted code blocks if a syntax error occurs.
- Images and static assets use base-relative paths compatible with GitHub Pages subpath hosting.

## Validation plan

| Check | Minimum evidence |
| --- | --- |
| Build & Content validation | `npm run build` in `docs/website` runs `build-content.ts` and Vite with 0 errors. Unpublished trees are absent from the manifest. |
| Code & Type Safety | `npx tsc --noEmit` returns 0 TypeScript errors across the website project. |
| Tests | `npm test` in `docs/website` runs Vitest smoke + slug/link unit tests. |
| Navigation & Search | HashRouter links, deep links, theme toggle, and Cmd+K MiniSearch function offline without 404 errors. |
| Theme & motion | Dark is default; light persists; first paint is not unstyled light; reduced-motion removes glow animation. |
| Privacy | Built `index.html` / JS has no Google Fonts, analytics, or cookie banner. |
| Deployment | GitHub Actions deploys static output to GitHub Pages. |

---
