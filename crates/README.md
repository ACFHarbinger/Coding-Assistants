# Coding-Assistants crates

| Crate | Binary | Purpose |
| --- | --- | --- |
| [`ca-hub`](ca-hub/) | library | SQLite shared memory, durable messages, wake side-channel, private journals, Markdown export |
| [`ca-cli`](ca-cli/) | `ca` | CLI any agent can invoke without the Tauri GUI |

## Build & install

`ca` is **not** installed system-wide by default. From the **Coding-Assistants repo root**:

```bash
# Build
cargo build -p ca-cli

# Option A — run via path (always works)
./target/debug/ca --help

# Option B — put on PATH (once per rebuild, or use cargo install)
mkdir -p ~/.local/bin
ln -sfn "$(pwd)/target/debug/ca" ~/.local/bin/ca
# ensure ~/.local/bin is on PATH (e.g. in ~/.bashrc):
#   export PATH="$HOME/.local/bin:$PATH"
hash -r
ca --help

# Option C — cargo run (no install)
cargo run -q -p ca-cli -- --help
```

Optional data dir (defaults to `~/.coding-assistants`):

```bash
export CA_HOME="$HOME/.coding-assistants"
```

## Quick start

```bash
# from repo root, after build/install above
ca init

# BODY is a positional argument (not --body)
ca memory write --tier episodic --agent grok --title "note" "hello from grok"

ca msg send --from grok --to claude --kind handoff "please read the hub note"
ca msg poll --to claude

ca wake request --target claude --reason "handoff ready" --human-gate
# Copy the printed "id" field, then:
ca wake resolve 'PASTE-UUID-HERE' --status delivered

ca memory compact --keep 20
ca memory purge-stale
ca memory age-out --hours 72
ca wake policy --set-default-gate true --set-allow-auto true
ca export-markdown
ca export-markdown --commit --message "chore(hub): update shared memory export"

# Sequential workflow (C5) — steps JSON array
ca task create --title "plan-code-review" --workspace "$PWD" \
  --steps '[{"agent":"grok","instruction":"Plan"},{"agent":"claude","instruction":"Implement"},{"agent":"gemini","instruction":"Review"}]'
ca task advance 'TASK-UUID'            # first step
ca task advance 'TASK-UUID' --from grok --note "plan ready"
ca task list --status running
```

**Do not** type angle brackets literally. These are placeholders:

| Placeholder | Meaning |
| --- | --- |
| `PASTE-UUID-HERE` | JSON `"id"` from a prior `memory write`, `msg send`, or `wake request` |
| `<id>` in docs | same — replace with the real UUID string |

Example resolve flow:

```bash
WAKE_JSON=$(ca wake request --target claude --reason "ping" --human-gate)
WAKE_ID=$(echo "$WAKE_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")
ca wake resolve "$WAKE_ID" --status delivered
```

Private journals (`ca journal append --agent grok "private note"`) never write into shared SQLite tables.

Desktop: Tauri `hub_*` commands + **Shared Hub** panel (same data dir).

Roadmap refs: `docs/moon/roadmaps/memory.md` (M1–M5), `docs/moon/roadmaps/communication.md` (C1–C4).

## Tests

```bash
cargo test -p ca-hub
```
