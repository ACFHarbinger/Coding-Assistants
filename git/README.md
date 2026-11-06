# git/

Human-browsable automation suite backing `.github/workflows/agent_sync.yml` and,
the analogous workflows in `.gitlab-ci.yml` and `.gitea/workflows/agent-sync.yml`.

| Directory | Purpose |
| --- | --- |
| `config/` | `automation_rules.yaml` (policy DSL) and `project_labels.json` (label taxonomy) |
| `scripts/` | `agent_tools.py` (ProjectV2 GraphQL client), `sync_backlog.py` (roadmap→board reconciler), `check_commit_ref.py` (commit-message ticket linker) |
| `hooks/` | Local git hooks (`pre-commit`, `post-commit`) plus `install.sh` to symlink them into `.git/hooks/` |

## Setup

```bash
bash git/hooks/install.sh
export PROJECT_ID="PVT_..."      # ProjectV2 node ID, see `gh project view <n> --owner <o> --format json`
export GITHUB_TOKEN="..."        # GitHub token with repo + project scopes
export GITEA_TOKEN="..."         # Gitea token with repo + project scopes
export GITLAB_TOKEN="..."        # GitLab token with repo + project scopes
export GEMINI_API_KEY="..."      # Google Gemini API key with project + roadmap scopes
```

## CI

`.github/workflows/agent_sync.yml` runs `git/scripts/sync_backlog.py` on
every push to `docs/moon/ROADMAP.md` or `docs/moon/CHANGELOG.md`, or on demand via
`workflow_dispatch`. It needs two repository secrets
(`PROJECT_AUTOMATION_TOKEN`, `GEMINI_API_KEY`) and one repository variable
(`PROJECT_ID`) configured before it can mutate a live board — until then,
runs will fail fast rather than silently no-op.

> **TODO:** update `git/config/automation_rules.yaml`'s `project.number` once
> the real Coding-Assistants Project (V2) board is created.
