# Documentation & Website Roadmap

> **Status:** Approved
> **Date:** 2026-08-13
> **Product name:** Coding-Assistants
> **Target:** A coherent, modern documentation programme: canonical Markdown content,
> contributor standards, a static React 19 + TypeScript documentation/product
> website matching the desktop app's glassmorphism aesthetic, and its GitHub Pages publishing workflow.

## Locked direction

- **Framework & Tech Stack:** Replace the `docs/website` Vue/Vite prototype with a modular React 19 + TypeScript + Vite application. Do not retain a parallel Vue implementation.
- **Branding:** Keep the public product name **Coding-Assistants** throughout the site, header, metadata, and social cards.
- **Design System & Aesthetics:** Implement a glassmorphism dark/light theme mirroring the desktop app's visual system (semitransparent blur cards, dark background, cyan `#24C8D8` / violet `#8B5CF6` accents, crisp borders, generous whitespace, and Google Fonts Inter / JetBrains Mono typography).
- **Dual-Purpose Experience:** Ship both a high-impact product landing page and an interactive documentation reader.
- **Content Pipeline:** Preserve Markdown files under `docs/` as the single canonical source of truth. A deterministic TypeScript build-time content script (`scripts/build-content.ts`) parses Markdown, validates internal links/headings, builds a local search index, and emits typed JSON content manifests (`docs-manifest.json` and `search-index.json`).
- **Routing Strategy:** Use `HashRouter` (`/#/docs/...`) to guarantee 100% reliable direct links and page reloads under GitHub Pages subpath hosting without requiring complex server redirect fallbacks.
- **Interactive Suite:** Include a local full-text search command palette (`Cmd+K` / `Ctrl+K`), scroll-aware on-page table of contents (TOC), code copy buttons, syntax highlighting, client-side Mermaid.js diagram rendering, and light/dark theme selection.
- **Automated Deployment:** Deploy the static build artifact via GitHub Actions (`.github/workflows/docs.yml`) to GitHub Pages beneath the repository base path.

## Current-state findings

| Area | Current state | Required outcome |
| --- | --- | --- |
| Deployed docs | `.github/workflows/docs.yml` builds Material for MkDocs from `docs/mkdocs.yml` | GitHub Actions installs Node, runs the content generator and Vite build, and deploys static output to Pages. |
| Prototype | `docs/website` is a Vue 3/Vite single-component prototype | React 19 + TypeScript + Vite application with modular feature components and HashRouter navigation. |
| Content | `docs/` Markdown is hand-authored; `generate_docs_json.py` builds legacy JSON | Markdown remains canonical; `scripts/build-content.ts` produces typed metadata, search index, and navigation manifest. |
| Branding | Prototype uses template-era "Polyglot Portal" metadata | Consistent Coding-Assistants name, tagline, metadata, favicon, and social graph preview cards. |
| Hosting | GitHub Pages is deployed by existing workflow | Vite `base` set for Pages subpath; HashRouter ensures zero 404s on page refresh or deep linking. |
| Documentation governance | `DOCUMENTATION_STANDARDS.md`, ADRs, Moon roadmaps, and changelogs exist | Single discoverable information architecture, clear contributor rules, and link validation script. |

## Information architecture

### Public routes

| Route | Purpose | Content & Experience |
| --- | --- | --- |
| `/#/` | Product landing page | Product narrative, core capabilities, architecture preview, workflow overview, documentation CTA, and GitHub link. |
| `/#/docs` | Documentation overview | Getting-started quick links, featured guides, capability map, and recent updates. |
| `/#/docs/:slug` | Rendered document route | Canonical documentation pages with Markdown rendering, code copy, Mermaid diagrams, and TOC. |
| `/#/docs/roadmap` | Product planning entry point | `docs/moon/ROADMAP.md` plus capability roadmap index. |
| `/#/docs/changelog` | Release/change history | Canonical release changelog and historical entries. |
| `*` | Not-found page | Custom 404 recovery page with search shortcut and navigation links. |

### Navigation groups

- **Start Here:** Overview, Tutorial, Installation & Development, Troubleshooting.
- **Understand the Product:** Architecture, Security Policy, Glossary, Benchmarks.
- **Contribute:** Development Guide, Testing Strategy, Dependencies, Documentation Standards, ADRs.
- **Project Status:** Roadmap, Capability Roadmaps (Memory, Communication, UI, Dashboard, Platform, Infrastructure), Changelog.

## Technical architecture

```text
docs/*.md + docs/moon/**/*.md
          │
          ▼
scripts/build-content.ts (Build-time TypeScript script)
          │  validates frontmatter & links; emits typed manifest + search index
          ▼
React 19 + TypeScript + Vite application (`docs/website/`)
 ├── app shell (glassmorphism top bar, theme toggle, mobile drawer)
 ├── landing-page feature (hero, feature grid, architecture preview, CTA)
 ├── documentation feature (sidebar, article renderer, TOC, prev/next)
 ├── search feature (Cmd+K command palette modal with FlexSearch/Fuse.js)
 ├── Markdown renderer (code copy, syntax highlighting, Mermaid.js)
 └── design system (CSS custom properties, glass cards, typography)
          │
          ▼
static `dist/` deployed via GitHub Actions to GitHub Pages
```

## Source layout target

```text
docs/website/
├── scripts/                 # build-content.ts & link-checker
├── src/
│   ├── app/                 # router, theme provider, app shell
│   ├── features/
│   │   ├── landing/         # hero, capability cards, architecture preview
│   │   ├── docs/            # sidebar, Markdown article, TOC, prev/next
│   │   └── search/          # Cmd+K command palette modal
│   ├── components/          # glass cards, buttons, badges, theme toggle
│   ├── content/             # generated docs-manifest.json and search-index.json
│   ├── styles/              # CSS tokens, glassmorphism, typography, markdown
│   └── assets/              # Coding-Assistants icons and visuals
├── public/                  # favicon, static assets, social metadata
└── tests/                   # unit and render smoke tests
```

## Delivery phases

| # | Milestone | Scope | Exit criteria |
| --- | --- | --- | --- |
| W1 | Foundation and React setup | Replace Vue tooling with React 19 + TypeScript + Vite in `docs/website`. Configure HashRouter, glassmorphism design tokens (CSS custom properties), and Vite Pages base. | `npm run build` succeeds; no Vue dependencies remain; React 19 shell renders cleanly. |
| W2 | Content inventory & build script | Implement `scripts/build-content.ts` to parse canonical `docs/**/*.md`, validate internal links and headings, build search index, and output typed JSON manifests. | Content build script runs deterministically; missing nav links or broken doc links raise build errors. |
| W3 | Documentation reading experience | Build docs layout: glass sidebar, article renderer, scroll-aware TOC, heading anchors, prev/next links, code copy button, syntax highlighting, and Mermaid.js diagrams. | Article reader renders all canonical docs; keyboard navigation and code copy work without server dependencies. |
| W4 | Product landing page | Create responsive Coding-Assistants landing page: hero section with quick-start snippet, capability grid, multi-agent architecture visualization, and CTA buttons. | Landing page matches glassmorphism theme, is fluidly responsive, and links directly into docs. |
| W5 | Search palette & theme toggle | Build `Cmd+K` / `Ctrl+K` command palette search modal with FlexSearch/Fuse.js; implement Dark/Light/System theme toggle with persisted choice and zero theme flash. | Offline search ranks titles, headings, and body content; theme toggle works smoothly across all routes. |
| W6 | CI/CD Pages deployment & cutover | Update `.github/workflows/docs.yml` to build React site and deploy static output to GitHub Pages; update contributor docs and retire legacy MkDocs config. | Site live on GitHub Pages; direct loads, refreshes, search, theme toggle, and Mermaid rendering verified. |

## Design and accessibility requirements

- **Design System Tokens:** CSS custom properties for surfaces (`--bg-primary`, `--glass-surface`, `--glass-border`), text colors, cyan accent (`#24C8D8`), violet accent (`#8B5CF6`), and focus rings.
- **Typography:** Self-hosted / Google Fonts Inter for UI/body text and JetBrains Mono for code blocks and technical identifiers.
- **Accessibility:** WCAG AA contrast compliance, visible keyboard focus rings, semantic landmarks (`<main>`, `<nav>`, `<aside>`), skip navigation link, and `prefers-reduced-motion` support.

## Content rules and migration constraints

- `docs/` Markdown files remain the sole canonical source of truth. Generated website content files are build artifacts only.
- Markdown links to `.md` files are automatically transformed into HashRouter path links (`/#/docs/...`).
- Client-side Mermaid rendering degrades gracefully to formatted code blocks if a syntax error occurs.
- Images and static assets use base-relative paths compatible with GitHub Pages subpath hosting.

## Validation plan

| Check | Minimum evidence |
| --- | --- |
| Build & Content validation | `npm run build` in `docs/website` runs `build-content.ts` and Vite build with 0 errors. |
| Code & Type Safety | `npx tsc --noEmit` returns 0 TypeScript errors across the React website project. |
| Navigation & Search | HashRouter links, deep links, theme toggle, and Cmd+K search function offline without 404 errors. |
| Deployment | GitHub Actions workflow successfully deploys static build output to GitHub Pages. |

---

## Changelog

### 2026-08-13 — Gemini (Roadmap Approval & Architecture Lock)

- Promoted status from `Draft` to `Approved`.
- Locked design system to **Glassmorphism Dark/Light Theme** matching the desktop app (`#24C8D8` cyan & `#8B5CF6` violet accents, Inter & JetBrains Mono typography).
- Locked content pipeline to a deterministic TypeScript script (`scripts/build-content.ts`) parsing canonical `docs/**/*.md` into typed JSON manifests and FlexSearch/Fuse.js index.
- Locked routing strategy to `HashRouter` (`/#/docs/...`) for guaranteed subpath link stability on GitHub Pages.
- Locked interactive feature suite to include Product Landing Page, `Cmd+K` Command Palette Search Modal, scroll-aware TOC, Code Copy buttons, and client-side Mermaid.js rendering.
- Detailed W1–W6 delivery milestones and technical architecture.
