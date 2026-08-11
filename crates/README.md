# Coding-Assistants crates

| Crate | Binary | Purpose |
| --- | --- | --- |
| [`ca-hub`](ca-hub/) | library | SQLite shared memory, durable messages, wake side-channel, private journals, Markdown export |
| [`ca-cli`](ca-cli/) | `ca` | CLI any agent can invoke without the Tauri GUI |

## Quick start

```bash
# from repo root
cargo build -p ca-cli
export CA_HOME="$HOME/.coding-assistants"   # optional; default is ~/.coding-assistants

./target/debug/ca init
./target/debug/ca memory write --tier episodic --agent grok --title "note" --body "hello"
./target/debug/ca msg send --from grok --to claude --kind handoff --body "please read note"
./target/debug/ca msg poll --to claude
./target/debug/ca wake request --target claude --reason "handoff ready" --human-gate
./target/debug/ca export-markdown
./target/debug/ca memory compact --keep 20
./target/debug/ca memory promote <id> --to episodic
```

Private journals (`ca journal append --agent grok`) never write into shared SQLite tables.

Roadmap refs: `docs/moon/roadmaps/memory.md` (M1+), `docs/moon/roadmaps/communication.md` (C1–C3).
