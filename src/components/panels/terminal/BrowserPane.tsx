import React, { useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { LeafNode, Rect } from "./layoutTree";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";
import { BROWSER_PANE_URL_ERROR, browserPaneUrlError, DEEPSEEK_GUI_HOME } from "./browserUrl";

export interface BrowserPaneProps {
  node: LeafNode;
  rect: Rect;
  bounds: Rect;
  isMaximized: boolean;
  isHidden: boolean;
  hasMaximized: boolean;
  /** False while the pane is covered or a grid drag needs the pointer. */
  active: boolean;
  onTitlePointerDown: (event: React.PointerEvent<HTMLDivElement>) => void;
  onUrlChange: (url: string) => void;
  onToggleMaximize: () => void;
  onClose: () => void;
}

interface PaneBounds {
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
}

function readBounds(slot: HTMLElement | null, active: boolean): PaneBounds {
  const rect = slot?.getBoundingClientRect();
  const width = rect?.width ?? 0;
  const height = rect?.height ?? 0;
  return {
    x: rect?.x ?? 0,
    y: rect?.y ?? 0,
    width,
    height,
    visible: active && width >= 8 && height >= 8,
  };
}

/**
 * DeepSeek's web GUI as a grid leaf. The page is a native child webview
 * positioned over the slot, so this component never mounts xterm.
 */
export default function BrowserPane({
  node,
  rect,
  bounds,
  isMaximized,
  isHidden,
  hasMaximized,
  active,
  onTitlePointerDown,
  onUrlChange,
  onToggleMaximize,
  onClose,
}: BrowserPaneProps) {
  const slotRef = useRef<HTMLDivElement | null>(null);
  const [draft, setDraft] = useState(node.url || DEEPSEEK_GUI_HOME);
  const [error, setError] = useState("");
  const initialUrl = useRef(node.url);

  useEffect(() => {
    let alive = true;
    const stored = (initialUrl.current ?? "").trim();
    if (!stored || browserPaneUrlError(stored)) {
      if (stored) setError(BROWSER_PANE_URL_ERROR);
      return () => {
        alive = false;
      };
    }
    void invoke("browser_pane_open", {
      leafId: node.id,
      url: stored,
      bounds: readBounds(slotRef.current, active),
    })
      .then(() => {
        if (!alive) return invoke("browser_pane_close", { leafId: node.id });
        return undefined;
      })
      .catch((cause: unknown) => {
        if (alive) setError(String(cause).replace(/^Error:\s*/, ""));
      });
    return () => {
      alive = false;
      void invoke("browser_pane_close", { leafId: node.id }).catch(() => undefined);
    };
    // The leaf id is the webview identity. URL edits go through navigate.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [node.id]);

  useEffect(() => {
    const slot = slotRef.current;
    if (!slot) return;
    let frame = 0;
    const send = () => {
      void invoke("browser_pane_place", {
        leafId: node.id,
        bounds: readBounds(slot, active),
      }).catch(() => undefined);
    };
    const schedule = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(send);
    };
    schedule();
    const resize = new ResizeObserver(schedule);
    resize.observe(slot);
    const intersection = typeof IntersectionObserver === "undefined" ? null : new IntersectionObserver(schedule);
    intersection?.observe(slot);
    window.addEventListener("scroll", schedule, true);
    window.addEventListener("resize", schedule);
    return () => {
      cancelAnimationFrame(frame);
      resize.disconnect();
      intersection?.disconnect();
      window.removeEventListener("scroll", schedule, true);
      window.removeEventListener("resize", schedule);
    };
  }, [node.id, active]);

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    const next = draft.trim();
    const problem = browserPaneUrlError(next);
    if (problem) {
      setError(problem);
      return;
    }
    void invoke("browser_pane_open", {
      leafId: node.id,
      url: next,
      bounds: readBounds(slotRef.current, active),
    })
      .then(() => {
        setError("");
        onUrlChange(next);
      })
      .catch((cause: unknown) => setError(String(cause).replace(/^Error:\s*/, "")));
  };

  return (
    <div
      data-testid={`browser-pane-${node.id}`}
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
          cursor: hasMaximized ? "default" : "grab",
          boxSizing: "border-box",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.45rem", minWidth: 0 }}>
          <span style={{ width: 7, height: 7, borderRadius: "50%", background: error ? NOC_COLORS.warn : NOC_COLORS.ok }} />
          <strong style={{ fontSize: "0.8rem", color: NOC_SURFACES.textPrimary }}>DeepSeek</strong>
          <code className="tabular-nums" style={{ fontSize: "0.68rem", color: NOC_COLORS.cyan, fontFamily: "var(--font-mono)" }}>
            {node.id.slice(0, 6)}
          </code>
        </div>
        <div style={{ display: "flex", gap: "0.3rem" }}>
          <button
            type="button"
            className="noc-focusable"
            aria-label={isMaximized ? "Restore DeepSeek GUI pane" : "Maximize DeepSeek GUI pane"}
            title={isMaximized ? "Restore grid (Esc)" : "Maximize pane"}
            style={{ background: "transparent", border: "none", color: NOC_SURFACES.textMuted, cursor: "pointer" }}
            onClick={onToggleMaximize}
          >
            {isMaximized ? "❐" : "⛶"}
          </button>
          <button
            type="button"
            className="noc-focusable"
            aria-label="Close DeepSeek GUI pane"
            title="Close pane"
            style={{ background: "transparent", border: "none", color: NOC_SURFACES.textMuted, cursor: "pointer" }}
            onClick={onClose}
          >
            ✕
          </button>
        </div>
      </div>
      <form onSubmit={submit} style={{ display: "flex", gap: "0.35rem", padding: "0.28rem 0.45rem", borderBottom: "1px solid rgba(148, 163, 184, 0.16)" }}>
        <input
          aria-label="DeepSeek GUI address"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onPointerDown={(event) => event.stopPropagation()}
          spellCheck={false}
          style={{
            flex: 1,
            minWidth: 0,
            background: "rgba(15, 23, 42, 0.8)",
            color: NOC_SURFACES.textPrimary,
            border: "1px solid rgba(148, 163, 184, 0.28)",
            borderRadius: "4px",
            fontSize: "0.75rem",
            padding: "0.15rem 0.4rem",
            fontFamily: "var(--font-mono)",
          }}
        />
        <button type="submit" className="btn-secondary noc-focusable" style={{ marginTop: 0, padding: "0.15rem 0.45rem", fontSize: "0.72rem" }}>
          Open
        </button>
      </form>
      {error && (
        <div style={{ padding: "0.25rem 0.5rem", color: "#fecaca", fontSize: "0.72rem" }}>{error}</div>
      )}
      <div ref={slotRef} style={{ flex: 1, minHeight: 0, background: "#050811" }}>
        <span style={{ color: "var(--text-muted)", fontSize: "0.75rem", padding: "0.4rem" }}>DeepSeek GUI</span>
      </div>
    </div>
  );
}
