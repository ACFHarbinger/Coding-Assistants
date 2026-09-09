import React, { useCallback, useRef, useState } from "react";
import type { LeafRect, Rect, DropEdge } from "./layoutTree";

export type DragTargetZone = DropEdge;

export interface DropTarget {
  leafId: string;
  harness: string;
  zone: DragTargetZone;
  rect: Rect;
}

export interface DragState {
  isDragging: boolean;
  sourceHarness: string | null;
  sourceLeafId: string | null;
  dropTarget: DropTarget | null;
}

function pointInRect(x: number, y: number, r: Rect): boolean {
  return x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height;
}

function determineDropZone(canvasX: number, canvasY: number, rect: Rect): DragTargetZone {
  const relX = (canvasX - rect.x) / Math.max(1, rect.width);
  const relY = (canvasY - rect.y) / Math.max(1, rect.height);

  const dLeft = relX;
  const dRight = 1 - relX;
  const dTop = relY;
  const dBottom = 1 - relY;
  const minDist = Math.min(dLeft, dRight, dTop, dBottom);

  if (minDist > 0.25) {
    return "center";
  }
  if (minDist === dLeft) return "left";
  if (minDist === dRight) return "right";
  if (minDist === dTop) return "top";
  return "bottom";
}

export function useTerminalGridDrag() {
  const [dragState, setDragState] = useState<DragState>({
    isDragging: false,
    sourceHarness: null,
    sourceLeafId: null,
    dropTarget: null,
  });

  const activeTargetRef = useRef<DropTarget | null>(null);

  const startPaneDrag = useCallback(
    (
      event: React.PointerEvent<HTMLDivElement>,
      harness: string,
      leafId: string,
      getLeaves: () => LeafRect[],
      containerEl: HTMLElement | null,
      onDrop: (sourceHarness: string, targetHarness: string, zone: DragTargetZone) => void,
    ) => {
      if (event.button !== 0) return;
      const targetEl = event.target as HTMLElement;
      if (targetEl.closest("button") || targetEl.closest("input") || targetEl.closest("select")) {
        return;
      }

      const handle = event.currentTarget;
      try {
        handle.setPointerCapture(event.pointerId);
      } catch {
        // Pointer capture may fail in certain synthetic or touch environments
      }

      const startX = event.clientX;
      const startY = event.clientY;
      let hasCrossedThreshold = false;
      activeTargetRef.current = null;

      const onMove = (e: PointerEvent) => {
        const dx = e.clientX - startX;
        const dy = e.clientY - startY;
        if (!hasCrossedThreshold && Math.sqrt(dx * dx + dy * dy) < 4) {
          return;
        }
        hasCrossedThreshold = true;

        if (!containerEl) return;
        const containerRect = containerEl.getBoundingClientRect();
        const canvasX = e.clientX - containerRect.left;
        const canvasY = e.clientY - containerRect.top;

        const currentLeaves = getLeaves();
        let matchedTarget: DropTarget | null = null;

        for (const leaf of currentLeaves) {
          if (leaf.node.harness === harness) continue;
          if (pointInRect(canvasX, canvasY, leaf.rect)) {
            const zone = determineDropZone(canvasX, canvasY, leaf.rect);
            matchedTarget = {
              leafId: leaf.node.id,
              harness: leaf.node.harness,
              zone,
              rect: leaf.rect,
            };
            break;
          }
        }

        activeTargetRef.current = matchedTarget;
        setDragState({
          isDragging: true,
          sourceHarness: harness,
          sourceLeafId: leafId,
          dropTarget: matchedTarget,
        });
      };

      const cleanup = () => {
        try {
          handle.releasePointerCapture(event.pointerId);
        } catch {
          /* ignore */
        }
        handle.removeEventListener("pointermove", onMove);
        handle.removeEventListener("pointerup", onUp);
        handle.removeEventListener("pointercancel", onCancel);
      };

      const onUp = () => {
        cleanup();
        const target = activeTargetRef.current;
        if (hasCrossedThreshold && target) {
          onDrop(harness, target.harness, target.zone);
        }
        activeTargetRef.current = null;
        setDragState({
          isDragging: false,
          sourceHarness: null,
          sourceLeafId: null,
          dropTarget: null,
        });
      };

      const onCancel = () => {
        cleanup();
        activeTargetRef.current = null;
        setDragState({
          isDragging: false,
          sourceHarness: null,
          sourceLeafId: null,
          dropTarget: null,
        });
      };

      handle.addEventListener("pointermove", onMove);
      handle.addEventListener("pointerup", onUp);
      handle.addEventListener("pointercancel", onCancel);
    },
    [],
  );

  return {
    dragState,
    startPaneDrag,
  };
}
