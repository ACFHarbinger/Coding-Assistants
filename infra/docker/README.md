# Docker

This repo ships a desktop (Tauri) + Android app, not a hosted backend
service — there is no server-side component to containerize. The one
containerizable artifact today is the static documentation site under
`docs/website/`; that's what this compose stack builds and serves.

## Quick start

```bash
docker compose -f infra/docker/docker-compose.yml up --build
# docs site now served at http://localhost:8080
```

## Files

| File | Purpose |
| --- | --- |
| `Dockerfile` | Multi-stage build: `npm run build` for `docs/website/`, served via nginx |
| `docker-compose.yml` | Local dev stack: the docs site container |
| `docker-compose.prod.yml` | Production overrides (apply with `-f infra/docker/docker-compose.yml -f infra/docker/docker-compose.prod.yml`) |
| `entrypoint.sh` | Placeholder hook point; nginx's own entrypoint handles startup today |

## Notes

- Build context is the **repository root**, not `infra/docker/` — the Dockerfile needs access to `docs/website/`.
- `.dockerignore` lives at the repo root for the same reason.
- If a hosted backend service is ever added (e.g. a cloud relay for the
  Android companion app), add a new stage/service here rather than
  repurposing the docs one.
