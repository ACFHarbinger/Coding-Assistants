import { invoke } from "../../../lib/tauri";

// Serialize each native view's lifetime, including React StrictMode remounts.
// An old close must complete before the same leaf can open a new view.
const pending = new Map<string, Promise<void>>();
export function browserPaneCommand(leafId: string, command: string, args: Record<string, unknown> | (() => Record<string, unknown>) = {}): Promise<void> {
  const next = (pending.get(leafId) ?? Promise.resolve()).catch(() => undefined)
    .then(() => invoke<void>(command, { ...(typeof args === "function" ? args() : args), leafId }));
  pending.set(leafId, next);
  void next.finally(() => {
    if (pending.get(leafId) === next) pending.delete(leafId);
  }).catch(() => undefined);
  return next;
}
