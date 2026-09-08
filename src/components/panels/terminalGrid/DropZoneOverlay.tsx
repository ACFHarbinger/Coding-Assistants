import type { DropTarget } from "./useTerminalGridDrag";

interface DropZoneOverlayProps {
  target: DropTarget;
  displayName: string;
}

export default function DropZoneOverlay({ target, displayName }: DropZoneOverlayProps) {
  const { rect, zone } = target;

  let x = rect.x;
  let y = rect.y;
  let width = rect.width;
  let height = rect.height;
  let label = `Swap with ${displayName}`;
  let icon = "⇄";

  if (zone === "left") {
    width = Math.round(rect.width / 2);
    label = `Dock left of ${displayName}`;
    icon = "←";
  } else if (zone === "right") {
    const half = Math.round(rect.width / 2);
    x = rect.x + half;
    width = rect.width - half;
    label = `Dock right of ${displayName}`;
    icon = "→";
  } else if (zone === "top") {
    height = Math.round(rect.height / 2);
    label = `Dock above ${displayName}`;
    icon = "↑";
  } else if (zone === "bottom") {
    const half = Math.round(rect.height / 2);
    y = rect.y + half;
    height = rect.height - half;
    label = `Dock below ${displayName}`;
    icon = "↓";
  }

  return (
    <div
      data-testid="drop-zone-overlay"
      style={{
        position: "absolute",
        left: `${x}px`,
        top: `${y}px`,
        width: `${width}px`,
        height: `${height}px`,
        background: "rgba(59, 130, 246, 0.22)",
        border: "2px dashed #60a5fa",
        borderRadius: "6px",
        zIndex: 40,
        pointerEvents: "none",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        transition: "all 0.08s ease-out",
        boxSizing: "border-box",
      }}
    >
      <div
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: "0.4rem",
          background: "rgba(15, 23, 42, 0.9)",
          color: "#93c5fd",
          border: "1px solid rgba(96, 165, 250, 0.5)",
          padding: "0.35rem 0.75rem",
          borderRadius: "6px",
          fontSize: "0.8rem",
          fontWeight: 600,
          boxShadow: "0 4px 14px rgba(0, 0, 0, 0.5)",
          backdropFilter: "blur(4px)",
        }}
      >
        <span style={{ fontSize: "1rem", lineHeight: 1 }}>{icon}</span>
        <span>{label}</span>
      </div>
    </div>
  );
}
