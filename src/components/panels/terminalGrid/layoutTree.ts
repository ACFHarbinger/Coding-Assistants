/**
 * Pure layout tree module for tiled terminal grid (#295, #296, U15).
 * Designed to be zero-dependency and reusable by `ca tui` (U7).
 */

export type SplitDirection = "row" | "col";

export interface LeafNode {
  type: "leaf";
  id: string;
  harness: string;
}

export interface SplitNode {
  type: "split";
  id: string;
  direction: SplitDirection;
  ratio: number;
  first: LayoutNode;
  second: LayoutNode;
}

export type LayoutNode = LeafNode | SplitNode;

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface LeafRect {
  node: LeafNode;
  rect: Rect;
}

export interface SplitterInfo {
  id: string;
  direction: SplitDirection;
  rect: Rect;
  splitNode: SplitNode;
  parentBounds: Rect;
}

export interface ComputedLayout {
  leaves: LeafRect[];
  splitters: SplitterInfo[];
}

export function clampRatio(ratio: number, min = 0.1, max = 0.9): number {
  if (isNaN(ratio)) return 0.5;
  return Math.max(min, Math.min(max, ratio));
}

let nextIdCounter = 0;
export function generateNodeId(prefix: string): string {
  nextIdCounter += 1;
  return `${prefix}-${Date.now().toString(36)}-${nextIdCounter.toString(36)}`;
}

/**
 * Computes pixel rects for leaves and splitters within `bounds`.
 */
export function computeRects(
  root: LayoutNode | null,
  bounds: Rect,
  splitterThickness = 6,
): ComputedLayout {
  const leaves: LeafRect[] = [];
  const splitters: SplitterInfo[] = [];

  if (!root || bounds.width <= 0 || bounds.height <= 0) {
    return { leaves, splitters };
  }

  function walk(node: LayoutNode, currentBounds: Rect) {
    if (node.type === "leaf") {
      leaves.push({ node, rect: currentBounds });
      return;
    }

    const ratio = clampRatio(node.ratio, 0.05, 0.95);

    if (node.direction === "row") {
      const totalAvail = Math.max(0, currentBounds.width - splitterThickness);
      const firstW = Math.round(totalAvail * ratio);
      const secondW = Math.max(0, totalAvail - firstW);

      const firstBounds: Rect = {
        x: currentBounds.x,
        y: currentBounds.y,
        width: firstW,
        height: currentBounds.height,
      };

      const splitterRect: Rect = {
        x: currentBounds.x + firstW,
        y: currentBounds.y,
        width: splitterThickness,
        height: currentBounds.height,
      };

      const secondBounds: Rect = {
        x: currentBounds.x + firstW + splitterThickness,
        y: currentBounds.y,
        width: secondW,
        height: currentBounds.height,
      };

      splitters.push({
        id: node.id,
        direction: node.direction,
        rect: splitterRect,
        splitNode: node,
        parentBounds: currentBounds,
      });

      walk(node.first, firstBounds);
      walk(node.second, secondBounds);
    } else {
      const totalAvail = Math.max(0, currentBounds.height - splitterThickness);
      const firstH = Math.round(totalAvail * ratio);
      const secondH = Math.max(0, totalAvail - firstH);

      const firstBounds: Rect = {
        x: currentBounds.x,
        y: currentBounds.y,
        width: currentBounds.width,
        height: firstH,
      };

      const splitterRect: Rect = {
        x: currentBounds.x,
        y: currentBounds.y + firstH,
        width: currentBounds.width,
        height: splitterThickness,
      };

      const secondBounds: Rect = {
        x: currentBounds.x,
        y: currentBounds.y + firstH + splitterThickness,
        width: currentBounds.width,
        height: secondH,
      };

      splitters.push({
        id: node.id,
        direction: node.direction,
        rect: splitterRect,
        splitNode: node,
        parentBounds: currentBounds,
      });

      walk(node.first, firstBounds);
      walk(node.second, secondBounds);
    }
  }

  walk(root, bounds);
  return { leaves, splitters };
}

/**
 * Collect all leaf nodes in tree order.
 */
export function collectLeaves(root: LayoutNode | null): LeafNode[] {
  if (!root) return [];
  if (root.type === "leaf") return [root];
  return [...collectLeaves(root.first), ...collectLeaves(root.second)];
}

/**
 * Find a leaf node by id or harness name.
 */
export function findLeaf(root: LayoutNode | null, idOrHarness: string): LeafNode | null {
  if (!root) return null;
  if (root.type === "leaf") {
    if (root.id === idOrHarness || root.harness === idOrHarness) return root;
    return null;
  }
  return findLeaf(root.first, idOrHarness) || findLeaf(root.second, idOrHarness);
}

/**
 * Check if the layout tree contains a harness.
 */
export function hasHarness(root: LayoutNode | null, harness: string): boolean {
  return findLeaf(root, harness) !== null;
}

/**
 * Insert a harness leaf into the layout.
 * If harness is already present, returns tree unchanged (no duplicate panes).
 * If targetLeafId is omitted, splits the leaf with the largest area, or balances by row/col.
 */
export function insertLeaf(
  root: LayoutNode | null,
  harness: string,
  targetLeafId?: string,
  direction?: SplitDirection,
): LayoutNode {
  const newLeaf: LeafNode = {
    type: "leaf",
    id: generateNodeId(`leaf-${harness}`),
    harness,
  };

  if (!root) {
    return newLeaf;
  }

  // Prevent duplicate harness panes per spec constraint
  if (hasHarness(root, harness)) {
    return root;
  }

  // Target selection
  let targetId = targetLeafId;
  let targetDirection = direction;

  if (!targetId) {
    // Compute rects on a standard virtual canvas (1000x1000) to find the largest pane
    const { leaves } = computeRects(root, { x: 0, y: 0, width: 1000, height: 1000 });
    if (leaves.length > 0) {
      let largest = leaves[0];
      let maxArea = largest.rect.width * largest.rect.height;
      for (let i = 1; i < leaves.length; i += 1) {
        const area = leaves[i].rect.width * leaves[i].rect.height;
        if (area > maxArea) {
          maxArea = area;
          largest = leaves[i];
        }
      }
      targetId = largest.node.id;
      if (!targetDirection) {
        targetDirection = largest.rect.width >= largest.rect.height ? "row" : "col";
      }
    }
  }

  function replaceInNode(node: LayoutNode): LayoutNode {
    if (node.type === "leaf") {
      if (node.id === targetId || !targetId) {
        const chosenDir: SplitDirection = targetDirection ?? "row";
        return {
          type: "split",
          id: generateNodeId("split"),
          direction: chosenDir,
          ratio: 0.5,
          first: node,
          second: newLeaf,
        };
      }
      return node;
    }

    return {
      ...node,
      first: replaceInNode(node.first),
      second: replaceInNode(node.second),
    };
  }

  return replaceInNode(root);
}

/**
 * Removes a leaf node from the layout tree.
 * When a child of a split is removed, the surviving sibling promotes up to take the split's place.
 */
export function removeLeaf(
  root: LayoutNode | null,
  leafIdOrHarness: string,
): LayoutNode | null {
  if (!root) return null;

  if (root.type === "leaf") {
    if (root.id === leafIdOrHarness || root.harness === leafIdOrHarness) {
      return null;
    }
    return root;
  }

  // Split node: check direct children first
  if (root.first.type === "leaf" && (root.first.id === leafIdOrHarness || root.first.harness === leafIdOrHarness)) {
    return root.second;
  }
  if (root.second.type === "leaf" && (root.second.id === leafIdOrHarness || root.second.harness === leafIdOrHarness)) {
    return root.first;
  }

  // Recurse into both branches
  const newFirst = removeLeaf(root.first, leafIdOrHarness);
  const newSecond = removeLeaf(root.second, leafIdOrHarness);

  if (!newFirst) return newSecond;
  if (!newSecond) return newFirst;

  return {
    ...root,
    first: newFirst,
    second: newSecond,
  };
}

/**
 * Resizes a split node identified by `splitId`.
 */
export function resizeSplit(
  root: LayoutNode,
  splitId: string,
  newRatio: number,
  minRatio = 0.1,
  maxRatio = 0.9,
): LayoutNode {
  const clamped = clampRatio(newRatio, minRatio, maxRatio);

  function walk(node: LayoutNode): LayoutNode {
    if (node.type === "leaf") return node;
    if (node.id === splitId) {
      return { ...node, ratio: clamped };
    }
    return {
      ...node,
      first: walk(node.first),
      second: walk(node.second),
    };
  }

  return walk(root);
}

/**
 * Validates whether an object matches the LayoutNode schema.
 */
export function isValidLayoutNode(node: unknown): node is LayoutNode {
  if (!node || typeof node !== "object") return false;
  const anyNode = node as Record<string, unknown>;

  if (anyNode.type === "leaf") {
    return typeof anyNode.id === "string" && typeof anyNode.harness === "string";
  }

  if (anyNode.type === "split") {
    return (
      typeof anyNode.id === "string" &&
      (anyNode.direction === "row" || anyNode.direction === "col") &&
      typeof anyNode.ratio === "number" &&
      isValidLayoutNode(anyNode.first) &&
      isValidLayoutNode(anyNode.second)
    );
  }

  return false;
}

/**
 * Serializes a layout tree to a JSON string.
 */
export function serializeLayout(root: LayoutNode | null): string {
  if (!root) return "";
  return JSON.stringify(root);
}

/**
 * Safely parses and validates a serialized layout tree string.
 * Returns null if parsing or validation fails.
 */
export function deserializeLayout(raw: string | null | undefined): LayoutNode | null {
  if (!raw || !raw.trim()) return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (isValidLayoutNode(parsed)) {
      return parsed;
    }
    return null;
  } catch {
    return null;
  }
}

export type DropEdge = "top" | "bottom" | "left" | "right" | "center";

/**
 * Swaps two leaves in the layout tree identified by leaf ID or harness name.
 * If either leaf is missing or both resolve to the same leaf, returns root unchanged.
 */
export function swapLeaves(
  root: LayoutNode,
  a: string,
  b: string,
): LayoutNode {
  const leafA = findLeaf(root, a);
  const leafB = findLeaf(root, b);
  if (!leafA || !leafB || leafA.id === leafB.id) {
    return root;
  }
  const leafAId = leafA.id;
  const leafBId = leafB.id;
  const leafAHarness = leafA.harness;
  const leafBHarness = leafB.harness;

  function walk(node: LayoutNode): LayoutNode {
    if (node.type === "leaf") {
      if (node.id === leafAId) {
        return { ...node, harness: leafBHarness };
      }
      if (node.id === leafBId) {
        return { ...node, harness: leafAHarness };
      }
      return node;
    }
    return {
      ...node,
      first: walk(node.first),
      second: walk(node.second),
    };
  }

  return walk(root);
}

/**
 * Moves a leaf node to an edge of another target leaf in the layout tree.
 * Removes the source leaf from its current position (collapsing its parent split),
 * then splits the target leaf along the specified edge.
 * If source and target are the same, or either is not found, returns root unchanged.
 */
export function moveLeaf(
  root: LayoutNode,
  sourceLeafIdOrHarness: string,
  targetLeafIdOrHarness: string,
  edge: Exclude<DropEdge, "center">,
): LayoutNode {
  const sourceLeaf = findLeaf(root, sourceLeafIdOrHarness);
  const targetLeaf = findLeaf(root, targetLeafIdOrHarness);
  if (!sourceLeaf || !targetLeaf || sourceLeaf.id === targetLeaf.id) {
    return root;
  }

  const fixedSource: LeafNode = { type: "leaf", id: sourceLeaf.id, harness: sourceLeaf.harness };
  const fixedTarget: LeafNode = { type: "leaf", id: targetLeaf.id, harness: targetLeaf.harness };

  const treeWithoutSource = removeLeaf(root, fixedSource.id);
  if (!treeWithoutSource) {
    return root;
  }

  const direction: SplitDirection = edge === "left" || edge === "right" ? "row" : "col";
  const sourceIsFirst = edge === "left" || edge === "top";

  function insertAtTarget(node: LayoutNode): LayoutNode {
    if (node.type === "leaf") {
      if (node.id === fixedTarget.id) {
        const newSplit: SplitNode = {
          type: "split",
          id: generateNodeId("split"),
          direction,
          ratio: 0.5,
          first: sourceIsFirst ? fixedSource : node,
          second: sourceIsFirst ? node : fixedSource,
        };
        return newSplit;
      }
      return node;
    }

    return {
      ...node,
      first: insertAtTarget(node.first),
      second: insertAtTarget(node.second),
    };
  }

  return insertAtTarget(treeWithoutSource);
}
