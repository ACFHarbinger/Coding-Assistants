# Documentation website tests

Test harness for the Coding-Assistants React documentation website.

| Path | Role |
| --- | --- |
| `unit/` | Fast Vitest checks for the content pipeline, search, UI invariants, privacy, and built output |
| `integration/` | Vitest checks joining the generated content manifest, MiniSearch index, and canonical Markdown |
| `cypress/e2e/` | Browser end-to-end flows |
| `cypress/smoke/` | Fast smoke specs |
| `cypress/cypress.config.js` | Cypress config (`baseUrl` defaults to Vite dev server) |

## Commands

From `docs/website/`:

```bash
npm test                 # unit + integration (Vitest)
npm run test:unit
npm run test:integration
npm run lint
npm run cypress:run      # requires `npm run dev -- --host 127.0.0.1` in another terminal
npm run cypress:smoke    # fast public landing and theme checks
```
