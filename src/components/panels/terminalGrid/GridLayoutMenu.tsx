import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { CanvasSize } from "./gridConstants";
import type { LayoutNode } from "./layoutTree";

export interface TerminalGridCanvas {
  width: number;
  height: number;
}

export interface TerminalGridLayoutFile {
  version: number;
  name: string;
  savedAt: string;
  canvas: TerminalGridCanvas | null;
  layout: LayoutNode;
}

export interface TerminalGridLayoutSummary {
  name: string;
  savedAt: string;
  version: number;
  canvas: TerminalGridCanvas | null;
}

export interface GridLayoutMenuProps {
  currentLayout: LayoutNode | null;
  currentCanvas: CanvasSize | null;
  onLoadLayout: (layout: LayoutNode, canvas: CanvasSize | null) => void;
  onError?: (error: string) => void;
  onStatus?: (status: string) => void;
}

export default function GridLayoutMenu({
  currentLayout,
  currentCanvas,
  onLoadLayout,
  onError,
  onStatus,
}: GridLayoutMenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [saveName, setSaveName] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [savedLayouts, setSavedLayouts] = useState<TerminalGridLayoutSummary[]>([]);
  const [isLoadingList, setIsLoadingList] = useState(false);
  const [activeActionName, setActiveActionName] = useState<string | null>(null);

  const menuRef = useRef<HTMLDivElement>(null);

  const fetchLayouts = useCallback(async () => {
    setIsLoadingList(true);
    try {
      const list = await invoke<TerminalGridLayoutSummary[]>("hub_list_terminal_grid_layouts");
      setSavedLayouts(list);
    } catch (e) {
      onError?.(`Failed to list layouts: ${e}`);
    } finally {
      setIsLoadingList(false);
    }
  }, [onError]);

  useEffect(() => {
    if (isOpen) {
      void fetchLayouts();
    }
  }, [isOpen, fetchLayouts]);

  useEffect(() => {
    if (!isOpen) return;
    const onMouseDown = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", onMouseDown);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onMouseDown);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = saveName.trim();
    if (!trimmed) return;
    if (!currentLayout) {
      onError?.("Cannot save empty layout. Open at least one harness pane first.");
      return;
    }

    setIsSaving(true);
    try {
      const saved = await invoke<TerminalGridLayoutFile>("hub_save_terminal_grid_layout", {
        name: trimmed,
        layout: currentLayout,
        canvas: currentCanvas ? { width: currentCanvas.width, height: currentCanvas.height } : null,
      });
      setSaveName("");
      onStatus?.(`Layout "${saved.name}" saved successfully.`);
      await fetchLayouts();
    } catch (e) {
      onError?.(String(e).replace(/^Error:\s*/, ""));
    } finally {
      setIsSaving(false);
    }
  };

  const handleLoad = async (name: string) => {
    setActiveActionName(name);
    try {
      const data = await invoke<TerminalGridLayoutFile>("hub_load_terminal_grid_layout", { name });
      const canvas: CanvasSize | null = data.canvas
        ? { width: Math.round(data.canvas.width), height: Math.round(data.canvas.height) }
        : null;
      onLoadLayout(data.layout, canvas);
      onStatus?.(`Layout "${name}" loaded.`);
      setIsOpen(false);
    } catch (e) {
      onError?.(`Failed to load layout "${name}": ${e}`);
    } finally {
      setActiveActionName(null);
    }
  };

  const handleDelete = async (name: string) => {
    if (!confirm(`Delete layout "${name}"?`)) return;
    setActiveActionName(name);
    try {
      await invoke("hub_delete_terminal_grid_layout", { name });
      onStatus?.(`Layout "${name}" deleted.`);
      await fetchLayouts();
    } catch (e) {
      onError?.(`Failed to delete layout "${name}": ${e}`);
    } finally {
      setActiveActionName(null);
    }
  };

  return (
    <div ref={menuRef} style={{ position: "relative", display: "inline-block" }}>
      <button
        type="button"
        className="btn-secondary"
        style={{ marginTop: 0, padding: "0.3rem 0.65rem", fontSize: "0.8rem", display: "inline-flex", alignItems: "center", gap: "0.3rem" }}
        onClick={() => setIsOpen((prev) => !prev)}
        title="Save or load named terminal grid layouts"
        aria-expanded={isOpen}
      >
        <span>💾 Layouts</span>
        <span style={{ fontSize: "0.65rem", opacity: 0.8 }}>▼</span>
      </button>

      {isOpen && (
        <div
          className="glass-card"
          style={{
            position: "absolute",
            top: "calc(100% + 0.35rem)",
            right: 0,
            width: "320px",
            background: "rgba(15, 23, 42, 0.96)",
            border: "1px solid rgba(148, 163, 184, 0.3)",
            borderRadius: "8px",
            boxShadow: "0 10px 25px -5px rgba(0, 0, 0, 0.6)",
            padding: "0.85rem",
            zIndex: 100,
            display: "flex",
            flexDirection: "column",
            gap: "0.75rem",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid rgba(148, 163, 184, 0.2)", paddingBottom: "0.4rem" }}>
            <strong style={{ fontSize: "0.85rem", color: "#f8fafc" }}>Grid Layouts</strong>
            <button
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.1rem 0.35rem", fontSize: "0.75rem" }}
              onClick={() => setIsOpen(false)}
              title="Close menu"
            >
              ✕
            </button>
          </div>

          {/* Save section */}
          <form onSubmit={handleSave} style={{ display: "flex", flexDirection: "column", gap: "0.35rem" }}>
            <label style={{ fontSize: "0.75rem", color: "var(--text-muted)", fontWeight: 600 }}>
              Save current layout:
            </label>
            <div style={{ display: "flex", gap: "0.35rem" }}>
              <input
                type="text"
                value={saveName}
                onChange={(e) => setSaveName(e.target.value)}
                placeholder="Layout name (e.g. trio-dev)"
                style={{
                  flex: 1,
                  padding: "0.3rem 0.5rem",
                  fontSize: "0.8rem",
                  borderRadius: "4px",
                  border: "1px solid rgba(148, 163, 184, 0.3)",
                  background: "rgba(0, 0, 0, 0.3)",
                  color: "#f8fafc",
                }}
                disabled={isSaving}
              />
              <button
                type="submit"
                className="btn-primary"
                style={{ marginTop: 0, padding: "0.3rem 0.6rem", fontSize: "0.8rem" }}
                disabled={isSaving || !saveName.trim() || !currentLayout}
                title={!currentLayout ? "Cannot save empty layout" : "Save layout"}
              >
                {isSaving ? "Saving…" : "Save"}
              </button>
            </div>
          </form>

          {/* List section */}
          <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem", borderTop: "1px solid rgba(148, 163, 184, 0.2)", paddingTop: "0.5rem" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", fontWeight: 600 }}>
                Saved layouts ({savedLayouts.length}):
              </span>
              <button
                type="button"
                className="btn-secondary"
                style={{ marginTop: 0, padding: "0.1rem 0.3rem", fontSize: "0.7rem" }}
                onClick={() => void fetchLayouts()}
                disabled={isLoadingList}
                title="Refresh list"
              >
                ↻
              </button>
            </div>

            <div style={{ maxHeight: "180px", overflowY: "auto", display: "flex", flexDirection: "column", gap: "0.4rem" }}>
              {isLoadingList && (
                <div style={{ fontSize: "0.78rem", color: "var(--text-muted)", fontStyle: "italic", padding: "0.25rem 0" }}>
                  Loading layouts…
                </div>
              )}
              {!isLoadingList && savedLayouts.length === 0 && (
                <div style={{ fontSize: "0.78rem", color: "var(--text-muted)", fontStyle: "italic", padding: "0.25rem 0" }}>
                  No saved layouts yet.
                </div>
              )}
              {!isLoadingList &&
                savedLayouts.map((item) => (
                  <div
                    key={item.name}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      padding: "0.35rem 0.5rem",
                      borderRadius: "6px",
                      background: "rgba(255, 255, 255, 0.04)",
                      border: "1px solid rgba(255, 255, 255, 0.08)",
                      gap: "0.5rem",
                    }}
                  >
                    <div style={{ display: "flex", flexDirection: "column", minWidth: 0, flex: 1 }}>
                      <strong style={{ fontSize: "0.8rem", color: "#f8fafc", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {item.name}
                      </strong>
                      <span style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>
                        {item.canvas ? `${item.canvas.width}×${item.canvas.height}px` : "Window fit"}
                      </span>
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: "0.25rem" }}>
                      <button
                        type="button"
                        className="btn-secondary"
                        style={{ marginTop: 0, padding: "0.15rem 0.4rem", fontSize: "0.72rem" }}
                        disabled={activeActionName === item.name}
                        onClick={() => void handleLoad(item.name)}
                        title="Load this layout (never auto-spawns)"
                      >
                        Load
                      </button>
                      <button
                        type="button"
                        className="btn-secondary"
                        style={{ marginTop: 0, padding: "0.15rem 0.35rem", fontSize: "0.72rem", color: "#f87171" }}
                        disabled={activeActionName === item.name}
                        onClick={() => void handleDelete(item.name)}
                        title="Delete this layout"
                      >
                        ✕
                      </button>
                    </div>
                  </div>
                ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
