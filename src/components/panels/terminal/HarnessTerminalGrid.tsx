import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  computeRects,
  resizeSplit,
  collectLeaves,
  type Rect,
  type SplitterInfo,
} from "./layoutTree";
import { useTerminalGridDrag } from "./useTerminalGridDrag";
import { useGridCanvasResize } from "./useGridCanvasResize";
import { useTerminalGridSessions } from "./useTerminalGridSessions";
import CanvasResizeHandles from "./CanvasResizeHandles";
import DropZoneOverlay from "./DropZoneOverlay";
import TerminalPane from "./TerminalPane";
import GridLayoutMenu from "./GridLayoutMenu";
import {
  ALL_HARNESSES,
  DISPLAY_NAMES,
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

  const {
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
  } = useTerminalGridSessions({
    workspace,
    requestedHarness,
    onHarnessRequestHandled,
    resetCanvasSize,
    setCanvasSize,
  });

  const { dragState, startPaneDrag } = useTerminalGridDrag();

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
        try {
          handle.releasePointerCapture(event.pointerId);
        } catch {
          /* ignore */
        }
        handle.removeEventListener("pointermove", onMove);
        handle.removeEventListener("pointerup", onUp);
        handle.removeEventListener("pointercancel", onUp);
      };
      handle.addEventListener("pointermove", onMove);
      handle.addEventListener("pointerup", onUp);
      handle.addEventListener("pointercancel", onUp);
    },
    [setLayout],
  );

  const openLeaves = collectLeaves(layout);
  const harnessCounts = openLeaves.reduce<Record<string, number>>((acc, leaf) => {
    acc[leaf.harness] = (acc[leaf.harness] || 0) + 1;
    return acc;
  }, {});
  const { leaves, splitters } = computeRects(layout, bounds, 6);

  const maxLeaf = maximizedId ? openLeaves.find((l) => l.id === maximizedId) : null;
  const maxLabel = maxLeaf ? DISPLAY_NAMES[maxLeaf.harness] || maxLeaf.harness : null;

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, gap: "0.75rem", height: "100%" }}>
      {/* Palette bar */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.75rem", padding: "0.5rem 0.85rem", borderRadius: "10px", background: "rgba(0, 0, 0, 0.3)", border: "1px solid var(--border-color)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
          <span style={{ fontSize: "0.8rem", fontWeight: 700, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
            + Add pane:
          </span>
          {ALL_HARNESSES.map((h) => {
            const count = harnessCounts[h] || 0;
            return (
              <button
                key={h}
                type="button"
                className="btn-secondary"
                style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }}
                disabled={busyId !== null}
                onClick={() => void launchHarness(h)}
              >
                + {DISPLAY_NAMES[h] || h}{count > 0 ? ` · ${count}` : ""}
              </button>
            );
          })}
          {maxLabel && (
            <span style={{ fontSize: "0.78rem", color: "#93c5fd", fontWeight: 600, background: "rgba(59, 130, 246, 0.18)", padding: "0.2rem 0.5rem", borderRadius: "6px", border: "1px solid rgba(59, 130, 246, 0.35)" }}>
              Maximized: {maxLabel} (Esc to restore)
            </span>
          )}
          {canvasSize && !maximizedId && (
            <span style={{ fontSize: "0.78rem", color: "var(--text-muted)", padding: "0.2rem 0.45rem", borderRadius: "4px", background: "rgba(255, 255, 255, 0.05)" }}>
              Grid: {canvasSize.width}×{canvasSize.height}px
            </span>
          )}
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          {maximizedId && (
            <button
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }}
              onClick={() => setMaximizedId(null)}
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
              Launch any of the 7 supported harness CLIs in an in-app interactive terminal pane. Drag title bars to rearrange or swap panes; drag splitters or borders to resize.
            </p>
            <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", justifyContent: "center", marginTop: "0.5rem" }}>
              {ALL_HARNESSES.map((h) => (
                <button
                  key={h}
                  type="button"
                  className="btn-primary"
                  style={{ marginTop: 0, padding: "0.45rem 0.85rem", fontSize: "0.85rem" }}
                  disabled={busyId !== null}
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
              const sid = terminals[node.id];
              const isBusy = busyId === node.id || busyId === node.harness;
              const isMaximized = maximizedId === node.id;
              const isHidden = maximizedId !== null && !isMaximized;

              return (
                <TerminalPane
                  key={node.id}
                  node={node}
                  rect={rect}
                  bounds={bounds}
                  sessionId={sid}
                  isBusy={isBusy}
                  isMaximized={isMaximized}
                  isHidden={isHidden}
                  hasMaximized={maximizedId !== null}
                  isDragging={dragState.isDragging && dragState.sourceLeafId === node.id}
                  onTitlePointerDown={(e) => {
                    if (!maximizedId) {
                      startPaneDrag(e, node.harness, node.id, () => leaves, containerRef.current, handleDrop);
                    }
                  }}
                  onConnect={() => void launchHarness(node.harness, node.id)}
                  onToggleMaximize={() => setMaximizedId(isMaximized ? null : node.id)}
                  onClose={() => void closePane(node.id)}
                  onExit={(detail) => setStatusMsg(`${DISPLAY_NAMES[node.harness] || node.harness} exit: ${detail}`)}
                  onError={(detail) => setError(`${DISPLAY_NAMES[node.harness] || node.harness} error: ${detail}`)}
                />
              );
            })}

            {/* Splitter Dividers - only when not maximized */}
            {!maximizedId && splitters.map((splitter) => (
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
        {!maximizedId && <CanvasResizeHandles onStartResize={startResize} />}
      </div>
    </div>
  );
}
