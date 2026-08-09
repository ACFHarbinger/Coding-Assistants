# This repo ships a desktop (Tauri) + Android app, not a hosted backend
# service, so there is nothing here to containerize by default. The one
# containerizable artifact today is the static documentation site under
# docs/website/ — this builds and serves it via nginx.
#
# Build context is the repository ROOT (see docker/docker-compose.yml:
#   build: { context: .., dockerfile: infra/global/docker/Dockerfile }).

# ---------------------------------------------------------------------------
# Stage 1 — builder
# ---------------------------------------------------------------------------
FROM node:26-slim AS builder

WORKDIR /site
COPY docs/website/package.json docs/website/package-lock.json ./
RUN npm ci
COPY docs/website/ ./
RUN npm run build

# ---------------------------------------------------------------------------
# Stage 2 — runtime
# ---------------------------------------------------------------------------
FROM nginx:alpine AS runtime

COPY --from=builder /site/dist /usr/share/nginx/html

EXPOSE 80
