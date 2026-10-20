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
  isWebHarness,
  maxKey,
  storageKey,
  terminalSessionId,
  type CanvasSize,
} from "./gridConstants";
import type { DragTargetZone } from "./useTerminalGridDrag";

export interface WebHarnessSession {
  harness: string;
  workspace: string;
  url: string;
  port: number;
  pid: number | null;
  detail: string;
}

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
  const [webUrls, setWebUrls] = useState<Record<string, string>>({});
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
          setWebUrls({});
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
          if (isWebHarness(leaf.harness)) {
            try {
              const session = await invoke<WebHarnessSession | null>("web_harness_status", {
                harness: leaf.harness,
                workspace,
              });
              if (session?.url) {
                return { leafId: leaf.id, harness: leaf.harness, sessionId: null, url: session.url };
              }
            } catch {
              /* probe errors drop the pane */
            }
            return null;
          }
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
          return foundSid ? { leafId: leaf.id, harness: leaf.harness, sessionId: foundSid, url: null } : null;
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
      setTerminals(
        Object.fromEntries(
          live.flatMap(({ leafId, sessionId }) => (sessionId ? [[leafId, sessionId] as const] : [])),
        ),
      );
      setWebUrls(
        Object.fromEntries(live.flatMap(({ leafId, url }) => (url ? [[leafId, url] as const] : []))),
      );

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
      if (isWebHarness(requestedHarness)) {
        try {
          const session = await invoke<WebHarnessSession | null>("web_harness_status", {
            harness: requestedHarness,
            workspace,
          });
          if (!disposed && session?.url) {
            setLayout((prev) => {
              const { tree, leafId } = insertLeaf(prev, requestedHarness);
              setWebUrls((urls) => ({ ...urls, [leafId]: session.url }));
              return tree;
            });
          }
        } finally {
          if (!disposed) onHarnessRequestHandled?.();
        }
        return;
      }
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
      const web = isWebHarness(harness);
      setStatusMsg(`Starting ${DISPLAY_NAMES[harness] || harness} ${web ? "web app" : "terminal"}…`);
      try {
        if (web) {
          const session = await invoke<WebHarnessSession>("web_harness_start", { harness, workspace });
          if (!session?.url) throw new Error("DeepSeek web harness did not return a URL.");
          setWebUrls((prev) => ({ ...prev, [leafId]: session.url }));
          if (!existingLeafId) {
            setLayout((prev) => insertLeaf(prev, harness, undefined, undefined, leafId).tree);
          }
          setStatusMsg(session.detail || `${DISPLAY_NAMES[harness] || harness} web app ready.`);
          return;
        }
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
      const closing = collectLeaves(layout).find((leaf) => leaf.id === leafId);
      setWebUrls((prev) => {
        if (!prev[leafId]) return prev;
        const next = { ...prev };
        delete next[leafId];
        return next;
      });
      if (closing && isWebHarness(closing.harness)) {
        const stillOpen = collectLeaves(layout).some(
          (leaf) => leaf.id !== leafId && leaf.harness === closing.harness,
        );
        if (!stillOpen) {
          try {
            await invoke("web_harness_stop", { harness: closing.harness, workspace });
          } catch {
            /* ignore */
          }
        }
      }
      setLayout((prev) => removeLeaf(prev, leafId));
      setMaximizedId((prev) => (prev === leafId ? null : prev));
    },
    [terminals, layout, workspace],
  );

  const resetGrid = useCallback(async () => {
    const webHarnesses = new Set(
      collectLeaves(layout).filter((leaf) => isWebHarness(leaf.harness)).map((leaf) => leaf.harness),
    );
    await Promise.allSettled([
      ...Object.values(terminals).map((sessionId) => invoke("pty_kill", { sessionId })),
      ...[...webHarnesses].map((harness) => invoke("web_harness_stop", { harness, workspace })),
    ]);
    setTerminals({});
    setWebUrls({});
    setLayout(null);
    setMaximizedId(null);
    setStatusMsg("All harness panes closed.");
  }, [terminals, layout, workspace]);

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
    webUrls,
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
