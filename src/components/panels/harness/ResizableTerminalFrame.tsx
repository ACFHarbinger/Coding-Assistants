import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

const MIN_W = 480;
const MIN_H = 280;
const DEFAULT_H = 480;

function storageKey(id: string): string {
  return `ca.liveTerminal.size.${id}`;
}

function loadSize(id: string): { width: number | null; height: number } {
  try {
    const raw = localStorage.getItem(storageKey(id));
    if (!raw) return { width: null, height: DEFAULT_H };
    const parsed = JSON.parse(raw) as { width?: number; height?: number };
    return {
      width: typeof parsed.width === "number" && parsed.width >= MIN_W ? parsed.width : null,
      height: typeof parsed.height === "number" && parsed.height >= MIN_H ? parsed.height : DEFAULT_H,
    };
  } catch {
    return { width: null, height: DEFAULT_H };
  }
}

/**
 * Layout-only frame around an in-app PTY (#167 width/resize).
 * Does not import or change `EmbeddedTerminal` internals — Gemini owns
 * xterm scroll/focus. This wrapper is full-width by default and drag-resizable.
 */
export default function ResizableTerminalFrame({
  persistId,
  children,
}: {
  persistId: string;
  children: ReactNode;
}) {
  const boxRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState(() => loadSize(persistId));

  useEffect(() => {
    setSize(loadSize(persistId));
  }, [persistId]);

  useEffect(() => {
    try {
      localStorage.setItem(storageKey(persistId), JSON.stringify(size));
    } catch {
      /* ignore quota / private mode */
    }
  }, [persistId, size]);

  const onDrag = useCallback((event: React.PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0) return;
    const box = boxRef.current;
    if (!box) return;
    const startX = event.clientX;
    const startY = event.clientY;
    const startW = box.clientWidth;
    const startH = box.clientHeight;
    const handle = event.currentTarget;
    handle.setPointerCapture(event.pointerId);

    const move = (next: PointerEvent) => {
      const parentW = box.parentElement?.clientWidth ?? startW;
      const width = Math.max(MIN_W, Math.min(parentW, startW + (next.clientX - startX)));
      const height = Math.max(MIN_H, Math.min(window.innerHeight - 80, startH + (next.clientY - startY)));
      setSize({ width, height });
    };
    const up = () => {
      handle.releasePointerCapture(event.pointerId);
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      handle.removeEventListener("pointercancel", up);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
    handle.addEventListener("pointercancel", up);
  }, []);

  return (
    <div
      ref={boxRef}
      style={{
        position: "relative",
        width: size.width == null ? "100%" : `${size.width}px`,
        maxWidth: "100%",
        minWidth: `${MIN_W}px`,
        height: `${size.height}px`,
        minHeight: `${MIN_H}px`,
        borderRadius: "10px",
        overflow: "hidden",
        border: "1px solid rgba(148, 163, 184, 0.28)",
        background: "#0b1220",
      }}
    >
      <div style={{ position: "absolute", inset: 0 }}>{children}</div>
      <button
        type="button"
        aria-label="Resize terminal"
        title="Drag to resize"
        onPointerDown={onDrag}
        style={{
          position: "absolute",
          right: 2,
          bottom: 2,
          width: 16,
          height: 16,
          padding: 0,
          border: "none",
          cursor: "nwse-resize",
          background: "transparent",
          zIndex: 2,
        }}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M14 6 L6 14 M14 10 L10 14" stroke="rgba(226,232,240,0.7)" strokeWidth="1.5" fill="none" />
        </svg>
      </button>
    </div>
  );
}
