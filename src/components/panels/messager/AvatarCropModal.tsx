import { useEffect, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { createPortal } from "react-dom";
import {
  CROP_OUTPUT_SIZE,
  encodeCroppedPng,
  initialCrop,
  moveCrop,
  resizeCropFromCorner,
  type CropCorner,
  type CropRect,
} from "./avatarCrop";

const HANDLE: CropCorner[] = ["nw", "ne", "sw", "se"];

const HANDLE_CURSOR: Record<CropCorner, string> = {
  nw: "nwse-resize",
  se: "nwse-resize",
  ne: "nesw-resize",
  sw: "nesw-resize",
};

type Drag =
  | { kind: "move"; startX: number; startY: number; crop: CropRect }
  | { kind: CropCorner; crop: CropRect };

function imagePoint(event: PointerEvent, img: HTMLImageElement, scale: number) {
  const rect = img.getBoundingClientRect();
  return {
    x: (event.clientX - rect.left) / scale,
    y: (event.clientY - rect.top) / scale,
  };
}

export function AvatarCropModal(props: {
  imageSrc: string;
  onCancel: () => void;
  onSave: (pngBase64: string) => Promise<void>;
}) {
  const { imageSrc, onCancel, onSave } = props;
  const imgRef = useRef<HTMLImageElement>(null);
  const dragRef = useRef<Drag | null>(null);
  const [natural, setNatural] = useState({ w: 0, h: 0 });
  const [crop, setCrop] = useState<CropRect | null>(null);
  const [viewport, setViewport] = useState(() => ({ w: window.innerWidth, h: window.innerHeight }));
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const onResize = () => setViewport({ w: window.innerWidth, h: window.innerHeight });
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (saving) return;
      if (event.key === "Escape") {
        event.preventDefault();
        onCancel();
        return;
      }
      if (!crop || !natural.w) return;
      const step = event.shiftKey ? 10 : 1;
      let dx = 0;
      let dy = 0;
      if (event.key === "ArrowLeft") dx = -step;
      else if (event.key === "ArrowRight") dx = step;
      else if (event.key === "ArrowUp") dy = -step;
      else if (event.key === "ArrowDown") dy = step;
      else return;
      event.preventDefault();
      setCrop(moveCrop(crop, dx, dy, natural.w, natural.h));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [crop, natural, onCancel, saving]);

  const maxW = Math.min(720, viewport.w * 0.86);
  const maxH = Math.min(560, viewport.h * 0.62);
  const scale = natural.w > 0 ? Math.min(maxW / natural.w, maxH / natural.h) : 1;
  const displayW = natural.w * scale;
  const displayH = natural.h * scale;

  const beginDrag = (event: PointerEvent<HTMLElement>, kind: Drag["kind"]) => {
    const img = imgRef.current;
    if (!img || !crop) return;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const point = imagePoint(event, img, scale);
    dragRef.current =
      kind === "move"
        ? { kind: "move", startX: point.x, startY: point.y, crop }
        : { kind, crop };
  };

  const onPointerMove = (event: PointerEvent<HTMLElement>) => {
    const drag = dragRef.current;
    const img = imgRef.current;
    if (!drag || !img) return;
    const point = imagePoint(event, img, scale);
    if (drag.kind === "move") {
      setCrop(moveCrop(drag.crop, point.x - drag.startX, point.y - drag.startY, natural.w, natural.h));
      return;
    }
    setCrop(resizeCropFromCorner(drag.crop, drag.kind, point.x, point.y, natural.w, natural.h));
  };

  const endDrag = () => {
    dragRef.current = null;
  };

  const handleSave = async () => {
    const img = imgRef.current;
    if (!img || !crop || saving) return;
    setSaving(true);
    try {
      const pngBase64 = await encodeCroppedPng(img, crop);
      await onSave(pngBase64);
    } catch (error) {
      alert(`Avatar crop failed: ${error}`);
    } finally {
      setSaving(false);
    }
  };

  const overlay: CSSProperties = {
    position: "fixed",
    inset: 0,
    background: "rgba(2,6,23,0.88)",
    display: "flex",
    justifyContent: "center",
    alignItems: "center",
    zIndex: 3000,
    padding: "1.5rem",
  };

  const panel: CSSProperties = {
    background: "var(--bg-card)",
    border: "1px solid var(--border-color)",
    borderRadius: 16,
    padding: "1.25rem 1.25rem 1rem",
    maxWidth: "min(820px, 96vw)",
    boxShadow: "0 25px 50px -12px rgba(0,0,0,0.45)",
    display: "flex",
    flexDirection: "column",
    gap: "0.9rem",
  };

  const stage: CSSProperties = {
    position: "relative",
    width: displayW || undefined,
    height: displayH || undefined,
    maxWidth: "100%",
    overflow: "hidden",
    background: "#000",
    userSelect: "none",
    touchAction: "none",
    alignSelf: "center",
  };

  const modal = (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Crop avatar"
      style={overlay}
      onClick={(event) => {
        event.stopPropagation();
        if (!saving) onCancel();
      }}
    >
      <div className="fade-in" style={panel} onClick={(event) => event.stopPropagation()}>
        <div>
          <h2 style={{ margin: 0, fontSize: "1.15rem", color: "var(--text-main)" }}>Crop avatar</h2>
          <p style={{ margin: "0.35rem 0 0", fontSize: "0.8rem", color: "var(--text-muted)" }}>
            Drag the square to reposition, use the corners to resize. Saved at {CROP_OUTPUT_SIZE}×
            {CROP_OUTPUT_SIZE} PNG.
          </p>
        </div>
        {loadError ? (
          <p style={{ color: "#f87171", fontSize: "0.9rem" }}>{loadError}</p>
        ) : (
          <div style={stage}>
            <img
              ref={imgRef}
              src={imageSrc}
              alt=""
              draggable={false}
              style={{ width: displayW, height: displayH, display: "block" }}
              onLoad={(event) => {
                const img = event.currentTarget;
                const w = img.naturalWidth;
                const h = img.naturalHeight;
                setNatural({ w, h });
                setCrop(initialCrop(w, h));
                setLoadError(w > 0 && h > 0 ? null : "Image has no dimensions");
              }}
              onError={() => setLoadError("Could not decode the picked image")}
            />
            {crop && natural.w > 0 && (
              <div
                style={{
                  position: "absolute",
                  left: crop.x * scale,
                  top: crop.y * scale,
                  width: crop.size * scale,
                  height: crop.size * scale,
                  boxSizing: "border-box",
                  border: "2px solid rgba(255,255,255,0.95)",
                  boxShadow: "0 0 0 9999px rgba(0,0,0,0.55)",
                  cursor: "move",
                  touchAction: "none",
                }}
                onPointerDown={(event) => beginDrag(event, "move")}
                onPointerMove={onPointerMove}
                onPointerUp={endDrag}
                onPointerCancel={endDrag}
              >
                {[1, 2].map((third) => (
                  <span
                    key={`v-${third}`}
                    style={{
                      position: "absolute",
                      top: 0,
                      bottom: 0,
                      left: `${(third / 3) * 100}%`,
                      width: 1,
                      background: "rgba(255,255,255,0.45)",
                      pointerEvents: "none",
                    }}
                  />
                ))}
                {[1, 2].map((third) => (
                  <span
                    key={`h-${third}`}
                    style={{
                      position: "absolute",
                      left: 0,
                      right: 0,
                      top: `${(third / 3) * 100}%`,
                      height: 1,
                      background: "rgba(255,255,255,0.45)",
                      pointerEvents: "none",
                    }}
                  />
                ))}
                {HANDLE.map((corner) => (
                  <span
                    key={corner}
                    onPointerDown={(event) => beginDrag(event, corner)}
                    onPointerMove={onPointerMove}
                    onPointerUp={endDrag}
                    onPointerCancel={endDrag}
                    style={{
                      position: "absolute",
                      width: 12,
                      height: 12,
                      background: "#fff",
                      border: "1px solid rgba(15,23,42,0.55)",
                      boxSizing: "border-box",
                      cursor: HANDLE_CURSOR[corner],
                      touchAction: "none",
                      ...(corner.includes("n") ? { top: 0 } : { bottom: 0 }),
                      ...(corner.includes("w") ? { left: 0 } : { right: 0 }),
                    }}
                  />
                ))}
              </div>
            )}
          </div>
        )}
        <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.75rem" }}>
          <button
            type="button"
            className="btn-secondary"
            disabled={saving}
            onClick={onCancel}
            style={{ marginTop: 0, padding: "0.55rem 1.1rem" }}
          >
            Cancel
          </button>
          <button
            type="button"
            className="btn-primary"
            disabled={saving || !crop || Boolean(loadError)}
            onClick={() => void handleSave()}
            style={{ padding: "0.55rem 1.1rem" }}
          >
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );

  return createPortal(modal, document.body);
}
