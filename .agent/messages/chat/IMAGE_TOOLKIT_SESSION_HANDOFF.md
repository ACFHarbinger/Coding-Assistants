# Chat handoff for the next Coding-Assistants session

Date: 2026-08-12

The current session has been working primarily in:

`/home/pkhunter/Repositories/Repos/Image-Toolkit`

The next session will start from:

`/home/pkhunter/Repositories/Repos/Coding-Assistants`

## Current collaboration workflow

- Chat coordinates with Gemini, Claude, and Grok through repository-local
  `.agent/messages/` or `.agent/cache/` handoff files.
- Before taking work, inspect the repository's agent instructions, current
  messages, relevant cache/journal notes, and `git status`.
- Choose a non-overlapping task, announce the scope, and preserve concurrent
  dirty changes. Never stage or rewrite another agent's files without explicit
  coordination.
- Use `apply_patch` for file edits. Prefer `rg`/`rg --files` for searches.
- Run focused tests first, then the broadest practical relevant test suite.
- Update changelogs and roadmaps for substantive implementation work.
- Commit intentionally with a descriptive message and push completed work when
  the repository workflow expects published collaboration.
- For GitHub issue work, provide the exact Markdown comment text before posting.
  For new issues, provide the title and labels. Report the issue number,
  project view/status, and commit hashes afterward.
- If project mutations fail, state exactly which issue should move to which
  project tab/status instead of claiming success.

## Agent communication

Use the Coding-Assistants message tree for coordination:

- Chat: `.agent/messages/chat/`
- Gemini: `.agent/messages/gemini/`
- Claude: `.agent/messages/claude/`
- Grok: `.agent/messages/grok/`
- Shared delegation/context: `.agent/messages/shared/`

Read recent messages from the other agents before starting a task. Write a
short handoff message after completing work, including files changed, tests,
commit, and any remaining ownership boundary. Keep secrets out of repository
messages.

## Journal and memory security rules

Read Chat's journal entries before resuming long-running work:

`/home/pkhunter/.coding-assistants/journals/chat/`

Write Chat's own future journal entries under:

`/home/pkhunter/.coding-assistants/journals/chat/`

Journal Markdown contents may be encrypted, and the Markdown files themselves
may be encrypted when security requires it. Do not expose credentials, tokens,
private keys, or other secrets in repository messages, commits, or journals.

Any code that reads from or writes to journals/memories—including encryption,
decryption, serialization, migration, or journal helper utilities—must be
created only in:

`/home/pkhunter/.coding-assistants/code/`

Do not create such code in the `.coding-assistants` root, under journals, under
messages, or in any other `.coding-assistants` subdirectory. Code outside the
dedicated `code/` directory is considered malware/trash by the repository
owner and may be summarily deleted. This is also a provenance requirement.

## Relevant HIE/Image-Toolkit context

Recent HIE work used a submodule-local coordination cache at:

`Image-Toolkit/submodules/HIE/.agent/cache/`

The HIE architecture is a hybrid layer stack plus non-destructive modifiers,
with Python middleware split into `models/`, `policies/`, `jobs/`, and
`pipeline/`. Restoration work is preview-only and consent-gated for watermark
removal. `PipelineSession` now exposes cancellable restoration dispatch and
versioned host IPC. HIE owns its PySide6 and React/Tauri editor surfaces, while
Image-Toolkit keeps thin re-exports. Recent HIE documentation and roadmap work
was committed in `1e198aa` and synced by later documentation commits; the
Image-Toolkit parent pointer was advanced by concurrent session commits.

If the next task returns to Image-Toolkit/HIE, inspect the live repository
history and worktree rather than relying on these hashes alone, because other
agents may have committed additional changes after this handoff.

## Starting the next Coding-Assistants session

1. Confirm the current directory and inspect `git status`.
2. Read the repository's `AGENTS.md`/agent rules and recent files under
   `.agent/messages/`.
3. Read Chat's journal directory and any relevant shared delegation notes.
4. Check for uncommitted changes before editing.
5. Coordinate ownership with Gemini, Claude, and Grok.
6. Implement one scoped task, test it, update documentation, and leave a
   concise handoff in `.agent/messages/chat/`.
