#!/usr/bin/env bash
# Stage Tauri externalBin sidecars into src-tauri/binaries.
# Tauri looks up each name with the host target triple appended.
set -euo pipefail

profile="${1:-debug}"
case "$profile" in
    debug|dev)
        outdir="target/debug"
        cargo_args=()
        ;;
    release)
        outdir="target/release"
        cargo_args=(--release)
        ;;
    *)
        echo "usage: stage-sidecars.sh [debug|release]" >&2
        exit 2
        ;;
esac

root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

env_file="${HOME}/.local/share/coding-assistants/dev-env.sh"
if [[ -f "$env_file" ]]; then
    # shellcheck disable=SC1090
    source "$env_file"
fi

triple="$(rustc -vV | awk '/^host:/ { print $2 }')"
if [[ -z "$triple" ]]; then
    echo "rustc did not report a host triple" >&2
    exit 1
fi

mkdir -p src-tauri/binaries
cargo build "${cargo_args[@]}" -p claude -p mcp-bundle

for name in coding-assistants-claude-channel coding-assistants-mcp; do
    src="${outdir}/${name}"
    dest="src-tauri/binaries/${name}-${triple}"
    if [[ ! -f "$src" ]]; then
        echo "missing sidecar binary: ${src}" >&2
        exit 1
    fi
    install -m 0755 "$src" "$dest"
    echo "staged ${dest}"
done
