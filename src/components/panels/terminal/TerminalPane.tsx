import React from "react";
import EmbeddedTerminal from "../harness/EmbeddedTerminal";
import TerminalPaneErrorBoundary from "../harness/TerminalPaneErrorBoundary";
import type { LeafNode, Rect } from "./layoutTree";
import { DISPLAY_NAMES } from "./gridConstants";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

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
        borderRadius: "8px",
        overflow: "hidden",
        border: isMaximized ? "1px solid rgba(56, 189, 248, 0.45)" : "1px solid rgba(148, 163, 184, 0.22)",
        background: "#080d1a",
        zIndex: isMaximized ? 20 : 1,
        visibility: isHidden ? "hidden" : "visible",
        pointerEvents: isHidden ? "none" : "auto",
      }}
    >
      {/* 28px NOC Telemetry Header Bar */}
      <div
        onPointerDown={onTitlePointerDown}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          height: "28px",
          minHeight: "28px",
          padding: "0 0.55rem",
          background: "rgba(2, 6, 23, 0.9)",
          borderBottom: "1px solid rgba(148, 163, 184, 0.18)",
          userSelect: "none",
          cursor: hasMaximized ? "default" : isDragging ? "grabbing" : "grab",
          boxSizing: "border-box",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.45rem", minWidth: 0, overflow: "hidden" }}>
          {!hasMaximized && (
            <span style={{ color: NOC_SURFACES.textMuted, fontSize: "0.72rem", letterSpacing: "-1px" }} title="Drag to rearrange pane">
              ⋮⋮
            </span>
          )}
          <span
            style={{
              width: "7px",
              height: "7px",
              borderRadius: "50%",
              background: sessionId ? NOC_COLORS.ok : NOC_COLORS.idle,
              display: "inline-block",
              flexShrink: 0,
            }}
          />
          <strong style={{ fontSize: "0.8rem", color: NOC_SURFACES.textPrimary, whiteSpace: "nowrap" }}>{title}</strong>
          <code
            className="tabular-nums"
            style={{
              fontSize: "0.68rem",
              color: NOC_COLORS.cyan,
              fontFamily: "var(--font-mono)",
              background: "rgba(56, 189, 248, 0.1)",
              padding: "0.05rem 0.3rem",
              borderRadius: "3px",
              flexShrink: 0,
            }}
          >
            {node.id.slice(0, 6)}
          </code>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.3rem", flexShrink: 0 }}>
          {!sessionId && (
            <button
              type="button"
              className="btn-secondary noc-focusable"
              style={{ padding: "0.1rem 0.45rem", fontSize: "0.7rem", marginTop: 0, lineHeight: 1.2 }}
              disabled={isBusy}
              onClick={onConnect}
            >
              {isBusy ? "…" : "Connect"}
            </button>
          )}
          <button
            type="button"
            className="noc-focusable"
            aria-label={isMaximized ? `Restore ${node.harness} pane` : `Maximize ${node.harness} pane`}
            title={isMaximized ? "Restore grid (Esc)" : "Maximize pane"}
            style={{
              background: "transparent",
              border: "none",
              color: NOC_SURFACES.textMuted,
              cursor: "pointer",
              padding: "0.1rem 0.3rem",
              borderRadius: "4px",
              fontSize: "0.82rem",
              lineHeight: 1,
            }}
            onClick={onToggleMaximize}
          >
            {isMaximized ? "❐" : "⛶"}
          </button>
          <button
            type="button"
            className="noc-focusable"
            aria-label={`Close ${node.harness} pane`}
            title="Close pane"
            style={{
              background: "transparent",
              border: "none",
              color: NOC_SURFACES.textMuted,
              cursor: "pointer",
              padding: "0.1rem 0.3rem",
              borderRadius: "4px",
              fontSize: "0.82rem",
              lineHeight: 1,
            }}
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
