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
