# Grok Leader-Mode Live-Session Delivery Acceptance Checklist (C14.6 / #154)

> **Reference:** Roadmap C14.6 ([`docs/moon/roadmaps/communication.md`](../roadmaps/communication.md)), GitHub Issue [#154](https://github.com/ACFHarbinger/Coding-Assistants/issues/154).  
> **Target Environment:** Linux Kubuntu / Desktop GUI + Grok CLI.

---

## 1. Automated Test Suite Verification

Run the automated bridge and channel tests in the repository:

```bash
cargo test -p hub --lib bridge::grok
cargo test -p hub --lib bridge::channels::grok
```

Verify that all unit tests pass cleanly:
- `grok_tests::grok_acp_client_args_structure`: ACP client invocation arguments structure (`grok agent --leader stdio --leader-socket <socket>`).
- `grok_tests::missing_leader_is_unavailable_and_does_not_spawn_a_tui`: Standalone TUI refusal without leader socket (returns `unavailable`, does not spawn spurious processes).
- `grok_tests::task_stays_queued_when_leader_socket_is_missing`: Task remains safely queued in Hub store when leader is missing.
- `leader::tests::grok_leader_daemon_args_contain_relay_and_socket`: Leader daemon spawn arguments (`agent leader --no-exit-on-disconnect --relay-on-demand`).
- `leader::tests::grok_leader_tui_args_include_workspace_and_leader`: Leader TUI arguments (`--leader --no-alt-screen --minimal --cwd <dir>`).
- `leader::tests::connect_requires_absolute_workspace`: Refuses relative workspace paths fail-closed.

---

## 2. Interactive Acceptance Checklist (Owner Live Verification)

### A. Preflight & Standalone Safe Refusal
- [ ] **Preflight Check**: Run `ca preflight` or open the desktop **Harness Readiness** strip. Verify Grok binary is detected.
- [ ] **Standalone Launch**: Open a standard terminal window in your workspace and start Grok without `--leader`:
  ```bash
  grok
  ```
- [ ] **Safe Refusal Verification**: In Coding-Assistants, navigate to **Chat & Memory** and send a task-tagged message to Grok:
  ```text
  @grok list the files in this directory
  ```
- [ ] **Expected Refusal Behavior**:
  - The live standalone Grok TUI is **completely undisturbed** (no foreign characters, no broken prompt, no uninvited input).
  - In Coding-Assistants, the task status is marked `unavailable` / `queued`.
  - The detail message clearly informs the user:
    `"no leader socket at ~/.grok/leader.sock — start Grok with --leader (or [cli] use_leader = true) to enable Hub delivery. Task stays queued."`

### B. Leader Mode Connect & Delivery
- [ ] **Connect via Channels Tab**:
  1. Open **Shared Hub** → **Channels**.
  2. Locate the **Grok leader** section.
  3. Verify the status badge initially displays `○ No leader socket` (or shows live standalone PID with notice).
  4. Click **Start new leader session** (or **Connect / resume live** if a session is present).
- [ ] **Verify Socket & Badge**:
  - A new terminal window opens running `grok --leader`.
  - The leader socket `~/.grok/leader.sock` is created.
  - The status badge in Channels tab switches to `● Leader connected`.
- [ ] **Task Delivery Verification**:
  1. Return to **Chat & Memory**.
  2. Send a task-tagged message:
     ```text
     @grok what is 13 * 17?
     ```
  3. Observe the live Grok leader terminal: the task is received over ACP (`session/load` + `session/prompt`), displayed in the TUI, and answered.
- [ ] **Transcript Capture**:
  - Verify Grok's generated response appears in the Coding-Assistants Chat & Memory transcript with correct attribution.

### C. Quiet Message Gating
- [ ] **Untagged Chat Test**: Send a plain, untagged chat message without `@grok` into the session.
- [ ] **Expected Result**: Plain messages remain quiet in the Hub inbox and do **not** interrupt the active leader session.

### D. Session Resume After Disconnect
- [ ] **Disconnect Test**: Close the Grok terminal window.
- [ ] **Resume Test**: In **Channels** tab, click **Connect / resume live**.
- [ ] **Expected Result**: Grok launches with `--resume <session-id>` in `--leader` mode, reconnecting to the previous session history.
