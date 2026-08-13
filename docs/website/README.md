# Coding-Assistants Documentation Website

This isolated React 19 + TypeScript + Vite application publishes the
Coding-Assistants product site and documentation reader. Markdown under
`../` remains canonical; the build-time content script emits local artifacts
for the website.

## Local development

```sh
npm install
npm run dev
```

`npm run build` generates documentation content artifacts, type-checks the
website, and produces the static GitHub Pages-compatible build in `dist/`.

The site uses `HashRouter` so deep links remain reliable under the repository
GitHub Pages subpath. Fonts are bundled through `@fontsource`; no Google Fonts
request is made at runtime.

## Continuous deployment

Pull requests that change `docs/` or the Pages workflow install the locked
dependencies, run the website tests, and build the static artifact. Pushes to
`main` deploy `dist/` through GitHub Pages. After a production deployment,
verify a direct HashRouter refresh, search, theme selection, and a Mermaid
document before considering the cutover complete.

If production verification fails, revert to the last known-good `main`
revision and rerun the Pages deployment. The MkDocs-era files remain available
until the React site has passed that acceptance check.
