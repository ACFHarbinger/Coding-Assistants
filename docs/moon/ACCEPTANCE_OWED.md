# Owner Live-Verification Checklist (`ACCEPTANCE_OWED.md`)

> **Authoritative Consolidated Checklist** for all features landed in `main` that require owner live verification before closing their respective GitHub issues.  
> **Reference:** Issue [#337](https://github.com/ACFHarbinger/Coding-Assistants/issues/337).  
> **Updated:** 2026-09-20.

---

## Quick Reference & Issue Close Matrix

| Workstream / Item | Description | Target Surface | Issue to Close | Status |
| --- | --- | --- | --- | --- |
| **#213** | Desktop UI scroll stall & layout shift fix | Desktop GUI | [#213](https://github.com/ACFHarbinger/Coding-Assistants/issues/213) | 🔲 Owed |
| **#215** | Workspace bootstrap guardrails & .agent discovery | Desktop GUI | [#215](https://github.com/ACFHarbinger/Coding-Assistants/issues/215) | 🔲 Owed |
| **#309** | Kimi Code CLI provider-native managed harness | CLI / Desktop | [#309](https://github.com/ACFHarbinger/Coding-Assistants/issues/309) | 🔲 Owed |
| **#317 (U24)** | Saved workspaces linked to Work Session Chat | Desktop GUI | [#317](https://github.com/ACFHarbinger/Coding-Assistants/issues/317) | 🔲 Owed |
| **#319 (U25)** | Telegram remote-control bot client | CLI / Telegram | [#319](https://github.com/ACFHarbinger/Coding-Assistants/issues/319) | 🔲 Owed |
| **#321** | Ableton Live MCP bridge & creative tool catalog | CLI / Live LOM | [#321](https://github.com/ACFHarbinger/Coding-Assistants/issues/321) | 🔲 Owed |
| **#322** | Hermes Agent CLI managed harness & usage capture | CLI / Desktop | [#322](https://github.com/ACFHarbinger/Coding-Assistants/issues/322) | 🔲 Owed |
| **#324 (D4)** | Tool & workspace activity views | Desktop GUI | [#324](https://github.com/ACFHarbinger/Coding-Assistants/issues/324) | 🔲 Owed |
| **#325 (P1)** | Internal event bus decoupled from AppHandle | Desktop / TCP | [#325](https://github.com/ACFHarbinger/Coding-Assistants/issues/325) | 🔲 Owed |
| **#307 (U17)** | Animated V-Tuber avatar presence (Open-LLM-VTuber) | Desktop GUI | [#307](https://github.com/ACFHarbinger/Coding-Assistants/issues/307) | 🔲 Owed |
| **#327 (P4)** | Direct HTTP providers using async-openai | Desktop / IPC | [#327](https://github.com/ACFHarbinger/Coding-Assistants/issues/327) | 🔲 Owed |
| **#328 (P10)** | Runtime budget pause, summary & shutdown | Desktop / Hub | [#328](https://github.com/ACFHarbinger/Coding-Assistants/issues/328) | 🔲 Owed |
| **P5 (#333)** | OS-level tool execution with approval & sandbox | CLI / Desktop | [#333](https://github.com/ACFHarbinger/Coding-Assistants/issues/333) | 🔲 Owed |
| **P6 (#332)** | LAN TCP authentication & Android pairing | Desktop / TCP | [#332](https://github.com/ACFHarbinger/Coding-Assistants/issues/332) | 🔲 Owed |
| **P11a (#334)** | Remote workflow delegation over TCP | Hub / TCP | [#334](https://github.com/ACFHarbinger/Coding-Assistants/issues/334) | 🔲 Owed |
| **S3 (#93)** | Google Drive app-data sync adapter | CLI / Hub | [#93](https://github.com/ACFHarbinger/Coding-Assistants/issues/93) | 🔲 Owed |
| **S4 (#94)** | Cloud Sync UI, CLI parity & Hub mutation lock | Desktop / CLI | [#94](https://github.com/ACFHarbinger/Coding-Assistants/issues/94) | 🔲 Owed |
| **T6–T8** | TUI harness panes, multi-instance coherence & Konsole | Ratatui TUI | [#140](https://github.com/ACFHarbinger/Coding-Assistants/issues/140), [#141](https://github.com/ACFHarbinger/Coding-Assistants/issues/141), [#142](https://github.com/ACFHarbinger/Coding-Assistants/issues/142) | 🔲 Owed |
| **P13 (#318)** | OpenRouter model-routing gateway | Desktop / Hub | [#318](https://github.com/ACFHarbinger/Coding-Assistants/issues/318) | 🔲 Owed |
| **P15 (#320)** | xAI Grok Bot direct assistant provider | Desktop / Hub | [#320](https://github.com/ACFHarbinger/Coding-Assistants/issues/320) | 🔲 Owed |

---

## 1. Desktop UI & Workspace Management

### 1.1 Desktop UI Scroll & Layout Shifts (#213)
- **Issue to Close:** [#213](https://github.com/ACFHarbinger/Coding-Assistants/issues/213)
- **Prerequisites:** Desktop app launched on Kubuntu/Linux (`npm run tauri dev`).
- **Verification Steps:**
  1. Launch the Tauri desktop app on a display with standard or fractional scaling.
  2. Maximize the main application window.
  3. Navigate to **Shared Hub** (containing tall card contents across Board, Branches, System, and Usage).
  4. Perform continuous vertical scrolling using the mouse wheel through all cards down to the bottom.
  5. Click and drag the scrollbar thumb directly from top to bottom.
  6. Switch between Orchestrate, Chat & Memory, and Shared Hub.
- **Expected Result:**
  - Scrolling proceeds smoothly without stutter, jitter, sudden jumps to the top/bottom edge, or layout shifts.
  - The scrollbar thumb stays glued to the cursor during drag and does not trap or lose mouse focus.
  - No horizontal scrollbars or overflow jitter appear in the main container.

---

### 1.2 Workspace Bootstrap Wizard Guardrails (#215)
- **Issue to Close:** [#215](https://github.com/ACFHarbinger/Coding-Assistants/issues/215)
- **Prerequisites:** Desktop app running (`npm run tauri dev`).
- **Verification Steps:**
  1. In the **Orchestrate** panel, locate the **Workspace Root** section.
  2. Type a system directory into the path input (e.g., `/etc/my-workspace` or `/usr/local/bin`).
  3. Observe the validation badge next to the input and click **Switch Workspace** or **Bootstrap**.
  4. Type a deep non-existent path where the parent folder does not exist (e.g., `/tmp/nonexistent_parent_dir/project`).
  5. Click **Bootstrap**.
  6. Type a tilde path (e.g., `~/test-ca-bootstrap`) and click **Bootstrap**.
  7. Confirm the creation prompt modal.
  8. Click the 1-click preset chip for `.agent/`.
- **Expected Result:**
  - System directory: Displays a red warning badge `System directory protected` and blocks switching or bootstrapping.
  - Missing parent: Alerts that the immediate parent directory does not exist and blocks arbitrary deep path generation.
  - Tilde expansion: `~` expands cleanly to the user's home directory.
  - Confirmation prompt: An explicit confirmation dialog appears before directory creation. Confirming creates the directory and bootstraps `.agent/` skeleton with rules and prompts.
  - Preset chips: 1-click `.agent/` chip opens without requiring manual hidden-file toggling in the OS file picker.

---

### 1.3 Saved Workspaces Linked to Work Session Chat (U24 / #317)
- **Issue to Close:** [#317](https://github.com/ACFHarbinger/Coding-Assistants/issues/317)
- **Prerequisites:** Desktop app running (`npm run tauri dev`).
- **Verification Steps:**
  1. In **Orchestrate** -> **Workspace Root**, enter an existing workspace path (e.g. repo root) and click **Switch Workspace**.
  2. Under the saved workspaces list, enter a label (e.g. `Main Repo`) in "Save current as..." and click **Save**.
  3. Click **Create team chat**, create a session named `Sprint Chat`, and verify it links to this workspace.
  4. Enter a second workspace path (e.g. `/tmp/test-workspace-2`), save it as `Scratch 2`, and create a linked session `Scratch Chat`.
  5. In the saved workspaces list, click on `Main Repo`.
  6. Click the pencil icon to rename `Main Repo` to `Core Engine`.
  7. Click the trash icon to delete `Scratch 2`.
- **Expected Result:**
  - Selecting `Main Repo` automatically switches `config.work_dir` and immediately focuses **Chat & Memory** on the linked `Sprint Chat`.
  - Renaming updates the name in Hub SQLite (`workspaces` table) and refreshes the picker.
  - Deleting removes the workspace row from the database (nulling session foreign keys) without deleting the physical directory on disk.

---

### 1.4 Animated V-Tuber Avatar Presence (U17 / #307)
- **Issue to Close:** [#307](https://github.com/ACFHarbinger/Coding-Assistants/issues/307)
- **Prerequisites:** Desktop app running. Optional: local [Open-LLM-VTuber](https://github.com/Open-LLM-VTuber/Open-LLM-VTuber) instance on `http://127.0.0.1:8000`.
- **Verification Steps:**
  1. Navigate to **Settings** -> **Agents & harnesses** (or Team Profiles section).
  2. Verify that for every agent identity, "Animated Avatar (Open-LLM-VTuber)" is **disabled by default**.
  3. Toggle the animated avatar on for `gemini` or `claude`.
  4. Click **Test Bridge** without Open-LLM-VTuber running.
  5. (Optional) Start Open-LLM-VTuber on port 8000 and click **Test Bridge**.
  6. Navigate to **Chat & Memory**. Observe the **VTuber Presence Dock** in the chat canvas.
  7. Send a message to the agent whose avatar is enabled.
- **Expected Result:**
  - Default-off: Zero background network requests and zero CPU overhead when off.
  - Offline probe: When service is offline, reports a non-blocking one-line notice: "Bridge offline / connection refused" and cleanly falls back to static avatar without blocking chat.
  - Connected probe: Shows green connected badge.
  - Chat delivery: Completed assistant text is forwarded directly to the `/speak` endpoint (bypassing the model loop in Open-LLM-VTuber to prevent double-generation).

---

## 2. Workspace & Tool Monitoring

### 2.1 Tool and Workspace Activity Views (D4 / #324)
- **Issue to Close:** [#324](https://github.com/ACFHarbinger/Coding-Assistants/issues/324)
- **Prerequisites:** Desktop app running. Have active tasks, work sessions, or tool runs recorded.
- **Verification Steps:**
  1. Navigate to **Shared Hub** -> **Dashboard** tab.
  2. Click the subtab switcher: switch from **Telemetry & Overview** to **Tool & Workspace Activity**.
  3. Verify summary metric cards: *Total activities*, *Commands executed*, *Files touched*, *Active agents*.
  4. Test the **Agent Filter** dropdown (select a specific agent vs "All Agents").
  5. Test the **Time Range** buttons (`1h`, `24h`, `7d`, `30d`, `all`).
  6. Test the **Scope Filter** (`all`, `task`, `work_session`).
  7. Use the search input to filter by a command name or file path.
  8. Click on an activity card to expand its file modifications and executed commands list.
- **Expected Result:**
  - Aggregates activity from `tasks`, `work_sessions`, `audit_events`, and `harness_captures`.
  - Filters update the activity list immediately.
  - Collapsible file touches show operation badges (`created`, `modified`, `removed`, `accessed`) with exact paths.
  - Commands run show executable argv arrays and execution timestamps.

---

### 2.2 Runtime Budget Pause, Summary & Shutdown (P10 / #328)
- **Issue to Close:** [#328](https://github.com/ACFHarbinger/Coding-Assistants/issues/328)
- **Prerequisites:** Hub database initialized (`~/.coding-assistants/hub.db`).
- **Verification Steps:**
  1. Set a small call budget for an agent (e.g. `gemini` with limit 2 calls) via `ca budget set gemini 2`.
  2. Execute an orchestration run requiring 3 or more turns involving `gemini`.
  3. Observe turn execution and budget consumption.
  4. Observe agent behavior when the second call finishes and the third is attempted.
  5. Check workspace directory for generated handoff summary markdown.
  6. Attempt to wake the agent while budget is exhausted: observe wake gate rejection.
  7. Run `ca budget resume gemini` or increase limit: verify agent resumes.
  8. Trigger task cancellation/shutdown: verify shutdown handoff is persisted and budget pauses cleanly.
- **Expected Result:**
  - Exact-fill last unit call succeeds.
  - First over-limit attempt is blocked by `HubStore::gate_provider_call` before calling the model API.
  - A C6 markdown handoff summary is generated on disk.
  - Further calls and wake requests stay paused until explicit human resume.

---

### 2.3 Provider-Native Harness: Moonshot Kimi Code (C14.14 / #309)
- **Issue to Close:** [#309](https://github.com/ACFHarbinger/Coding-Assistants/issues/309)
- **Prerequisites:** Moonshot Kimi CLI installed (`~/.kimi-code/bin/kimi` or on `PATH`), authenticated with valid credentials in `~/.kimi-code/config.toml`.
- **Verification Steps:**
  1. Check CLI detection: run `ca preflight` or inspect the **Harness Readiness** strip in the desktop app.
  2. Verify that `kimi` is recognized with installed and authenticated status.
  3. In **Chat & Memory**, enroll `kimi` in the active team session.
  4. Send a task-tagged message to `kimi`: `@kimi create a hello.txt file with current date`.
  5. Observe PTY/process execution and transcript capture.
  6. Check session list: `kimi session list --json`.
  7. Send a follow-up reply in the session to trigger session resume via `--session <id>`.
- **Expected Result:**
  - Kimi starts headlessly via managed spawn (`kimi -p --output stream-json`).
  - Wire events in `~/.kimi-code/sessions/.../wire.jsonl` are captured into Chat & Memory transcript.
  - Follow-up turn resumes the existing session using `--session <id>`.
  - The presence dot for Kimi in Chat & Memory stays green when ready.

---

### 2.4 Provider-Native Harness: Hermes Agent CLI (C14.16 / #322)
- **Issue to Close:** [#322](https://github.com/ACFHarbinger/Coding-Assistants/issues/322)
- **Prerequisites:** Nous Research Hermes CLI installed (`hermes` on `PATH`), authenticated via `hermes setup` or `~/.hermes/auth.json`.
- **Verification Steps:**
  1. Check Hermes status: run `hermes status`.
  2. Verify desktop app **Harness Readiness** strip shows Hermes detected.
  3. Send a task-tagged message addressing `@hermes`: `@hermes calculate factorial of 10 in python`.
  4. Verify headless one-shot invocation with `--usage-file`.
  5. Verify transcript capture from `hermes sessions export --format jsonl --session-id <id>`.
  6. Verify quota report in desktop Usage tab reflecting tokens and cost from the usage JSON.
- **Expected Result:**
  - Hermes executes headlessly with `-z <task> --in <dir> --usage-file <path>`.
  - Strict sandbox blocks dangerous flags (`--accept-hooks`, `--yolo`).
  - Assistant text appears in Chat & Memory transcript.
  - Token and API call counts are populated truthfully in the usage dashboard.

---

### 2.5 Creative Tools: Ableton Live MCP Bridge (#321)
- **Issue to Close:** [#321](https://github.com/ACFHarbinger/Coding-Assistants/issues/321)
- **Prerequisites:** Python 3. Optional: Ableton Live 11/12 installed.
- **Verification Steps:**
  1. Run the offline dummy-LOM smoke test:
     ```bash
     python3 plugins/ableton/smoke.py
     ```
  2. Start the MCP server in standalone mode:
     ```bash
     cargo run -p mcp-bundle -- ableton --port 9770
     ```
  3. In desktop **Settings** -> **Creative Tools**, verify that Ableton Live is listed in the 8-tool catalog on port 9770.
  4. (If Ableton Live is available) Copy `plugins/ableton/` to:
     `~/Music/Ableton/User Library/Remote Scripts/CodingAssistants/`
     Select **CodingAssistants** under Preferences -> Link/Tempo/MIDI -> Control Surface.
- **Expected Result:**
  - `smoke.py` prints `SMOKE OK` verifying line-JSON protocol over TCP.
  - `coding-assistants-mcp ableton` starts cleanly and speaks standard stdio MCP.
  - When `--allow-run-lom` is omitted, arbitrary code execution tool `run_lom` is disabled.

---

## 3. Direct Model Providers & Gateways

### 3.1 Direct HTTP Providers using async-openai (P4 / #327)
- **Issue to Close:** [#327](https://github.com/ACFHarbinger/Coding-Assistants/issues/327)
- **Prerequisites:** Valid `OPENAI_API_KEY` set in environment or stored in vault.
- **Verification Steps:**
  1. In **Orchestrate** -> **Team Configuration**, configure an agent role with `provider: openai` and `model: gpt-4o-mini`.
  2. Check **Shared Hub** -> **Provider Health**: verify `openai` health row shows HTTP connected.
  3. Start a multi-agent task or send a message requiring OpenAI completion.
  4. Observe execution: verify that NO `opencode` child process is spawned; completion runs in-process.
  5. Check streaming: observe tokens streaming line-by-line in the UI.
  6. Unset `OPENAI_API_KEY` and repeat: verify clean degradation with structured error `unauthenticated` (no app crash or freeze).
- **Expected Result:**
  - Calls execute directly via in-process `async-openai` 0.26 client.
  - Token usage is reported upon completion.
  - Streaming SSE deltas emit onto `agent-event` bus.

---

### 3.2 OpenRouter Model-Routing Gateway (P13 / #318)
- **Issue to Close:** [#318](https://github.com/ACFHarbinger/Coding-Assistants/issues/318)
- **Prerequisites:** Valid `OPENROUTER_API_KEY` set in environment or vault (`tool.openrouter.api_key`).
- **Verification Steps:**
  1. In **Orchestrate** -> **Team Configuration**, select an agent with `provider: openrouter`.
  2. Enter model string (e.g. `openai/gpt-4o-mini,anthropic/claude-3-haiku`).
  3. Check **Provider Health**: verify OpenRouter reports green with key present.
  4. Execute a prompt turn.
  5. Inspect the response in **Chat & Memory** and usage logs in **Shared Hub** -> **Usage**.
- **Expected Result:**
  - Request is dispatched to `https://openrouter.ai/api/v1/chat/completions`.
  - Comma-separated models act as an automated fallback chain.
  - Per-request cost from OpenRouter response header/body is parsed and attributed to the task.

---

### 3.3 xAI Grok Bot Direct Assistant Provider (P15 / #320)
- **Issue to Close:** [#320](https://github.com/ACFHarbinger/Coding-Assistants/issues/320)
- **Prerequisites:** Valid `GROK_BOT_API_KEY` (or `XAI_API_KEY`) set in environment or vault.
- **Verification Steps:**
  1. In **Orchestrate** -> **Team Configuration**, select an agent with `provider: grok-bot`.
  2. Verify that this identity is completely distinct from `HarnessId::Grok` (the CLI coding harness).
  3. Check **Provider Health**: verify `grok-bot` row shows HTTP status.
  4. Send a prompt to the `grok-bot` agent.
- **Expected Result:**
  - Dispatches directly to `https://api.x.ai/v1` via the direct HTTP client.
  - Streams response deltas into Chat & Memory without launching a Grok CLI process.

---

## 4. Platform Execution, Remote & Networking

### 4.1 OS-Level Tool Execution with Approval & Sandbox (P5 / #333)
- **Issue to Close:** [#333](https://github.com/ACFHarbinger/Coding-Assistants/issues/333)
- **Prerequisites:** Terminal CLI and desktop app.
- **Verification Steps:**
  1. Test safe read tool under default Standard sandbox:
     ```bash
     ca tool run ls -la
     ```
  2. Test command with arguments:
     ```bash
     ca tool run git status --short
     ```
  3. Test command requiring approval (mutating/unrecognized):
     ```bash
     ca tool run touch /tmp/ca-tool-test.txt
     ```
  4. Check pending approval queue:
     ```bash
     ca tool pending
     ```
  5. Approve the pending command using its audit ID:
     ```bash
     ca tool approve <audit-id>
     ```
  6. Attempt to run a hard-denied command (shell / sudo):
     ```bash
     ca tool run sudo whoami
     ca tool run bash -c "echo hack"
     ```
- **Expected Result:**
  - Safe read commands (`ls`, `git status`, `rg`) auto-run and print stdout/stderr.
  - Mutating commands return `proposed` with an audit ID and wait for approval.
  - `ca tool approve <id>` executes the approved command and updates status to `approved`.
  - Hard-denied commands (`sudo`, `sh`, `bash`, `zsh`) are rejected immediately (`denied`).
  - Every execution writes an audited entry in `audit_events` with full argv array.

---

### 4.2 LAN TCP Authentication & Android Pairing (P6 / #332)
- **Issue to Close:** [#332](https://github.com/ACFHarbinger/Coding-Assistants/issues/332)
- **Prerequisites:** Desktop app running with TCP server active on port 5555. Android companion device on the same LAN (or `nc` for manual TCP).
- **Verification Steps:**
  1. Verify token configuration in vault or environment: `tool.tcp.auth_token` or `CA_TCP_AUTH_TOKEN`.
  2. Test unauthenticated connection with netcat:
     ```bash
     nc 127.0.0.1 5555
     # Send an unauthorized command without authenticating:
     {"type":"GetAgentCards"}
     ```
  3. Observe socket disconnect and inspect Hub audit journal:
     `ca audit list` -> look for `tcp.auth_rejected`.
  4. Test authenticated connection:
     ```bash
     nc 127.0.0.1 5555
     {"type":"Authenticate","token":"<your-token>"}
     {"type":"GetAgentCards"}
     ```
  5. Launch the Android companion app, enter the server IP and auth token in pairing settings, and connect.
- **Expected Result:**
  - Unauthenticated connections are dropped immediately; broadcasts are withheld until auth.
  - Audit log records `tcp.auth_rejected` with peer IP and reason (never logs the token).
  - Valid token authenticates the session; client receives broadcasts and can trigger actions.

---

### 4.3 Remote Workflow Delegation over TCP (P11a / #334)
- **Issue to Close:** [#334](https://github.com/ACFHarbinger/Coding-Assistants/issues/334)
- **Prerequisites:** Two running instances on LAN (or two ports on localhost) with valid pairing tokens.
- **Verification Steps:**
  1. On Instance A (Server), start TCP server on port 5555 with auth token configured.
  2. On Instance B (Client), configure a workflow step specifying the peer target:
     `WorkflowStep.peer = "127.0.0.1:5555"`.
  3. Execute the workflow on Instance B.
  4. Observe Instance B connecting, sending `Authenticate`, and dispatching `StartTask`.
  5. Observe Instance A executing the task headlessly and returning `TaskComplete`.
- **Expected Result:**
  - Instance A executes the task headlessly in its workspace with full P2 task isolation and budget limits.
  - Instance B receives `TaskComplete{result}` and records it as the step's Handoff message.
  - If token is missing or invalid, delegation fails closed with `HubError` without hanging.

---

### 4.4 Telegram Remote-Control Bot Client (U25 / #319)
- **Issue to Close:** [#319](https://github.com/ACFHarbinger/Coding-Assistants/issues/319)
- **Prerequisites:** Telegram Bot token created via [@BotFather](https://t.me/BotFather), set in `TELEGRAM_BOT_TOKEN` or vault (`tool.telegram.bot_token`).
- **Verification Steps:**
  1. Verify bot token status:
     ```bash
     ca telegram status
     ```
  2. Generate a pairing code:
     ```bash
     ca telegram pair
     ```
  3. In Telegram, open a private chat with the bot and send:
     `/start <pairing-code>`
  4. Run the polling daemon:
     ```bash
     ca telegram run
     ```
  5. From Telegram, send:
     `/wakes`
     `/send Hello team from Telegram`
  6. Trigger a wake gate request in desktop app: observe notification pushed to Telegram.
  7. Reply in Telegram with `/approve <gate-id>` or `/reject <gate-id>`.
  8. Check status and unbind:
     ```bash
     ca telegram status
     ca telegram unbind <user-id>
     ```
- **Expected Result:**
  - Pairing succeeds only with valid, unexpired code within 10 minutes.
  - Outbound long-polling (`getUpdates`) receives commands with no open inbound ports.
  - Non-paired senders are ignored silently.
  - `/approve` and `/reject` resolve wake gates in `HubStore` as `human`.
  - Pushed notifications deliver pending wake requests to paired chats.

---

### 4.5 Decoupled Internal Event Bus (P1 / #325)
- **Issue to Close:** [#325](https://github.com/ACFHarbinger/Coding-Assistants/issues/325)
- **Prerequisites:** Desktop app running with TCP server active.
- **Verification Steps:**
  1. Connect a TCP listener (Android app or `nc` with auth) on port 5555.
  2. In the desktop app Chat & Memory, trigger an agent message or memory recall.
  3. Observe event receipt simultaneously:
     - On desktop GUI (via `AppHandle::emit` subscriber)
     - On TCP socket (via TCP broadcast subscriber)
- **Expected Result:**
  - Both subscribers receive the same `agent-event`, `agent-memory-recall`, and `hub:agents-changed` payloads.
  - Publishers do not block on slow subscribers (bounded sync channel).

---

## 5. Cloud Synchronization & Storage

### 5.1 Google Drive Sync Adapter & Replica Layout (S3 / #93)
- **Issue to Close:** [#93](https://github.com/ACFHarbinger/Coding-Assistants/issues/93)
- **Prerequisites:** Google Drive refresh token configured in `tool.sync.google_refresh_token` or `GOOGLE_DRIVE_REFRESH_TOKEN`.
- **Verification Steps:**
  1. Inspect Drive client initialization: verify API scope is `drive.appdata` (hidden App Data folder).
  2. Run unit / conformance tests:
     ```bash
     cargo test -p hub --lib sync::
     ```
  3. Inspect remote object naming: verify that only hashed `BlobId` hex strings are used (never plaintext workspace names or file paths).
- **Expected Result:**
  - Connects strictly to `spaces=appDataFolder`.
  - Remote objects are isolated under `devices/<id>/` and `replica/` namespaces.
  - Credentials and tokens are redacted from all logs and error strings.

---

### 5.2 Cloud Sync UI, CLI Parity & Hub Mutation Lock (S4 / #94)
- **Issue to Close:** [#94](https://github.com/ACFHarbinger/Coding-Assistants/issues/94)
- **Prerequisites:** Desktop app and terminal CLI.
- **Verification Steps:**
  1. In desktop **Settings**, navigate to the **Synchronization** tab.
  2. Observe account status, category counts, last verified base, and preview plan.
  3. In a terminal, run the CLI equivalent:
     ```bash
     ca sync preview
     ```
  4. Compare desktop preview with CLI preview: verify identical plans.
  5. Start a sync operation (or test simulated held lock `sync/lock`).
  6. While lock is held, attempt a mutating action in another terminal:
     ```bash
     ca inbox watch
     ca tool run touch /tmp/test.txt
     ```
  7. While lock is held, attempt read-only operations:
     ```bash
     ca journal list
     ca budget list
     ```
- **Expected Result:**
  - `ca sync preview` generates plan without taking the mutation lock.
  - When sync lock is active, mutating actions (messages, wakes, tasks, audit/inbox watch) are rejected with a clear error: `Hub is locked during sync`.
  - Read-only operations (list, get, query) continue to function without interruption.
  - Cross-device schema mismatches produce warnings in preview rather than silent hard failures.

---

## 6. Terminal User Interface (Ratatui TUI)

### 6.1 TUI Owned & Observed Harness Panes (T6 / #140)
- **Issue to Close:** [#140](https://github.com/ACFHarbinger/Coding-Assistants/issues/140)
- **Prerequisites:** Run `ca tui` in a terminal window (KDE Konsole).
- **Verification Steps:**
  1. Press `5` or type `:panes` in command palette to open **Harness Panes** tab.
  2. Press `c` (or `Ctrl+B c`) to open launcher modal. Select a harness (e.g. Gemini) with `Owned [Interactive PTY]` mode, and press Enter.
  3. In the focused pane, type shell commands (`ls`, `echo "hello"`) and verify input forwarding and ANSI color rendering.
  4. Press `Ctrl+B c`, toggle mode with `m` to `Observed [Read-Only Captured]`, and press Enter.
  5. Focus the observed pane: type keys and verify keystrokes are blocked with warning `Observed sessions are read-only`.
  6. Test prefix chords:
     - `Ctrl+B s`: Toggle horizontal split view (when window width ≥ 100 cols).
     - `Ctrl+B n` / `Ctrl+B p`: Cycle next and previous panes.
     - `Ctrl+B d`: Detach terminal focus back to top-level TUI navigation.
     - `Ctrl+B x`: Close the active pane and terminate its child process.
- **Expected Result:**
  - Owned panes run interactive child PTYs with dynamic resizing and input routing.
  - Observed panes remain strictly read-only; no interactive writers or PTYs are attached to foreign PIDs.
  - Tmux prefix chords (`Ctrl+B`) intercept cleanly without leaking into child shell.

---

### 6.2 Local Multi-Instance Coherence (T7 / #141)
- **Issue to Close:** [#141](https://github.com/ACFHarbinger/Coding-Assistants/issues/141)
- **Prerequisites:** Two terminal instances running `ca tui` simultaneously against the same home directory.
- **Verification Steps:**
  1. Open Terminal 1: run `ca tui`.
  2. Open Terminal 2: run `ca tui`.
  3. In Terminal 2 (or via desktop app), modify a setting (e.g. change theme or toggle fallback).
  4. In Terminal 1, attempt to edit a setting or wait ~1 second for the periodic runner tick.
  5. Observe the top of Terminal 1 for the conflict banner:
     `Conflict: settings.toml was modified by another instance. Press [r] to Refresh & retry, [Esc] to dismiss.`
  6. Press `r`: verify settings refresh to the latest disk version and banner clears.
  7. Modify again in Terminal 2, then in Terminal 1 press `Esc`: verify banner is dismissed.
- **Expected Result:**
  - Version-stamped reject-and-refresh rejects stale writes without silent last-writer-wins.
  - Conflict banner is non-blocking and does not hide active panes or transcripts.
  - Pressing `r` synchronizes state safely without data loss.

---

### 6.3 Kubuntu/Konsole Resilience & Safety Validation (T8 / #142)
- **Issue to Close:** [#142](https://github.com/ACFHarbinger/Coding-Assistants/issues/142)
- **Prerequisites:** Kubuntu (KDE Plasma + Konsole). See also [the Konsole acceptance checklist](checklists/tui_konsole_acceptance.md).
- **Verification Steps:**
  1. Launch `ca tui` in KDE Konsole.
  2. **Mouse-Off Selection**: Left-click and drag to select text on screen. Middle-click to paste. Verify native Konsole selection works natively without mouse capture interception.
  3. **Narrow Mode**: Resize Konsole window narrower than 80 columns (e.g. 70×24). Cycle through all 5 tabs (`1` through `5`).
  4. **Wide Mode**: Widen Konsole window to ≥120 columns. Open two harness panes and press `Ctrl+B s` to observe dual side-by-side terminal tiles.
  5. **ASCII Fallback**: Run `LANG=C ca tui` or set `unicode_fallback = true` in `settings.toml`. Verify lightning bolt `⚡` falls back to `[*]` and status badges fall back to `[*]`, `[OK]`, `[X]`, `[OBS]`.
  6. **Clean Exit & Restoration**: Press `q` or `Ctrl+C`. Verify terminal cleanly leaves alternate screen with cursor visible and canonical mode restored.
  7. **Desktop Parity (C10–C13)**: Create a session, send messages, resolve wake gates. Inspect `git status`: verify **zero** writes to `.agent/cache/AGENT_BUS.md` or markdown bus files.
- **Expected Result:**
  - Layout adapts smoothly between narrow and wide geometries without crashing or visual corruption.
  - Native clipboard copy-paste works natively in Konsole.
  - All coordination is stored durably in `HubStore` (SQLite) and `settings.toml` with zero markdown bus mutations.
