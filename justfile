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

# Build and install the `ca` hub CLI onto ~/.local/bin
install-ca:
    @just dev::install-ca

# Launch the Tauri application in development mode
start:
    @just dev::dev

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

# --- Shorthands ---
# Note: none of these share a name with a `mod` above (just forbids that);
# use the module directly (e.g. `just build::debug`) for anything not listed here.

# Loop the Claude Code agent on a stateful context
loop-claude prompt="Continue implementing the studio, updating the ROADMAP and CHANGELOG, and commiting your work": helper::_print_header
    just agent::loop-claude "{{prompt}}"

# Loop the Grok agent on a stateful context
loop-grok prompt="Continue implementing the studio, updating the ROADMAP and CHANGELOG, and commiting your work": helper::_print_header
    just agent::loop-grok "{{prompt}}"

# Loop the Gemini agent on a stateful context
loop-gemini prompt="Continue implementing the studio, updating the ROADMAP and CHANGELOG, and commiting your work": helper::_print_header
    just agent::loop-gemini "{{prompt}}"

# Loop the ChatGPT agent on a stateful context
loop-chatgpt prompt="Continue implementing the studio, updating the ROADMAP and CHANGELOG, and commiting your work": helper::_print_header
    just agent::loop-chatgpt "{{prompt}}"
