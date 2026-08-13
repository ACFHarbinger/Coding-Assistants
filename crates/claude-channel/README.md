# `claude-channel`

C14.3: an **opt-in**, two-way Coding-Assistants bridge for Claude Code's
documented, research-preview `claude/channel` MCP capability
(Claude Code 2.1.231+).

This is not another way to attach to an arbitrary running `claude`
process. It is a plain stdio MCP server that Claude Code deliberately
loads for one specific, deliberately-configured workspace. Sessions that
have not opted in keep using the existing capture-only C12 bridge
(`hub::bridge::claude`) exactly as before — this crate never touches that
file, and it never uses Claude Code's undocumented internal `cc-socks`
control socket.

## What it does

- **Pushes authenticated Hub events into the session.** A background
  thread polls the Hub for pending messages addressed to `claude` and
  relays each one as an MCP `notifications/claude/channel` push. The
  *authenticated sender gate* is: only messages from an **enrolled team
  member** are relayed — an unenrolled or stray identity string can never
  reach a live session through this path. See
  `hub::bridge::claude_channel::poll_channel_events`.
- **Routes Claude's replies back to the Hub.** Claude calls the `reply`
  MCP tool this server exposes; the handler records it as a Hub message
  addressed to the original sender (or `human` if the reply isn't tied to
  a specific prior message), optionally scoped to a Hub session. See
  `hub::bridge::claude_channel::record_channel_reply`.
- **Relays permission decisions only after explicit human approval.**
  When Claude Code opens a permission dialog in a Channel-enabled session,
  it notifies this server (`notifications/claude/channel/permission_request`).
  The request is durably recorded as `pending` on the Hub's audit chain —
  never auto-approved. A human resolves it via
  `hub::bridge::claude_channel::resolve_permission_request`, and only then
  does this server relay the verdict
  (`notifications/claude/channel/permission`) back to Claude Code.

## Setup (per workspace, explicit opt-in)

```bash
cargo build -p claude-channel
./target/debug/coding-assistants-claude-channel --setup --workspace /abs/path/to/workspace
```

This registers `claude` as a Hub-**managed** harness session for that
workspace (so the existing C14.1 single-writer lease applies to it like
any other managed provider) and writes/merges a `coding-assistants-channel`
entry into that workspace's `.mcp.json`, without touching any other server
already configured there.

Then, in that workspace:

```bash
claude --channels
```

Channels are a **research preview**: they require Claude Code 2.1.231+,
Anthropic authentication (claude.ai or a Console API key — not available
on Bedrock/Vertex/Foundry), and until this bridge is allowlisted,
`--dangerously-load-development-channels` to load it. Team/Enterprise
orgs must have `channelsEnabled` (and optionally an `allowedChannelPlugins`
entry for this bridge).

## Protocol surface

Everything this process sends or receives is standard MCP (JSON-RPC 2.0
over stdio) plus Claude Code's two documented experimental capabilities:

| Direction | Method | Purpose |
| --- | --- | --- |
| server → client (declared at `initialize`) | `capabilities.experimental["claude/channel"]` | Opts this server into receiving/pushing Channel events. |
| server → client (declared at `initialize`) | `capabilities.experimental["claude/channel/permission"]` | Opts into the permission-relay extension. |
| server → client (notification) | `notifications/claude/channel` | Pushes one authenticated Hub event (`{content, meta}`) into the session. |
| client → server (tool call) | `tools/call` (`reply`) | Claude sends output back; the handler routes it into the Hub. |
| client → server (notification) | `notifications/claude/channel/permission_request` | A permission dialog opened; recorded `pending`, never auto-resolved. |
| server → client (notification) | `notifications/claude/channel/permission` | Relayed only after a human explicitly approves/denies. |

## Tests

```bash
cargo test -p claude-channel   # pure helpers: config merge, tool schema, response shaping
cargo test -p hub -- claude_channel::  # Hub-side gate, reply routing, and permission lifecycle
```

Nothing here spawns a real Claude Code process or touches a live session
in tests — the MCP protocol plumbing is exercised through pure functions
(`merge_mcp_config`, `tool_call_response`, `reply_tool_schema`), and the
Hub-side authenticated-gate/reply/permission logic is exercised directly
against a temporary `HubStore`.
