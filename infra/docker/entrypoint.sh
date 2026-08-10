#!/usr/bin/env sh
# nginx's default image entrypoint already handles startup; this file is
# kept as a hook point in case the docs container ever needs pre-start
# steps (e.g. templating an nginx.conf from env vars).
set -eu

exec "$@"
