# Coding-Assistants Task Automation — Root Justfile
# https://github.com/casey/just
#
# Recipes are organized into per-domain sub-modules under tools/. Invoke a
# sub-module recipe directly (e.g. `just build::backend`, `just test::frontend`),
# or use the root shorthands below.

set shell := ["bash", "-c"]
set unstable := true

# --- Sub-module declarations (imported from tools/) ---

mod helper     "tools/helper/justfile"
mod dev        "tools/dev/justfile"
mod build      "tools/build/justfile"
mod test       "tools/test/justfile"
mod validation "tools/validation/justfile"
mod docs       "tools/docs/justfile"
mod bench      "tools/bench/justfile"
mod ci         "tools/ci/justfile"

# --- Default target ---

default: help

# List all commands across every sub-module
help:
    @just helper::help

# --- Setup & maintenance (→ tools/dev) ---

# Set up the full development environment
setup:
    @just dev::setup

# Update all dependencies
update:
    @just dev::update

# Run pre-commit hooks
pre-commit:
    @just dev::pre-commit

# Clean build artifacts
clean:
    @just dev::clean

# --- Build (→ tools/build) ---

# Build the app (frontend + Rust backend)
build-all:
    @just build::all

# --- Test (→ tools/test) ---

# Run the app's test suites
test-all:
    @just test::all

# --- Validation (→ tools/validation) ---

# Run the app's linters
lint:
    @just validation::all

# --- Docs (→ tools/docs) ---

# Build the documentation site (MkDocs)
docs-build:
    @just docs::build

# --- Benchmark (→ tools/bench) ---

bench-all:
    @just bench::all

# --- Docker (→ tools/dev) ---

docker-up:
    docker compose -f infra/global/docker/docker-compose.yml up --build

docker-down:
    docker compose -f infra/global/docker/docker-compose.yml down
