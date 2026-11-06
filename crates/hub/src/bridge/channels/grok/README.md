# Grok Leader Channel (C14.6 / #154)

Documented, opt-in two-way Coding-Assistants bridge for Grok's leader-mode ACP
capability.

This bridge enables Hub-delivered messages to reach a live, human-attended Grok
terminal without attaching undocumented PTYs or inventing custom sockets.

---

## Architecture & How It Works

Grok Build provides a documented Agent Client Protocol (ACP) server interface:

1. **Leader Daemon:** Grok runs as a leader server via:
   ```bash
   grok agent leader --no-exit-on-disconnect --relay-on-demand --leader-socket ~/.grok/leader.sock
   ```
2. **Leader TUI:** A live terminal session connects to the leader daemon via:
   ```bash
   grok --leader --leader-socket ~/.grok/leader.sock --cwd <workspace> [--resume <session-id>]
   ```
3. **Hub ACP Client:** When a task is dispatched to Grok in a workspace, Coding-Assistants
   invokes `grok agent --leader stdio` as an ACP client connecting to `~/.grok/leader.sock`.
   It loads the active session (`session/load`) and prompts the agent (`session/prompt`).

### Why Standalone Grok TUIs Are Refused

A default standalone Grok TUI (started without `--leader`) does **not** listen on
`~/.grok/leader.sock`. To preserve terminal stability and avoid corrupting the user's
interactive session, Coding-Assistants strictly refuses injection when the leader
socket is absent:
- Status reports: `unavailable`
- Detail: `"no leader socket at ~/.grok/leader.sock — start Grok with --leader (or [cli] use_leader = true) to enable Hub delivery. Task stays queued."`
- The task remains safely queued in `hub.db` until a leader session is available.

---

## Setup & Connecting Grok

### Method 1: Desktop GUI (Recommended)

1. Open **Shared Hub** → **Channels**.
2. Under the **Grok leader** section:
   - Click **Start new leader session** to spawn `grok agent leader` daemon and open a new leader TUI.
   - Click **Connect / resume live** to attach an existing standalone session into a new `--leader` window.
3. Once connected, the badge switches to **● Leader connected**.

### Method 2: Command Line (CLI)

Start Grok directly in leader mode in your workspace:
```bash
cd /path/to/workspace
grok --leader
```

Or configure Grok to always use leader mode in `~/.grok/config.toml`:
```toml
[cli]
use_leader = true
```

---

## Live Acceptance Verification Checklist

To verify live delivery for [#154](https://github.com/ACFHarbinger/Coding-Assistants/issues/154):

1. **Standalone Safe Refusal:**
   - Start Grok normally (`grok`) in a standalone terminal.
   - Send a task to `@grok` from Chat & Memory.
   - Verify the message stays `queued`, status shows `unavailable`, and the standalone terminal is undisturbed.
2. **Leader Delivery:**
   - Click **Connect / resume live** in **Channels** tab (or launch `grok --leader`).
   - Re-send the task to `@grok`.
   - Verify the message appears in the live Grok TUI and Grok begins generating a response.
3. **Output Capture:**
   - Verify the generated reply is captured into Coding-Assistants Chat & Memory transcript.
