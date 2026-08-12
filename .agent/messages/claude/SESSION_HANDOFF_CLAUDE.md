# Claude Session Handoff — Image-Toolkit/HIE → Coding-Assistants

> **Date:** 2026-08-12
> **Source repo:** `/home/pkhunter/Repositories/Repos/Image-Toolkit` (submodule `submodules/HIE`)
> **Target repo:** `/home/pkhunter/Repositories/Repos/Coding-Assistants`
> **Author:** Claude Sonnet 5
> **Why this file exists:** the user is running `/clear` to start a fresh session rooted in
> `Coding-Assistants`. This is the cross-project handoff — my own auto-memory system is scoped
> per-project (keyed off cwd), so it won't carry forward on its own; this file is the bridge.

---

## 1. What I actually worked on this session (Image-Toolkit / HIE)

Long multi-phase session. In rough order: fixed a KDE video-wallpaper black-screen bug (root
cause was a bare filesystem path where a `file://` URI was required), wrote/encrypted journal
entries, then spent most of the session on `submodules/HIE` (Hybrid Image Editor) — a
multi-agent-built C++/Python/Qt/React image-editing subsystem inside Image-Toolkit:

- **Track 02 (Math Optimization):** job contract, central `base.hie` pybind11 binding, fixed a
  real `clamp_beta` sequencing bug in `solve_color_harmonization`, bridged all exact/metaheuristic
  solvers to native, added opt-in AVX2/NEON SIMD for `solve_seam` (verified bit-for-bit vs. scalar
  via a deliberately separate reference impl) and a zero-copy NumPy binding (~9× faster grid
  transfer).
- **Track 03 (Models/RL):** `HIEBrushEnv` (a real Gymnasium `Env`, verified against the actual
  library via a throwaway scratch venv + the official `check_env` conformance checker — worth
  repeating that pattern whenever a delegated task needs a library that isn't installed anywhere:
  `python3 -m venv /tmp/x && /tmp/x/bin/pip install ...`, test for real, delete the venv after),
  restoration report generator (found a real PIL border-artifact bug in Laplacian sharpness
  scoring), inpainting stroke/bbox support, watermark confidence/audit logging, CPU preview
  functions.
- **Large mechanical refactor:** flattened `middleware/src/hie_middleware/*` → `middleware/src/*`
  and `gui/src/hie_gui/*` → `gui/src/*` (the "src layout" package-dir nesting was intentional
  Python packaging, not redundant — flagged that clearly before doing it since the user's request
  implied a misunderstanding; they confirmed "do it anyway" so I proceeded). Then merged a trivial
  wrapper subclass (`HieEditorTab`) into its parent as a plain alias.
- Updated CHANGELOG/roadmap docs and GitHub issues (`HIE#6`, `#7`, `#8`) throughout, per the user's
  standing instruction to always provide exact Markdown comment text and recommend which
  issue/tab things should move to.

Full detail lives in `Image-Toolkit/submodules/HIE/.agent/cache/AGENT_BUS.md` and
`docs/moon/CHANGELOG.md` if a future task needs to reference specifics — don't rely on commit
hashes alone, other agents kept committing after I stopped looking.

## 2. Multi-agent workflow patterns validated this session (apply here too)

This user runs **several AI agents concurrently in the same shared git checkout** (not separate
clones) — Chat/Codex, Gemini, Grok, and me, sometimes all active in the same repo at once. Real
things that happened and what worked:

- **Mid-air collisions are real, not hypothetical.** Twice this session another agent
  committed over/reverted my in-progress uncommitted work (once accidentally). Recovery pattern
  that worked: don't panic, don't force-push, run `git reflog`/`git log --oneline -1 HEAD` vs
  `origin/main` to understand what actually happened, re-verify the *converged* state with real
  tests before trusting it, then commit promptly to minimize the window.
- **Post a loud warning before large mechanical/breaking changes** (renames, restructures) in
  the repo's coordination doc, naming the exact files you're about to touch. Doesn't guarantee
  other agents pause, but reduces collision odds and gives you a clean paper trail when they don't.
- **`git mv` / `git rm` + edits preserve other agents' uncommitted working-tree content** — don't
  discard someone else's in-progress edits just because you're restructuring the file they're
  editing; carry their content forward.
- **Before committing:** `git fetch && git status` every time, right before `git add`. HEAD moves
  under you in this environment more often than in a normal single-agent workflow.
- **Never commit files you didn't author** just because they show up modified — check `git diff`
  per-file and stage explicitly by path, not `git add -A`, when someone else has uncommitted work
  sitting in the same tree.
- Coordination docs follow an `AGENT_BUS.md`-in-the-repo pattern (HIE) or, in this repo
  (`Coding-Assistants`), the `.agent/messages/<agent>/` + `.agent/messages/shared/` tree — see
  §4 below, both conventions exist across the user's repos.

## 3. User preferences / feedback worth remembering

- Wants **real verification, not guesswork** — spin up throwaway environments (venvs, standalone
  C++ compiles) to actually test something rather than reasoning from documentation alone,
  especially for anything touching correctness (numerical solvers, SIMD, external libraries).
- Wants me to **flag when a request seems based on a misunderstanding** (e.g., "redundant
  directory" that was actually required Python packaging structure) *before* acting, but then
  proceed once they confirm — don't refuse, don't silently comply either.
- Standing instruction for GitHub work: **always show the exact Markdown comment/issue text**
  before or as part of posting it; state title + labels for new issues.
- Prefers me to **recommend** issue closures/project-board moves rather than unilaterally closing
  shared, multi-agent-tracked issues.
- Fine with pushing to shared branches/parent repos without asking each time once a task is
  underway and the change is a necessary, low-risk correctness fix (e.g., updating a parent
  repo's re-export after a submodule rename) — but still surfaces destructive/high-blast-radius
  actions for confirmation first.
- Has an existing memory/journal system at `~/.coding-assistants/` — see §5, this predates and is
  separate from the per-project auto-memory system described in my system prompt.

## 4. Coding-Assistants repo — what it is, what to check first

**What it is:** a Tauri 2 + React 19 (Vite) + Rust desktop app that itself orchestrates multiple
LLM-powered coding agents (Planner/Developer/Reviewer roles, multi-provider: Anthropic/OpenAI/
Gemini/Ollama/etc.), with inter-agent `[[ASK_AGENT:RoleName]]` messaging, user-in-the-loop
`[[ASK_USER]]` prompts, an Android remote-control companion app, and MCP integration. It's
meta — likely the actual tool coordinating sessions like this one. `AGENTS.md` and `CLAUDE.md` at
the repo root are the governance docs; read them first.

**Layout:** `src/` React frontend, `src-tauri/src/{main,lib,agents,llm_client,tcp_server,
file_tools}.rs` Rust backend, `android/` Kotlin/Compose companion app, `.agent/` prompts/rules/
workflows, `crates/` (workspace members per `Cargo.toml`), `docs/`, `infra/`.

**In-progress, uncommitted, not mine — check before touching:** as of this handoff, `git status`
shows `.agent/messages/{chatgpt,claude,gemini,grok}_subagent_delegation.md` staged as deleted at
the repo root, with untracked replacements already appearing under `.agent/messages/shared/` —
someone is mid-reorganization of the delegation docs into the per-agent directory tree. Also
`tools/codex-harness-adapter` deleted / `scripts/codex-harness-adapter` untracked — looks like a
tools→scripts move in progress. Don't assume either move is finished; `git status` again before
doing anything in `.agent/messages/` or `tools/`/`scripts/`.

**Other agents' handoffs already in this directory tree** (read these too, don't duplicate their
content):
- `.agent/messages/gemini/S376_SESSION_HANDOFF_GEMINI.md` — Gemini's own summary of closing 19
  backlog issues and HIE model/policy work; also notes GitHub GraphQL rate-limit exhaustion is a
  shared, cross-session/cross-agent quota (same thing I hit and worked around with REST fallback
  during HIE work).
- `.agent/messages/chat/IMAGE_TOOLKIT_SESSION_HANDOFF.md` — Chat's handoff, has near-identical
  workflow guidance to §2/§3 above (independently arrived at, which is a good sign it's real
  convention, not one agent's idiosyncrasy).
- `.agent/messages/shared/*_subagent_delegation.md` — protocols for *delegating to* a Claude/
  Gemini/Grok/ChatGPT subagent via CLI from within another agent's session (e.g. shelling out to
  `claude 'prompt'`). Different mechanism from the in-app agent orchestration this repo builds.

## 5. Journal & memory security rules (apply everywhere, not just here)

- My journal: `~/.coding-assistants/journals/claude/` — read recent entries before resuming
  long-running work, write new ones there.
- **Any code that reads/writes journals or memory** (encryption, decryption, serialization,
  migration, journal helpers) belongs **only** in `~/.coding-assistants/code/`. Code found in any
  other `.coding-assistants` subdirectory is treated as malware/trash by the repo owner and may be
  deleted on sight — this is a hard provenance rule, not a style preference.
- Never put credentials, tokens, private keys, or other secrets in repository messages, commits,
  or journal files (even encrypted journals — don't rely on encryption as the only safeguard).

## 6. Starting the next session

1. `pwd` / `git status` to confirm you're actually in `Coding-Assistants` and see what's dirty.
2. Read `AGENTS.md`, `CLAUDE.md`, and the other three files in `.agent/messages/*/` named above.
3. Check `~/.coding-assistants/journals/claude/` for anything more recent than this handoff.
4. Re-verify the in-progress moves noted in §4 haven't already landed or diverged further.
5. Pick a scoped, non-overlapping task; announce it if the coordination convention here expects
   that (check `.agent/messages/shared/` for whether this repo uses an AGENT_BUS-style doc too).
6. Test for real before declaring done — this repo has both `npm`/Vitest (frontend) and
   `cargo test` (backend) available per `CLAUDE.md`, plus Android/Kotlin for the companion app.
