import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";
import EmbeddedTerminal from "../harness/EmbeddedTerminal";
import TerminalPaneErrorBoundary from "../harness/TerminalPaneErrorBoundary";
import type { EmbeddedRelaunchOutcome, HarnessSessionRegistration } from "../harness/types";
import {
  computeRects,
  insertLeaf,
  removeLeaf,
  resizeSplit,
  collectLeaves,
  serializeLayout,
  deserializeLayout,
  type LayoutNode,
  type Rect,
  type SplitterInfo,
} from "./layoutTree";

const ALL_HARNESSES = ["grok", "chat", "claude", "gemini", "muse", "cursor"] as const;

const DISPLAY_NAMES: Record<string, string> = {
  grok: "Grok",
  chat: "Codex",
  claude: "Claude",
  gemini: "Gemini",
  muse: "Muse",
  cursor: "Cursor",
};

function storageKey(workspace: string): string {
  return `ca.terminalGrid.layout.${workspace || "default"}`;
}

export interface HarnessTerminalGridProps {
  workspace: string;
  onOpenSetup?: () => void;
}

export default function HarnessTerminalGrid({
  workspace,
  onOpenSetup,
}: HarnessTerminalGridProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [bounds, setBounds] = useState<Rect>({ x: 0, y: 0, width: 900, height: 600 });
  const [layout, setLayout] = useState<LayoutNode | null>(() => {
    try {
      return deserializeLayout(localStorage.getItem(storageKey(workspace)));
    } catch {
      return null;
    }
  });

  const [terminals, setTerminals] = useState<Record<string, string>>({});
  const [busyHarness, setBusyHarness] = useState<string | null>(null);
  const [error, setError] = useState<string>("");
  const [statusMsg, setStatusMsg] = useState<string>("");

  useEffect(() => {
    try {
      setLayout(deserializeLayout(localStorage.getItem(storageKey(workspace))));
    } catch {
      setLayout(null);
    }
  }, [workspace]);

  useEffect(() => {
    try {
      if (layout) localStorage.setItem(storageKey(workspace), serializeLayout(layout));
      else localStorage.removeItem(storageKey(workspace));
    } catch {
      /* ignore quota */
    }
  }, [workspace, layout]);

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
          // Ignore listing failure
        }
        const outcome = await invoke<EmbeddedRelaunchOutcome>("hub_relaunch_harness_embedded", {
          harness,
          workspace,
          existingPid,
        });
        const sid = outcome.sessionId ?? (outcome as { session_id?: string }).session_id;
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
      try {
        await invoke("pty_kill", { sessionId: sid });
      } catch {
        // already exited
      }
      setTerminals((prev) => {
        const next = { ...prev };
        delete next[harness];
        return next;
      });
    }
    setLayout((prev) => removeLeaf(prev, harness));
  }, [terminals]);

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
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          {openLeaves.length > 0 && (
            <button
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem" }}
              onClick={() => setLayout(null)}
              title="Close all panes and reset layout"
            >
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
          flex: 1,
          minHeight: "480px",
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
              Launch any of the 6 supported harness CLIs in an in-app interactive terminal pane. Splitters can be dragged to push/pull pane sizes.
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

              return (
                <div
                  key={node.harness}
                  style={{
                    position: "absolute",
                    left: `${rect.x}px`,
                    top: `${rect.y}px`,
                    width: `${rect.width}px`,
                    height: `${rect.height}px`,
                    display: "flex",
                    flexDirection: "column",
                    borderRadius: "6px",
                    overflow: "hidden",
                    border: "1px solid rgba(148, 163, 184, 0.25)",
                    background: "#080d1a",
                    zIndex: 1,
                  }}
                >
                  {/* Pane Title Bar */}
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "0.3rem 0.6rem", background: "rgba(0, 0, 0, 0.5)", borderBottom: "1px solid rgba(148, 163, 184, 0.2)", userSelect: "none" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "0.45rem" }}>
                      <span style={{ width: "8px", height: "8px", borderRadius: "50%", background: sid ? "#22c55e" : "#64748b" }} />
                      <strong style={{ fontSize: "0.82rem", color: "var(--text-main)" }}>{DISPLAY_NAMES[node.harness] || node.harness}</strong>
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
                      {!sid && (
                        <button
                          type="button"
                          className="btn-secondary"
                          style={{ padding: "0.15rem 0.45rem", fontSize: "0.72rem", marginTop: 0 }}
                          disabled={isBusy}
                          onClick={() => void launchHarness(node.harness)}
                        >
                          {isBusy ? "…" : "Connect"}
                        </button>
                      )}
                      <button
                        type="button"
                        aria-label={`Close ${node.harness} pane`}
                        title="Close pane"
                        style={{ background: "transparent", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: "0.1rem 0.35rem", borderRadius: "4px", fontSize: "0.85rem", lineHeight: 1 }}
                        onClick={() => void closePane(node.harness)}
                      >
                        ✕
                      </button>
                    </div>
                  </div>

                  {/* Pane Content */}
                  <div style={{ flex: 1, position: "relative", minHeight: 0, background: "#050811" }}>
                    <TerminalPaneErrorBoundary>
                      {sid ? (
                        <EmbeddedTerminal
                          sessionId={sid}
                          onExit={(detail) => setStatusMsg(`${DISPLAY_NAMES[node.harness] || node.harness} exit: ${detail}`)}
                          onError={(detail) => setError(`${DISPLAY_NAMES[node.harness] || node.harness} error: ${detail}`)}
                        />
                      ) : (
                        <div style={{ display: "flex", height: "100%", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "0.5rem", color: "var(--text-muted)", fontSize: "0.82rem" }}>
                          <span>{isBusy ? `Starting ${DISPLAY_NAMES[node.harness] || node.harness}…` : "Session idle"}</span>
                          {!isBusy && (
                            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.78rem" }} onClick={() => void launchHarness(node.harness)}>
                              Connect CLI
                            </button>
                          )}
                        </div>
                      )}
                    </TerminalPaneErrorBoundary>
                  </div>
                </div>
              );
            })}

            {/* Splitter Dividers */}
            {splitters.map((splitter) => (
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
          </>
        )}
      </div>
    </div>
  );
}
