import type { PointerEvent } from "react";
import type { ResizeDirection } from "./useGridCanvasResize";

interface CanvasResizeHandlesProps {
  onStartResize: (e: PointerEvent<HTMLDivElement>, direction: ResizeDirection) => void;
}

export default function CanvasResizeHandles({ onStartResize }: CanvasResizeHandlesProps) {
  return (
    <>
      {/* Right border resize handle */}
      <div
        data-testid="canvas-resize-right"
        aria-label="Resize canvas width"
        title="Drag to resize canvas width"
        onPointerDown={(e) => onStartResize(e, "right")}
        style={{
          position: "absolute",
          right: 0,
          top: 0,
          bottom: "14px",
          width: "8px",
          cursor: "ew-resize",
          zIndex: 25,
          userSelect: "none",
          touchAction: "none",
        }}
      />

      {/* Bottom border resize handle */}
      <div
        data-testid="canvas-resize-bottom"
        aria-label="Resize canvas height"
        title="Drag to resize canvas height"
        onPointerDown={(e) => onStartResize(e, "bottom")}
        style={{
          position: "absolute",
          left: 0,
          right: "14px",
          bottom: 0,
          height: "8px",
          cursor: "ns-resize",
          zIndex: 25,
          userSelect: "none",
          touchAction: "none",
        }}
      />

      {/* Bottom-right corner resize handle */}
      <div
        data-testid="canvas-resize-corner"
        aria-label="Resize canvas width and height"
        title="Drag to resize canvas size"
        onPointerDown={(e) => onStartResize(e, "corner")}
        style={{
          position: "absolute",
          right: 0,
          bottom: 0,
          width: "14px",
          height: "14px",
          cursor: "nwse-resize",
          zIndex: 30,
          userSelect: "none",
          touchAction: "none",
          display: "flex",
          alignItems: "flex-end",
          justifyContent: "flex-end",
          padding: "2px",
        }}
      >
        <svg
          width="10"
          height="10"
          viewBox="0 0 10 10"
          fill="none"
          stroke="rgba(148, 163, 184, 0.6)"
          strokeWidth="1.5"
          strokeLinecap="round"
          style={{ pointerEvents: "none" }}
        >
          <line x1="8" y1="2" x2="2" y2="8" />
          <line x1="8" y1="5" x2="5" y2="8" />
          <line x1="8" y1="8" x2="8" y2="8" />
        </svg>
      </div>
    </>
  );
}
