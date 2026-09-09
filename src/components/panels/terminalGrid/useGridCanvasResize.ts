import { useCallback, useEffect, useState } from "react";
import type { RefObject } from "react";
import { canvasSizeKey, parseCanvasSize, type CanvasSize } from "./gridConstants";

export type ResizeDirection = "right" | "bottom" | "corner";

export interface UseGridCanvasResizeOptions {
  workspace: string;
  containerRef: RefObject<HTMLDivElement | null>;
  onSizeChange?: (size: CanvasSize | null) => void;
}

export function useGridCanvasResize({
  workspace,
  containerRef,
  onSizeChange,
}: UseGridCanvasResizeOptions) {
  const [canvasSize, setCanvasSize] = useState<CanvasSize | null>(() => {
    try {
      return parseCanvasSize(localStorage.getItem(canvasSizeKey(workspace)));
    } catch {
      return null;
    }
  });

  const [isResizing, setIsResizing] = useState(false);

  useEffect(() => {
    try {
      const restored = parseCanvasSize(localStorage.getItem(canvasSizeKey(workspace)));
      setCanvasSize(restored);
      onSizeChange?.(restored);
    } catch {
      setCanvasSize(null);
      onSizeChange?.(null);
    }
  }, [workspace, onSizeChange]);

  useEffect(() => {
    try {
      if (canvasSize) {
        localStorage.setItem(canvasSizeKey(workspace), JSON.stringify(canvasSize));
      } else {
        localStorage.removeItem(canvasSizeKey(workspace));
      }
    } catch {
      /* ignore quota */
    }
  }, [workspace, canvasSize]);

  const resetCanvasSize = useCallback(() => {
    setCanvasSize(null);
    onSizeChange?.(null);
    try {
      localStorage.removeItem(canvasSizeKey(workspace));
    } catch {
      /* ignore */
    }
  }, [workspace, onSizeChange]);

  const startResize = useCallback(
    (event: React.PointerEvent<HTMLDivElement>, direction: ResizeDirection) => {
      if (event.button !== 0) return;
      const handle = event.currentTarget;
      try {
        handle.setPointerCapture(event.pointerId);
      } catch {
        /* ignore */
      }

      setIsResizing(true);
      const startX = event.clientX;
      const startY = event.clientY;

      const container = containerRef.current;
      const containerRect = container?.getBoundingClientRect();
      const initialWidth =
        canvasSize?.width ?? (containerRect?.width && containerRect.width > 0 ? containerRect.width : (container?.offsetWidth || 900));
      const initialHeight =
        canvasSize?.height ?? (containerRect?.height && containerRect.height > 0 ? containerRect.height : (container?.offsetHeight || 600));
      const parentRect = container?.parentElement?.getBoundingClientRect();
      const parentWidth =
        parentRect?.width && parentRect.width > 0
          ? parentRect.width
          : (container?.parentElement?.clientWidth || window.innerWidth || 1200);

      const onMove = (e: PointerEvent) => {
        const dx = e.clientX - startX;
        const dy = e.clientY - startY;

        let nextWidth = initialWidth;
        let nextHeight = initialHeight;

        if (direction === "right" || direction === "corner") {
          nextWidth = Math.max(360, Math.min(parentWidth, Math.round(initialWidth + dx)));
        }
        if (direction === "bottom" || direction === "corner") {
          nextHeight = Math.max(280, Math.round(initialHeight + dy));
        }

        const nextSize: CanvasSize = { width: nextWidth, height: nextHeight };
        setCanvasSize(nextSize);
        onSizeChange?.(nextSize);
      };

      const onUp = () => {
        try {
          handle.releasePointerCapture(event.pointerId);
        } catch {
          /* ignore */
        }
        setIsResizing(false);
        handle.removeEventListener("pointermove", onMove);
        handle.removeEventListener("pointerup", onUp);
        handle.removeEventListener("pointercancel", onUp);
      };

      handle.addEventListener("pointermove", onMove);
      handle.addEventListener("pointerup", onUp);
      handle.addEventListener("pointercancel", onUp);
    },
    [canvasSize, containerRef, onSizeChange],
  );

  return {
    canvasSize,
    isResizing,
    startResize,
    resetCanvasSize,
  };
}
