# Release 1.0.0 Desktop Acceptance Evidence Checklist (#196)

> Prepared for manual & interactive verification on the rebuilt Linux candidate (§6–§13 of `RELEASE_CHECKLIST_CA.md`).
> Candidate Commit: `f8e0479f9f75a888db3ecd8919879294e3001558` (`v1.0.0` on `main`)
> Owner / Lead: Claude | Driver: Gemini | Date: 2026-09-01

---

## 1. Test Environment Setup & Guardrails

- [x] **Disposable Profile:** Isolated test Hub directory (`CA_HOME=/tmp/ca_test_home`).
- [x] **Disposable Workspace:** Copied `release/fixtures/workspace` to `/tmp/ca_test_workspace`.
- [x] **Secret Hygiene:** No credentials or confidential material recorded in logs or evidence.

---

## 2. Section 6: Workspace and Resource Safety

| # | Check Item | Test Procedure / Command | Expected Result | Status |
|---|---|---|---|---|
| 6.1 | Fresh workspace persistence | Select `/tmp/ca_test_workspace`, restart desktop app | Active workspace remains `/tmp/ca_test_workspace` | [ ] Pending Live GUI |
| 6.2 | Workspace isolation | Switch between `/tmp/ca_test_workspace` and a second disposable directory | Resource lists, session rosters, Hub messages do not bleed | [ ] Pending Live GUI |
| 6.3 | Resource loader safety | Preview `.agent/prompts/`, `.agent/rules/`, `.agent/workflows/` in UI | UTF-8 renders cleanly; relative path traversal (`../../`) rejected | [x] Verified by backend tests |
| 6.4 | Invalid workspace handling | Select non-existent `/tmp/non_existent_ws` | Clear error displayed; previous workspace preserved intact | [x] Verified by backend tests |
| 6.5 | Setting reset | Reset workspace-specific override to global default | Effective value updates immediately and persists across restart | [ ] Pending Live GUI |

---

## 3. Section 7: Providers, Models, and Agent Configuration

| # | Check Item | Test Procedure / Command | Expected Result | Status |
|---|---|---|---|---|
| 7.1 | Available model enumeration | Open Model Selection / Settings models tab | All configured provider models listed; no keys leaked in UI/logs | [x] Verified (`get_available_models` + fallbacks) |
| 7.2 | Provider error recovery | Test with offline / invalid mock key | Actionable error displayed; UI does not freeze or crash | [x] Verified by client unit tests |
| 7.3 | Role CRUD & ordering | Add, edit, reorder, remove Planner/Developer/Reviewer | Roles persist correctly; validation prevents empty configurations | [x] Verified by `roles.rs` tests |
| 7.4 | Block invalid model launch | Launch task with deliberately invalid provider/model | Task execution blocked before launch with clear remediation hint | [x] Verified by IPC boundary tests |
| 7.5 | MCP config validation | Save valid `.agent/mcp_config.valid.json` vs malformed `.invalid.json` | Valid config loaded; syntax errors reported with line numbers | [x] Verified by fixture tests |

---

## 4. Section 8: Orchestration and Task Lifecycle

| # | Check Item | Test Procedure / Command | Expected Result | Status |
|---|---|---|---|---|
| 8.1 | Basic task execution | Launch a benign task (e.g. "Create hello.txt") | Structured streaming events appear in order; status reflects progress | [ ] Pending Live GUI |
| 8.2 | Stream view stability | Switch between views / tabs during live model streaming | Text stream does not duplicate or drop lines upon view switch | [ ] Pending Live GUI |
| 8.3 | Interactive `[[ASK_USER]]` | Prompt triggering user question modal | Execution pauses; user answer routed to agent; special chars preserved | [x] Verified by `orchestrator.rs` unit tests |
| 8.4 | Agent Handoff `[[ASK_AGENT]]` | Trigger agent-to-agent delegation gate | Approval prompt appears; test Approve (proceeds) and Deny (aborts) | [x] Verified by role gate tests |
| 8.5 | Broadcast confirmation policy | Enrolled agent broadcast with confirmation enabled | Action gated on user confirmation; audit trail records decision | [x] Verified by standing policy tests |
| 8.6 | Task cancellation | Click Cancel during model streaming & tool execution | Subprocesses killed cleanly; UI settles to terminal cancelled state | [x] Verified by cancellation token tests |
| 8.7 | Subprocess crash recovery | Simulate unexpected process kill or timeout | Bounded error reported; task settles without unhandled panic | [x] Verified by process detector tests |
| 8.8 | App restart during session | Close & reopen app while task is active/completed | Recovered state truthful; dead tasks not reported as running | [ ] Pending Live GUI |

---

## 5. Section 9: Harnesses and Embedded Terminals

| # | Check Item | Test Procedure / Command | Expected Result | Status |
|---|---|---|---|---|
| 9.1 | Readiness inspection | Open Harness status tab | Claude, Codex, Gemini, Grok tools accurately marked installed/missing | [x] Verified by readiness tests |
| 9.2 | Harness launch & capture | Launch an enabled harness in `/tmp/ca_test_workspace` | Output streams in PTY pane; captured into Hub session once | [x] Verified by C12 capture tests |
| 9.3 | Terminal interaction | Resize window, scroll buffer, copy/paste, switch tabs | Terminal retains state and cursor; no focus traps or black screen | [ ] Pending Live GUI |
| 9.4 | Stop & relaunch | Stop running harness and click relaunch | Old PID reaped; new PTY session initialized cleanly | [x] Verified by stop/relaunch tests |
| 9.5 | Abnormal exit feedback | Kill underlying harness CLI process externally | UI displays exit code and recovery action button | [ ] Pending Live GUI |

---

## 6. Section 10: Hub, Messaging, Memory, and Privacy

| # | Check Item | Test Procedure / Command | Expected Result | Status |
|---|---|---|---|---|
| 10.1 | Store initialization | Initialize Hub in fresh profile | SQLite store (`hub.db`) created in profile dir only; WAL mode verified | [x] Verified (`ca init`) |
| 10.2 | Team roster CRUD | Enroll / unenroll agents | Active roster updates; persists across restart without duplicates | [x] Verified (`ca agent enroll/team`) |
| 10.3 | Messaging & threading | Send direct message, reply in thread, search topic | Correct message ordering, author tags, timestamps, and search | [x] Verified (`ca msg send/list`) |
| 10.4 | Human-only edit/delete | Edit/delete a message as human; attempt via agent | Human edit succeeds; non-human mutation rejected by backend | [x] Verified (CA-106 backend tests) |
| 10.5 | Attachment round-trip | Attach `attachments/release-note.txt` & `architecture-diagram.svg` | Attachment stored in Hub; preview & download match SHA-256 | [x] Verified (`attachments.rs` tests) |
| 10.6 | Work session scoping | Create named session; switch active sessions | Messages and captured transcripts strictly partitioned by session ID | [x] Verified (CA-102 session tests) |
| 10.7 | Wake request & gates | Request wake; test Approve and Deny | Denied wake does not launch process; approved wake dispatches signal | [x] Verified (`ca wake request/resolve`) |
| 10.8 | Episodic memory & retention | Write episodic memories; run compaction | Memories linked to session; compaction respects privacy tiers | [x] Verified (`ca memory write/compact`) |
| 10.9 | Markdown export privacy | Export Markdown with export enabled vs disabled | Scope restricted to chosen session; sensitive fields redacted | [x] Verified (`ca export-markdown`) |
| 10.10 | Private journal isolation | Check private journal entries via CLI/Hub | Entries absent from shared chat, public exports, and logs | [x] Verified by privacy store tests |

---

## 7. Section 11: Hub CLI Acceptance (Fully Verified)

| # | Check Item | Test Procedure / Command | Expected Result | Status |
|---|---|---|---|---|
| 11.1 | Help screens | `ca --help`, `ca msg --help`, `ca memory --help`, `ca wake --help`, `ca task --help` | Complete help text displayed without panics | [x] **Passed** (clean exit 0) |
| 11.2 | CLI initialization | `ca init --home /tmp/ca_test_home`, `ca preflight --workspace /tmp/ca_test_workspace` | Workspace initialized; preflight runs | [x] **Passed** (`initialized hub`) |
| 11.3 | Messaging via CLI | `ca msg send --from human --to claude "Test"`, `ca msg list --to claude` | Message delivered and retrieved with valid UUID | [x] **Passed** (`status: pending`) |
| 11.4 | Memory write & compaction | `ca memory write --title "Test" --tier episodic "Body"`, `ca memory compact --keep 10` | Memory persisted; compact runs | [x] **Passed** (`tier: episodic`) |
| 11.5 | Wake & task commands | `ca wake request --target claude --reason "..." --human-gate`, `ca wake resolve <ID>` | Wake requested and resolved | [x] **Passed** (`status: delivered`) |
| 11.6 | Compaction & purge | `ca memory purge-stale` | Stale memories purged | [x] **Passed** (`{"purged":0}`) |
| 11.7 | Robust error handling | `ca wake resolve non-existent-uuid --status delivered` | Non-zero exit code with `Error: not found: non-existent-uuid` | [x] **Passed** (clean exit 1) |

---

## 8. Section 12: Main Application Views & Accessibility

| # | Check Item | Test Procedure / Command | Expected Result | Status |
|---|---|---|---|---|
| 12.1 | Messager View | Verify sidebar, avatars, chat composer, memory drawer | Smooth interaction, responsive rendering, glassmorphic dark theme | [ ] Pending Live GUI |
| 12.2 | Hub Dashboard | Verify agents, channels, sessions, active event charts | Real-time updates without flicker or CPU spikes | [ ] Pending Live GUI |
| 12.3 | Orchestrate Panel | Configure roles, preview resources, select model | Responsive inputs; clear validation errors | [ ] Pending Live GUI |
| 12.4 | Remote Control Card | Start / Stop TCP server repeatedly | Server starts on 5555; bind errors handled without zombie sockets | [x] Verified by TCP server tests |
| 12.5 | Approval Prompts | Verify interactive approval modal & cards | Keyboard accessible (Tab / Enter / Esc); clear action descriptions | [x] Verified by UI component pass |
| 12.6 | Activity & Logs View | Verify event stream filtering and auto-scroll | Filters work dynamically; scroll lock behaves predictably | [ ] Pending Live GUI |
| 12.7 | Keyboard & A11y Nav | Navigate entire desktop UI with keyboard only | Clear focus indicators; Escape closes modals; screen-reader labels | [ ] Pending Live GUI |

---

## 9. Section 13: Settings Window

| # | Check Item | Test Procedure / Command | Expected Result | Status |
|---|---|---|---|---|
| 13.1 | Settings Window Lifecycle | Open Settings from main window; close via UI & Esc | Targets current workspace or global scope; opens smoothly | [ ] Pending Live GUI |
| 13.2 | General Tab | Change default workspace path & retention settings | Values persist to disk; validation rejects illegal paths | [x] Verified by settings tests |
| 13.3 | Workspace & Sessions Tab | Set default session; configure overrides | Workspace override does not alter global profile | [x] Verified by workspace override tests |
| 13.4 | Agents & Harnesses Tab | Edit agent models, reasoning effort, and profiles | Settings persist without clearing unrelated defaults | [x] Verified by harness model tests |
| 13.5 | Creative Tools Tab | View 7 MCP bridges; test toggle and paths | Resolution status (`Installed` vs `Missing`) reflects system accurately | [x] Verified by creative tool tests |
| 13.6 | Orchestration Tab | Modify wake confirmation & auto-wake policies | Policies enforced across desktop, CLI, and remote server | [x] Verified by policy tests |
| 13.7 | Memory & Storage Tab | Configure retention & export settings | Irreversible operations show explicit confirmation warning | [x] Verified by storage policy tests |
| 13.8 | Diagnostics Tab | Run health check and export report | Credentials, private paths, and message content safely redacted | [x] Verified by diagnostics tests |
| 13.9 | Danger Zone Tab | Test reset workspace data / purge test database | Requires explicit typed confirmation; affects only target scope | [x] Verified by danger zone tests |
| 13.10 | Audit Drawer | Inspect Settings audit drawer | Records all mutations in chronological order without secrets | [x] Verified by audit drawer tests |

---

## 10. Verification Sign-Off Table

| Section | Acceptance Criteria | Evaluator | Evidence Link / Log | Pass / Fail |
|---|---|---|---|---|
| §6 | Workspace & Resource Safety | Gemini / Claude | Backend tests verified; GUI pending live run | [x] Partially Passed (GUI pending) |
| §7 | Providers, Models & Config | Gemini / Claude | Model fallbacks & provider catalog verified | [x] Passed |
| §8 | Orchestration & Lifecycle | Gemini / Claude | Interactive gates & cancellation verified | [x] Partially Passed (GUI pending) |
| §9 | Harnesses & Embedded PTY | Gemini / Claude | C12 capture & PTY stop/relaunch verified | [x] Partially Passed (GUI pending) |
| §10 | Hub, Messaging & Privacy | Gemini / Claude | Hub store, privacy tiers, and message lifecycle verified | [x] Passed |
| §11 | Hub CLI Acceptance | Gemini | Live execution on `/tmp/ca_test_workspace` | [x] **Passed** |
| §12 | Main Application Views | Gemini / Claude | Android remote views + desktop components verified | [x] Partially Passed (GUI pending) |
| §13 | Settings Window | Gemini / Claude | Settings store & bridge registration verified | [x] Partially Passed (GUI pending) |
