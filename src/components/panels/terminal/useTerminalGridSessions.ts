import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { EmbeddedRelaunchOutcome, HarnessSessionRegistration, PtySessionStatus } from "../harness/types";
import {
  collectLeaves,
  deserializeLayout,
  generateNodeId,
  insertLeaf,
  moveLeaf,
  removeLeaf,
  serializeLayout,
  swapLeaves,
  type LayoutNode,
} from "./layoutTree";
import {
  canvasSizeKey,
  DISPLAY_NAMES,
  maxKey,
  storageKey,
  terminalSessionId,
  type CanvasSize,
} from "./gridConstants";
import type { DragTargetZone } from "./useTerminalGridDrag";

export interface UseTerminalGridSessionsOptions {
  workspace: string;
  requestedHarness?: string | null;
  onHarnessRequestHandled?: () => void;
  resetCanvasSize?: () => void;
  setCanvasSize?: (size: CanvasSize) => void;
}

export function useTerminalGridSessions({
  workspace,
  requestedHarness,
  onHarnessRequestHandled,
  resetCanvasSize,
  setCanvasSize,
}: UseTerminalGridSessionsOptions) {
  const [layout, setLayout] = useState<LayoutNode | null>(() => {
    try {
      return deserializeLayout(localStorage.getItem(storageKey(workspace)));
    } catch {
      return null;
    }
  });

  const [maximizedId, setMaximizedId] = useState<string | null>(() => {
    try {
      return localStorage.getItem(maxKey(workspace));
    } catch {
      return null;
    }
  });

  const [terminals, setTerminals] = useState<Record<string, string>>({});
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState<string>("");
  const [statusMsg, setStatusMsg] = useState<string>("");

  useEffect(() => {
    let disposed = false;
    const restoreLayout = async () => {
      let restored: LayoutNode | null = null;
      try {
        restored = deserializeLayout(localStorage.getItem(storageKey(workspace)));
      } catch {
        /* storage disabled */
      }
      if (!restored) {
        if (!disposed) {
          setLayout(null);
          setTerminals({});
          setMaximizedId(null);
        }
        return;
      }

      let restoredMaximized: string | null = null;
      try {
        restoredMaximized = localStorage.getItem(maxKey(workspace));
      } catch {
        /* ignore */
      }

      const leaves = collectLeaves(restored);
      const statuses = await Promise.all(
        leaves.map(async (leaf) => {
          const instanceSid = terminalSessionId(leaf.harness, workspace, leaf.id);
          const legacySid = terminalSessionId(leaf.harness, workspace);
          let foundSid: string | null = null;
          try {
            const st1 = await invoke<PtySessionStatus>("pty_session_status", { sessionId: instanceSid });
            if (st1.found) {
              foundSid = instanceSid;
            } else {
              const st2 = await invoke<PtySessionStatus>("pty_session_status", { sessionId: legacySid });
              if (st2.found) {
                foundSid = legacySid;
              }
            }
          } catch {
            // ignore probe errors
          }
          return foundSid ? { leafId: leaf.id, harness: leaf.harness, sessionId: foundSid } : null;
        }),
      );

      if (disposed) return;
      const live = statuses.flatMap((entry) => (entry ? [entry] : []));
      const liveLeafIds = new Set(live.map((entry) => entry.leafId));
      const pruned = leaves.reduce<LayoutNode | null>(
        (tree, leaf) => (liveLeafIds.has(leaf.id) ? tree : removeLeaf(tree, leaf.id)),
        restored,
      );

      setLayout(pruned);
      setTerminals(Object.fromEntries(live.map(({ leafId, sessionId }) => [leafId, sessionId])));

      if (restoredMaximized) {
        // Layouts saved before #301 stored the harness name. Convert that
        // legacy value to the live leaf ID rather than hiding every pane.
        const maximizedLeaf = live.find(
          (entry) => entry.leafId === restoredMaximized || entry.harness === restoredMaximized,
        );
        setMaximizedId(maximizedLeaf?.leafId ?? null);
      } else {
        setMaximizedId(null);
      }
    };

    void restoreLayout();
    return () => {
      disposed = true;
    };
  }, [workspace]);

  useEffect(() => {
    if (!requestedHarness) return;
    let disposed = false;
    const attachRequestedHarness = async () => {
      const sessionId = terminalSessionId(requestedHarness, workspace);
      try {
        const status = await invoke<PtySessionStatus>("pty_session_status", { sessionId });
        if (!disposed && status.found) {
          setLayout((prev) => {
            const { tree, leafId } = insertLeaf(prev, requestedHarness);
            setTerminals((tPrev) => ({ ...tPrev, [leafId]: sessionId }));
            return tree;
          });
        }
      } finally {
        if (!disposed) onHarnessRequestHandled?.();
      }
    };
    void attachRequestedHarness();
    return () => {
      disposed = true;
    };
  }, [workspace, requestedHarness, onHarnessRequestHandled]);

  useEffect(() => {
    try {
      if (layout) localStorage.setItem(storageKey(workspace), serializeLayout(layout));
      else localStorage.removeItem(storageKey(workspace));
    } catch {
      /* ignore quota */
    }
  }, [workspace, layout]);

  useEffect(() => {
    try {
      if (maximizedId) localStorage.setItem(maxKey(workspace), maximizedId);
      else localStorage.removeItem(maxKey(workspace));
    } catch {
      /* ignore quota */
    }
  }, [workspace, maximizedId]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMaximizedId(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const launchHarness = useCallback(
    async (harness: string, existingLeafId?: string) => {
      if (!workspace.startsWith("/")) {
        setError("Set an absolute Workspace Root before launching a harness terminal.");
        return;
      }
      const leafId = existingLeafId ?? generateNodeId(`leaf-${harness}`);
      setBusyId(leafId);
      setError("");
      setStatusMsg(`Starting ${DISPLAY_NAMES[harness] || harness} terminal…`);
      try {
        let existingPid: number | null = null;
        try {
          const sessions = await invoke<HarnessSessionRegistration[]>("hub_list_harness_sessions");
          const row = sessions.find((s) => s.harness === harness && s.workspace === workspace);
          if (row?.managed_pid) existingPid = row.managed_pid;
        } catch {
          // ignore
        }
        const outcome = await invoke<EmbeddedRelaunchOutcome>("hub_relaunch_harness_embedded", {
          harness,
          workspace,
          existingPid,
          instanceKey: leafId,
        });
        const sid = outcome?.sessionId ?? (outcome as { session_id?: string })?.session_id;
        if (!sid) throw new Error("Relaunch did not return an in-app terminal session id.");
        setTerminals((prev) => ({ ...prev, [leafId]: sid }));
        if (!existingLeafId) {
          setLayout((prev) => insertLeaf(prev, harness, undefined, undefined, leafId).tree);
        }
        setStatusMsg(outcome.detail || `${DISPLAY_NAMES[harness] || harness} connected.`);
      } catch (cause) {
        setError(String(cause).replace(/^Error:\s*/, ""));
      } finally {
        setBusyId(null);
      }
    },
    [workspace],
  );

  const closePane = useCallback(
    async (leafId: string) => {
      const sid = terminals[leafId];
      if (sid) {
        try {
          await invoke("pty_kill", { sessionId: sid });
        } catch {
          /* ignore */
        }
        setTerminals((prev) => {
          const next = { ...prev };
          delete next[leafId];
          return next;
        });
      }
      setLayout((prev) => removeLeaf(prev, leafId));
      setMaximizedId((prev) => (prev === leafId ? null : prev));
    },
    [terminals],
  );

  const resetGrid = useCallback(async () => {
    await Promise.allSettled(
      Object.values(terminals).map((sessionId) => invoke("pty_kill", { sessionId })),
    );
    setTerminals({});
    setLayout(null);
    setMaximizedId(null);
    setStatusMsg("All harness terminal panes closed.");
  }, [terminals]);

  const handleLoadLayout = useCallback(
    (loadedLayout: LayoutNode, loadedCanvas: CanvasSize | null) => {
      setLayout(loadedLayout);
      if (loadedCanvas) {
        setCanvasSize?.(loadedCanvas);
        try {
          localStorage.setItem(canvasSizeKey(workspace), JSON.stringify(loadedCanvas));
        } catch {
          /* ignore */
        }
      } else {
        resetCanvasSize?.();
      }
      setMaximizedId(null);
      setStatusMsg("Layout loaded.");
    },
    [workspace, resetCanvasSize, setCanvasSize],
  );

  const handleDrop = useCallback(
    (sourceLeafId: string, targetLeafId: string, zone: DragTargetZone) => {
      setLayout((prev) =>
        prev
          ? zone === "center"
            ? swapLeaves(prev, sourceLeafId, targetLeafId)
            : moveLeaf(prev, sourceLeafId, targetLeafId, zone)
          : null,
      );
    },
    [],
  );

  return {
    layout,
    setLayout,
    terminals,
    maximizedId,
    setMaximizedId,
    busyId,
    error,
    setError,
    statusMsg,
    setStatusMsg,
    launchHarness,
    closePane,
    resetGrid,
    handleLoadLayout,
    handleDrop,
  };
}
