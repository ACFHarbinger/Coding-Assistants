# Python Rules (repo tooling scripts only)

This repo has no Python application code — Python is used only for small utility
scripts under `git/scripts/` (backlog sync, commit-ref checks) and
`docs/website/generate_docs_json.py`.

- Target Python 3.11+, no external dependency manager required unless a script's
  imports demand one — keep these scripts dependency-light.
- Format and lint with `ruff` (`ruff format`, `ruff check --fix`) if `ruff` is available.
- Use `pathlib.Path` instead of `os.path`.
- Log with `print`/`logging` as appropriate for a short CLI utility — these are not
  long-running services, so don't over-engineer them with DI or class hierarchies.
