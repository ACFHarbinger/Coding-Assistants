import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { EmbeddedRelaunchOutcome, HarnessSessionRegistration, PtySessionStatus } from "../harness/types";
import {
  computeRects, insertLeaf, removeLeaf, resizeSplit, swapLeaves, moveLeaf,
  collectLeaves, serializeLayout, deserializeLayout,
  type LayoutNode, type Rect, type SplitterInfo,
} from "./layoutTree";
import { useTerminalGridDrag, type DragTargetZone } from "./useTerminalGridDrag";
import { useGridCanvasResize } from "./useGridCanvasResize";
import CanvasResizeHandles from "./CanvasResizeHandles";
import DropZoneOverlay from "./DropZoneOverlay";
import TerminalPane from "./TerminalPane";
import GridLayoutMenu from "./GridLayoutMenu";
import {
  ALL_HARNESSES, DISPLAY_NAMES, storageKey, maxKey, canvasSizeKey, terminalSessionId,
  type CanvasSize,
} from "./gridConstants";

export interface HarnessTerminalGridProps {
  workspace: string;
  onOpenSetup?: () => void;
  requestedHarness?: string | null;
  onHarnessRequestHandled?: () => void;
}

export default function HarnessTerminalGrid({
  workspace,
  onOpenSetup,
  requestedHarness,
  onHarnessRequestHandled,
}: HarnessTerminalGridProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [bounds, setBounds] = useState<Rect>({ x: 0, y: 0, width: 900, height: 600 });
  const [layout, setLayout] = useState<LayoutNode | null>(() => {
    try { return deserializeLayout(localStorage.getItem(storageKey(workspace))); } catch { return null; }
  });
  const [maximizedHarness, setMaximizedHarness] = useState<string | null>(() => {
    try { return localStorage.getItem(maxKey(workspace)); } catch { return null; }
  });

  const [terminals, setTerminals] = useState<Record<string, string>>({});
  const [busyHarness, setBusyHarness] = useState<string | null>(null);
  const [error, setError] = useState<string>("");
  const [statusMsg, setStatusMsg] = useState<string>("");

  const { dragState, startPaneDrag } = useTerminalGridDrag();

  const handleCanvasSizeChange = useCallback((size: CanvasSize | null) => {
    if (size) {
      setBounds((prev) => ({ ...prev, width: size.width, height: size.height }));
    }
  }, []);

  const { canvasSize, setCanvasSize, startResize, resetCanvasSize } = useGridCanvasResize({
    workspace,
    containerRef,
    onSizeChange: handleCanvasSizeChange,
  });

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
          setMaximizedHarness(null);
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
          const sessionId = terminalSessionId(leaf.harness, workspace);
          try {
            const status = await invoke<PtySessionStatus>("pty_session_status", { sessionId });
            return status.found ? { harness: leaf.harness, sessionId } : null;
          } catch {
            return null;
          }
        }),
      );
      if (disposed) return;
      const live = statuses.flatMap((entry) => (entry ? [entry] : []));
      const liveHarnesses = new Set(live.map((entry) => entry.harness));
      const pruned = leaves.reduce<LayoutNode | null>(
        (tree, leaf) => (liveHarnesses.has(leaf.harness) ? tree : removeLeaf(tree, leaf.id)),
        restored,
      );
      setLayout(pruned);
      setTerminals(Object.fromEntries(live.map(({ harness, sessionId }) => [harness, sessionId])));
      setMaximizedHarness(
        restoredMaximized && liveHarnesses.has(restoredMaximized) ? restoredMaximized : null,
      );
    };
    void restoreLayout();
    return () => { disposed = true; };
  }, [workspace]);

  useEffect(() => {
    if (!requestedHarness) return;
    let disposed = false;
    const attachRequestedHarness = async () => {
      const sessionId = terminalSessionId(requestedHarness, workspace);
      try {
        const status = await invoke<PtySessionStatus>("pty_session_status", { sessionId });
        if (!disposed && status.found) {
          setTerminals((prev) => ({ ...prev, [requestedHarness]: sessionId }));
          setLayout((prev) => insertLeaf(prev, requestedHarness));
        }
      } finally {
        if (!disposed) onHarnessRequestHandled?.();
      }
    };
    void attachRequestedHarness();
    return () => { disposed = true; };
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
      if (maximizedHarness) localStorage.setItem(maxKey(workspace), maximizedHarness);
      else localStorage.removeItem(maxKey(workspace));
    } catch {
      /* ignore quota */
    }
  }, [workspace, maximizedHarness]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMaximizedHarness(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0) {
          setBounds({ x: 0, y: 0, width: Math.round(width), height: Math.round(height) });
        }
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const launchHarness = useCallback(
    async (harness: string) => {
      if (!workspace.startsWith("/")) {
        setError("Set an absolute Workspace Root before launching a harness terminal.");
        return;
      }
      setBusyHarness(harness);
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
        });
        const sid = outcome?.sessionId ?? (outcome as { session_id?: string })?.session_id;
        if (!sid) throw new Error("Relaunch did not return an in-app terminal session id.");
        setTerminals((prev) => ({ ...prev, [harness]: sid }));
        setLayout((prev) => insertLeaf(prev, harness));
        setStatusMsg(outcome.detail || `${DISPLAY_NAMES[harness] || harness} connected.`);
      } catch (cause) {
        setError(String(cause).replace(/^Error:\s*/, ""));
      } finally {
        setBusyHarness(null);
      }
    },
    [workspace],
  );

  const closePane = useCallback(async (harness: string) => {
    const sid = terminals[harness];
    if (sid) {
      try { await invoke("pty_kill", { sessionId: sid }); } catch { /* ignore */ }
      setTerminals((prev) => {
        const next = { ...prev };
        delete next[harness];
        return next;
      });
    }
    setLayout((prev) => removeLeaf(prev, harness));
    setMaximizedHarness((prev) => (prev === harness ? null : prev));
  }, [terminals]);

  const resetGrid = useCallback(async () => {
    await Promise.allSettled(
      Object.values(terminals).map((sessionId) => invoke("pty_kill", { sessionId })),
    );
    setTerminals({});
    setLayout(null);
    setMaximizedHarness(null);
    setStatusMsg("All harness terminal panes closed.");
  }, [terminals]);

  const handleLoadLayout = useCallback((loadedLayout: LayoutNode, loadedCanvas: CanvasSize | null) => {
    setLayout(loadedLayout);
    if (loadedCanvas) {
      setCanvasSize(loadedCanvas);
      try {
        localStorage.setItem(canvasSizeKey(workspace), JSON.stringify(loadedCanvas));
      } catch {
        /* ignore */
      }
    } else {
      resetCanvasSize();
    }
    setMaximizedHarness(null);
    setStatusMsg("Layout loaded.");
  }, [workspace, resetCanvasSize, setCanvasSize]);

  const handleDrop = useCallback((source: string, target: string, zone: DragTargetZone) => {
    setLayout((prev) => (prev ? (zone === "center" ? swapLeaves(prev, source, target) : moveLeaf(prev, source, target, zone)) : null));
  }, []);

  const startSplitterDrag = useCallback(
    (event: React.PointerEvent<HTMLDivElement>, splitter: SplitterInfo) => {
      if (event.button !== 0) return;
      const handle = event.currentTarget;
      handle.setPointerCapture(event.pointerId);
      const startX = event.clientX;
      const startY = event.clientY;
      const initialRatio = splitter.splitNode.ratio;
      const parentBounds = splitter.parentBounds;

      const onMove = (e: PointerEvent) => {
        const deltaPx = splitter.direction === "row" ? e.clientX - startX : e.clientY - startY;
        const totalPx = splitter.direction === "row" ? parentBounds.width : parentBounds.height;
        if (totalPx > 0) {
          const newRatio = initialRatio + deltaPx / totalPx;
          setLayout((prev) => (prev ? resizeSplit(prev, splitter.id, newRatio) : null));
        }
      };
      const onUp = () => {
        try { handle.releasePointerCapture(event.pointerId); } catch { /* ignore */ }
        handle.removeEventListener("pointermove", onMove);
        handle.removeEventListener("pointerup", onUp);
        handle.removeEventListener("pointercancel", onUp);
      };
      handle.addEventListener("pointermove", onMove);
      handle.addEventListener("pointerup", onUp);
      handle.addEventListener("pointercancel", onUp);
    },
    [],
  );

  const openLeaves = collectLeaves(layout);
  const openHarnesses = new Set(openLeaves.map((l) => l.harness));
  const availableHarnesses = ALL_HARNESSES.filter((h) => !openHarnesses.has(h));
  const { leaves, splitters } = computeRects(layout, bounds, 6);

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, gap: "0.75rem", height: "100%" }}>
      {/* Palette bar */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.75rem", padding: "0.5rem 0.85rem", borderRadius: "10px", background: "rgba(0, 0, 0, 0.3)", border: "1px solid var(--border-color)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
          <span style={{ fontSize: "0.8rem", fontWeight: 700, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
            + Add pane:
          </span>
          {availableHarnesses.map((h) => (
            <button
              key={h}
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }}
              disabled={busyHarness !== null}
              onClick={() => void launchHarness(h)}
            >
              + {DISPLAY_NAMES[h]}
            </button>
          ))}
          {availableHarnesses.length === 0 && (
            <span style={{ fontSize: "0.78rem", color: "var(--text-muted)", fontStyle: "italic" }}>All 6 harnesses open</span>
          )}
          {maximizedHarness && (
            <span style={{ fontSize: "0.78rem", color: "#93c5fd", fontWeight: 600, background: "rgba(59, 130, 246, 0.18)", padding: "0.2rem 0.5rem", borderRadius: "6px", border: "1px solid rgba(59, 130, 246, 0.35)" }}>
              Maximized: {DISPLAY_NAMES[maximizedHarness] || maximizedHarness} (Esc to restore)
            </span>
          )}
          {canvasSize && !maximizedHarness && (
            <span style={{ fontSize: "0.78rem", color: "var(--text-muted)", padding: "0.2rem 0.45rem", borderRadius: "4px", background: "rgba(255, 255, 255, 0.05)" }}>
              Grid: {canvasSize.width}×{canvasSize.height}px
            </span>
          )}
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          {maximizedHarness && (
            <button
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }}
              onClick={() => setMaximizedHarness(null)}
              title="Restore grid layout"
            >
              Restore grid
            </button>
          )}
          {canvasSize && (
            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }} onClick={resetCanvasSize} title="Reset grid size to fit window">
              Fit to window
            </button>
          )}
          <GridLayoutMenu
            currentLayout={layout}
            currentCanvas={canvasSize}
            onLoadLayout={handleLoadLayout}
            onError={(err) => setError(err)}
            onStatus={(msg) => setStatusMsg(msg)}
          />
          {openLeaves.length > 0 && (
            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }} onClick={() => void resetGrid()} title="Close all panes and reset layout">
              Reset grid
            </button>
          )}
          {onOpenSetup && (
            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }} onClick={onOpenSetup}>
              ⚙️ Harness Setup
            </button>
          )}
        </div>
      </div>

      {error && (
        <div style={{ padding: "0.5rem 0.75rem", borderRadius: "8px", background: "rgba(239, 68, 68, 0.15)", border: "1px solid rgba(248, 113, 113, 0.5)", color: "#fecaca", fontSize: "0.82rem" }}>
          {error}
        </div>
      )}
      {statusMsg && !error && (
        <div style={{ padding: "0.4rem 0.75rem", borderRadius: "8px", background: "rgba(16, 185, 129, 0.12)", border: "1px solid rgba(16, 185, 129, 0.35)", color: "#a7f3d0", fontSize: "0.82rem" }}>
          {statusMsg}
        </div>
      )}

      {/* Grid Canvas */}
      <div
        ref={containerRef}
        style={{
          position: "relative",
          flex: canvasSize ? "none" : 1,
          width: canvasSize ? `${canvasSize.width}px` : "100%",
          height: canvasSize ? `${canvasSize.height}px` : undefined,
          maxWidth: "100%",
          minHeight: canvasSize ? "280px" : "480px",
          borderRadius: "10px",
          background: "#030712",
          border: "1px solid var(--border-color)",
          overflow: "hidden",
        }}
      >
        {openLeaves.length === 0 ? (
          <div style={{ position: "absolute", inset: 0, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "1rem", padding: "2rem", textAlign: "center" }}>
            <div style={{ fontSize: "1.05rem", fontWeight: 600, color: "var(--text-main)" }}>No active harness terminals in grid</div>
            <p style={{ maxWidth: "440px", color: "var(--text-muted)", fontSize: "0.85rem", margin: 0 }}>
              Launch any of the 6 supported harness CLIs in an in-app interactive terminal pane. Drag title bars to rearrange or swap panes; drag splitters or borders to resize.
            </p>
            <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", justifyContent: "center", marginTop: "0.5rem" }}>
              {ALL_HARNESSES.map((h) => (
                <button
                  key={h}
                  type="button"
                  className="btn-primary"
                  style={{ marginTop: 0, padding: "0.45rem 0.85rem", fontSize: "0.85rem" }}
                  disabled={busyHarness !== null}
                  onClick={() => void launchHarness(h)}
                >
                  Launch {DISPLAY_NAMES[h]}
                </button>
              ))}
            </div>
          </div>
        ) : (
          <>
            {/* Flat layer of always-mounted panes positioned via CSS rects */}
            {leaves.map(({ node, rect }) => {
              const sid = terminals[node.harness];
              const isBusy = busyHarness === node.harness;
              const isMaximized = maximizedHarness === node.harness;
              const isHidden = maximizedHarness !== null && !isMaximized;

              return (
                <TerminalPane
                  key={node.harness}
                  node={node}
                  rect={rect}
                  bounds={bounds}
                  sessionId={sid}
                  isBusy={isBusy}
                  isMaximized={isMaximized}
                  isHidden={isHidden}
                  hasMaximized={maximizedHarness !== null}
                  isDragging={dragState.isDragging && dragState.sourceHarness === node.harness}
                  onTitlePointerDown={(e) => {
                    if (!maximizedHarness) {
                      startPaneDrag(e, node.harness, node.id, () => leaves, containerRef.current, handleDrop);
                    }
                  }}
                  onConnect={() => void launchHarness(node.harness)}
                  onToggleMaximize={() => setMaximizedHarness(isMaximized ? null : node.harness)}
                  onClose={() => void closePane(node.harness)}
                  onExit={(detail) => setStatusMsg(`${DISPLAY_NAMES[node.harness] || node.harness} exit: ${detail}`)}
                  onError={(detail) => setError(`${DISPLAY_NAMES[node.harness] || node.harness} error: ${detail}`)}
                />
              );
            })}

            {/* Splitter Dividers - only when not maximized */}
            {!maximizedHarness && splitters.map((splitter) => (
              <div
                key={splitter.id}
                role="separator"
                aria-orientation={splitter.direction === "row" ? "vertical" : "horizontal"}
                title="Drag to resize pane"
                onPointerDown={(e) => startSplitterDrag(e, splitter)}
                style={{
                  position: "absolute",
                  left: `${splitter.rect.x}px`,
                  top: `${splitter.rect.y}px`,
                  width: `${splitter.rect.width}px`,
                  height: `${splitter.rect.height}px`,
                  cursor: splitter.direction === "row" ? "col-resize" : "row-resize",
                  background: "rgba(100, 116, 139, 0.25)",
                  zIndex: 10,
                  userSelect: "none",
                  touchAction: "none",
                }}
              />
            ))}

            {/* Drop zone overlay during pane drag */}
            {dragState.isDragging && dragState.dropTarget && (
              <DropZoneOverlay
                target={dragState.dropTarget}
                displayName={DISPLAY_NAMES[dragState.dropTarget.harness] || dragState.dropTarget.harness}
              />
            )}
          </>
        )}

        {/* Canvas border resize handles (right, bottom, corner) - only when not maximized */}
        {!maximizedHarness && <CanvasResizeHandles onStartResize={startResize} />}
      </div>
    </div>
  );
}
