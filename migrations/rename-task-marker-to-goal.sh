#!/usr/bin/env bash
# One-shot rewrite of the chat Task marker to Goal (#380).
# Run once on each PC, with Coding Assistants quit, so hub.db is not open.
# Safe to run again: rows that already say [GOAL] are left alone.
# Historical audit records are hash-chained and must remain unchanged.
# Any renamed audit labels belong in presentation, not the signed payload.
#
#   migrations/rename-task-marker-to-goal.sh
#   migrations/rename-task-marker-to-goal.sh /path/to/.coding-assistants
#
# Default data dir: $CA_HOME, or $HOME/.coding-assistants.

set -euo pipefail

root="${1:-${CA_HOME:-${HOME}/.coding-assistants}}"
db="${root}/hub.db"

if [[ ! -f "${db}" ]]; then
  echo "No hub database at ${db}" >&2
  exit 1
fi
echo "Rewriting [TASK] -> [GOAL] in ${db}"
echo "Quit Coding Assistants on this PC before continuing."

python3 - "${db}" <<'PY'
import sqlite3, sys
db = sys.argv[1]
conn = sqlite3.connect(db, timeout=5)
conn.execute("PRAGMA busy_timeout = 5000")
statements = [
    "UPDATE messages SET body = REPLACE(body, '[TASK]', '[GOAL]') WHERE instr(body, '[TASK]') > 0",
    "UPDATE messages SET subject = REPLACE(subject, '[TASK]', '[GOAL]') WHERE subject IS NOT NULL AND instr(subject, '[TASK]') > 0",
    "UPDATE messages SET subject = REPLACE(subject, ':kind:task', ':kind:goal') WHERE subject IS NOT NULL AND instr(subject, ':kind:task') > 0",
    "UPDATE messages SET task_id = REPLACE(task_id, '[TASK]', '[GOAL]') WHERE task_id IS NOT NULL AND instr(task_id, '[TASK]') > 0",
    "UPDATE messages SET task_id = REPLACE(task_id, ':kind:task', ':kind:goal') WHERE task_id IS NOT NULL AND instr(task_id, ':kind:task') > 0",
    "UPDATE wake_requests SET reason = REPLACE(reason, '[TASK]', '[GOAL]') WHERE reason IS NOT NULL AND instr(reason, '[TASK]') > 0",
    "UPDATE tagged_send_outcomes SET subject = REPLACE(subject, ':kind:task', ':kind:goal') WHERE instr(subject, ':kind:task') > 0",
    "UPDATE tagged_send_outcomes SET subject = REPLACE(subject, '[TASK]', '[GOAL]') WHERE instr(subject, '[TASK]') > 0",
    "UPDATE tagged_send_outcomes SET reason = REPLACE(reason, '[TASK]', '[GOAL]') WHERE reason IS NOT NULL AND instr(reason, '[TASK]') > 0",
]
present = {row[0] for row in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table'")}
needed = {"messages", "wake_requests", "tagged_send_outcomes"}
missing = sorted(needed - present)
if missing:
    raise SystemExit(f"hub.db is missing tables: {', '.join(missing)}")
with conn:
    for statement in statements:
        conn.execute(statement)
conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
rows = conn.execute(
    """
    SELECT 'messages.body [TASK]', COUNT(*) FROM messages WHERE instr(body, '[TASK]') > 0
    UNION ALL
    SELECT 'messages.subject :kind:task', COUNT(*) FROM messages WHERE subject IS NOT NULL AND instr(subject, ':kind:task') > 0
    UNION ALL
    SELECT 'messages.body [GOAL]', COUNT(*) FROM messages WHERE instr(body, '[GOAL]') > 0
    """
).fetchall()
for label, count in rows:
    print(f"{label}: {count}")
conn.close()
PY

python3 - "${root}" <<'PY'
import pathlib, sys
root = pathlib.Path(sys.argv[1])
changed = 0
for folder in ("journals", "markdown"):
    base = root / folder
    if not base.is_dir():
        continue
    for path in base.rglob("*"):
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        updated = text.replace("[TASK]", "[GOAL]").replace(":kind:task", ":kind:goal")
        if updated != text:
            path.write_text(updated, encoding="utf-8")
            changed += 1
            print(f"rewrote {path}")
print(f"text files rewritten: {changed}")
PY

echo "Done. Remaining chat [TASK] counts are in the sqlite report above."
echo "Historical audit records were left unchanged to preserve their hash chain."
