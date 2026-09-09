import React from "react";
import EmbeddedTerminal from "../harness/EmbeddedTerminal";
import TerminalPaneErrorBoundary from "../harness/TerminalPaneErrorBoundary";
import type { LeafNode, Rect } from "./layoutTree";
import { DISPLAY_NAMES } from "./gridConstants";

export interface TerminalPaneProps {
  node: LeafNode;
  rect: Rect;
  bounds: Rect;
  sessionId?: string;
  isBusy: boolean;
  isMaximized: boolean;
  isHidden: boolean;
  hasMaximized: boolean;
  isDragging: boolean;
  onTitlePointerDown: (e: React.PointerEvent<HTMLDivElement>) => void;
  onConnect: () => void;
  onToggleMaximize: () => void;
  onClose: () => void;
  onExit: (detail: string) => void;
  onError: (detail: string) => void;
}

export default function TerminalPane({
  node,
  rect,
  bounds,
  sessionId,
  isBusy,
  isMaximized,
  isHidden,
  hasMaximized,
  isDragging,
  onTitlePointerDown,
  onConnect,
  onToggleMaximize,
  onClose,
  onExit,
  onError,
}: TerminalPaneProps) {
  const title = DISPLAY_NAMES[node.harness] || node.harness;

  return (
    <div
      style={{
        position: "absolute",
        left: isMaximized ? 0 : `${rect.x}px`,
        top: isMaximized ? 0 : `${rect.y}px`,
        width: isMaximized ? `${bounds.width}px` : `${rect.width}px`,
        height: isMaximized ? `${bounds.height}px` : `${rect.height}px`,
        display: "flex",
        flexDirection: "column",
        borderRadius: "6px",
        overflow: "hidden",
        border: isMaximized ? "1px solid rgba(96, 165, 250, 0.45)" : "1px solid rgba(148, 163, 184, 0.25)",
        background: "#080d1a",
        zIndex: isMaximized ? 20 : 1,
        visibility: isHidden ? "hidden" : "visible",
        pointerEvents: isHidden ? "none" : "auto",
      }}
    >
      {/* Pane Title Bar */}
      <div
        onPointerDown={onTitlePointerDown}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0.3rem 0.6rem",
          background: "rgba(0, 0, 0, 0.5)",
          borderBottom: "1px solid rgba(148, 163, 184, 0.2)",
          userSelect: "none",
          cursor: hasMaximized ? "default" : (isDragging ? "grabbing" : "grab"),
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.45rem" }}>
          {!hasMaximized && (
            <span style={{ color: "var(--text-muted)", fontSize: "0.75rem", letterSpacing: "-1px" }} title="Drag to rearrange pane">
              ⋮⋮
            </span>
          )}
          <span style={{ width: "8px", height: "8px", borderRadius: "50%", background: sessionId ? "#22c55e" : "#64748b" }} />
          <strong style={{ fontSize: "0.82rem", color: "var(--text-main)" }}>{title}</strong>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
          {!sessionId && (
            <button
              type="button"
              className="btn-secondary"
              style={{ padding: "0.15rem 0.45rem", fontSize: "0.72rem", marginTop: 0 }}
              disabled={isBusy}
              onClick={onConnect}
            >
              {isBusy ? "…" : "Connect"}
            </button>
          )}
          <button
            type="button"
            aria-label={isMaximized ? `Restore ${node.harness} pane` : `Maximize ${node.harness} pane`}
            title={isMaximized ? "Restore grid (Esc)" : "Maximize pane"}
            style={{ background: "transparent", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: "0.1rem 0.35rem", borderRadius: "4px", fontSize: "0.85rem", lineHeight: 1 }}
            onClick={onToggleMaximize}
          >
            {isMaximized ? "❐" : "⛶"}
          </button>
          <button
            type="button"
            aria-label={`Close ${node.harness} pane`}
            title="Close pane"
            style={{ background: "transparent", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: "0.1rem 0.35rem", borderRadius: "4px", fontSize: "0.85rem", lineHeight: 1 }}
            onClick={onClose}
          >
            ✕
          </button>
        </div>
      </div>

      {/* Pane Body */}
      <div style={{ flex: 1, position: "relative", minHeight: 0, background: "#050811" }}>
        <TerminalPaneErrorBoundary>
          {sessionId ? (
            <EmbeddedTerminal
              sessionId={sessionId}
              onExit={onExit}
              onError={onError}
            />
          ) : (
            <div style={{ display: "flex", height: "100%", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "0.5rem", color: "var(--text-muted)", fontSize: "0.82rem" }}>
              <span>{isBusy ? `Starting ${title}…` : "Session idle"}</span>
              {!isBusy && (
                <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.78rem" }} onClick={onConnect}>
                  Connect CLI
                </button>
              )}
            </div>
          )}
        </TerminalPaneErrorBoundary>
      </div>
    </div>
  );
}
