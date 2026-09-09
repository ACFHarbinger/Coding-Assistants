import { describe, it, expect } from "vitest";
import {
  computeRects,
  insertLeaf,
  removeLeaf,
  resizeSplit,
  collectLeaves,
  findLeaf,
  hasHarness,
  serializeLayout,
  deserializeLayout,
  isValidLayoutNode,
  clampRatio,
  swapLeaves,
  moveLeaf,
  type LayoutNode,
} from "../layoutTree";

describe("layoutTree - pure tree operations", () => {
  it("clamps ratios within valid bounds", () => {
    expect(clampRatio(0.5)).toBe(0.5);
    expect(clampRatio(-0.2)).toBe(0.1);
    expect(clampRatio(1.5)).toBe(0.9);
    expect(clampRatio(NaN)).toBe(0.5);
  });

  describe("computeRects", () => {
    it("returns empty result for null or invalid bounds", () => {
      const res = computeRects(null, { x: 0, y: 0, width: 800, height: 600 });
      expect(res.leaves).toHaveLength(0);
      expect(res.splitters).toHaveLength(0);

      const leaf: LayoutNode = { type: "leaf", id: "l1", harness: "claude" };
      expect(computeRects(leaf, { x: 0, y: 0, width: 0, height: 600 }).leaves).toHaveLength(0);
    });

    it("computes single leaf filling entire bounds", () => {
      const leaf: LayoutNode = { type: "leaf", id: "l1", harness: "claude" };
      const bounds = { x: 10, y: 20, width: 800, height: 600 };
      const res = computeRects(leaf, bounds);

      expect(res.leaves).toHaveLength(1);
      expect(res.leaves[0].node).toBe(leaf);
      expect(res.leaves[0].rect).toEqual(bounds);
      expect(res.splitters).toHaveLength(0);
    });

    it("computes row split with splitter between children", () => {
      const root: LayoutNode = {
        type: "split",
        id: "s1",
        direction: "row",
        ratio: 0.5,
        first: { type: "leaf", id: "l1", harness: "grok" },
        second: { type: "leaf", id: "l2", harness: "claude" },
      };

      const bounds = { x: 0, y: 0, width: 1006, height: 600 };
      const res = computeRects(root, bounds, 6);

      // Total available = 1006 - 6 = 1000
      // firstW = 500, secondW = 500
      expect(res.leaves).toHaveLength(2);
      expect(res.leaves[0].node.harness).toBe("grok");
      expect(res.leaves[0].rect).toEqual({ x: 0, y: 0, width: 500, height: 600 });

      expect(res.leaves[1].node.harness).toBe("claude");
      expect(res.leaves[1].rect).toEqual({ x: 506, y: 0, width: 500, height: 600 });

      expect(res.splitters).toHaveLength(1);
      expect(res.splitters[0].id).toBe("s1");
      expect(res.splitters[0].direction).toBe("row");
      expect(res.splitters[0].rect).toEqual({ x: 500, y: 0, width: 6, height: 600 });
    });

    it("computes col split with splitter between children", () => {
      const root: LayoutNode = {
        type: "split",
        id: "s2",
        direction: "col",
        ratio: 0.4,
        first: { type: "leaf", id: "l1", harness: "chat" },
        second: { type: "leaf", id: "l2", harness: "gemini" },
      };

      const bounds = { x: 0, y: 0, width: 800, height: 506 };
      const res = computeRects(root, bounds, 6);

      // Total available = 506 - 6 = 500
      // firstH = 200, secondH = 300
      expect(res.leaves[0].rect).toEqual({ x: 0, y: 0, width: 800, height: 200 });
      expect(res.leaves[1].rect).toEqual({ x: 0, y: 206, width: 800, height: 300 });

      expect(res.splitters[0].direction).toBe("col");
      expect(res.splitters[0].rect).toEqual({ x: 0, y: 200, width: 800, height: 6 });
    });
  });

  describe("insertLeaf", () => {
    it("creates first leaf when root is null", () => {
      const res = insertLeaf(null, "claude");
      expect(res.type).toBe("leaf");
      if (res.type === "leaf") {
        expect(res.harness).toBe("claude");
      }
    });

    it("does not insert duplicate harness", () => {
      const root: LayoutNode = { type: "leaf", id: "l1", harness: "claude" };
      const res = insertLeaf(root, "claude");
      expect(res).toBe(root);
    });

    it("splits single leaf into a split node", () => {
      const root: LayoutNode = { type: "leaf", id: "l1", harness: "grok" };
      const res = insertLeaf(root, "chat");

      expect(res.type).toBe("split");
      if (res.type === "split") {
        expect(res.first.type).toBe("leaf");
        expect(res.second.type).toBe("leaf");
        expect((res.first as any).harness).toBe("grok");
        expect((res.second as any).harness).toBe("chat");
      }
    });

    it("targets a specific leaf when targetLeafId is provided", () => {
      const root: LayoutNode = {
        type: "split",
        id: "s1",
        direction: "row",
        ratio: 0.5,
        first: { type: "leaf", id: "target-leaf", harness: "grok" },
        second: { type: "leaf", id: "other-leaf", harness: "claude" },
      };

      const res = insertLeaf(root, "muse", "target-leaf", "col");
      expect(res.type).toBe("split");
      if (res.type === "split") {
        expect(res.second.type).toBe("leaf"); // other-leaf untouched
        expect(res.first.type).toBe("split");
        const subSplit = res.first as any;
        expect(subSplit.direction).toBe("col");
        expect(subSplit.first.harness).toBe("grok");
        expect(subSplit.second.harness).toBe("muse");
      }
    });
  });

  describe("removeLeaf", () => {
    it("returns null when removing only leaf", () => {
      const root: LayoutNode = { type: "leaf", id: "l1", harness: "claude" };
      expect(removeLeaf(root, "claude")).toBeNull();
      expect(removeLeaf(root, "l1")).toBeNull();
      expect(removeLeaf(root, "grok")).toBe(root);
    });

    it("promotes surviving sibling when removing first child of split", () => {
      const root: LayoutNode = {
        type: "split",
        id: "s1",
        direction: "row",
        ratio: 0.5,
        first: { type: "leaf", id: "l1", harness: "grok" },
        second: { type: "leaf", id: "l2", harness: "claude" },
      };

      const res = removeLeaf(root, "grok");
      expect(res).toEqual({ type: "leaf", id: "l2", harness: "claude" });
    });

    it("promotes surviving sibling when removing second child of split", () => {
      const root: LayoutNode = {
        type: "split",
        id: "s1",
        direction: "row",
        ratio: 0.5,
        first: { type: "leaf", id: "l1", harness: "grok" },
        second: { type: "leaf", id: "l2", harness: "claude" },
      };

      const res = removeLeaf(root, "claude");
      expect(res).toEqual({ type: "leaf", id: "l1", harness: "grok" });
    });

    it("removes deeply nested leaf and preserves other branches", () => {
      const root: LayoutNode = {
        type: "split",
        id: "s1",
        direction: "row",
        ratio: 0.5,
        first: {
          type: "split",
          id: "s2",
          direction: "col",
          ratio: 0.5,
          first: { type: "leaf", id: "l1", harness: "grok" },
          second: { type: "leaf", id: "l2", harness: "chat" },
        },
        second: { type: "leaf", id: "l3", harness: "claude" },
      };

      const res = removeLeaf(root, "chat");
      expect(res?.type).toBe("split");
      if (res?.type === "split") {
        expect(res.first).toEqual({ type: "leaf", id: "l1", harness: "grok" });
        expect(res.second).toEqual({ type: "leaf", id: "l3", harness: "claude" });
      }
    });
  });

  describe("resizeSplit", () => {
    it("updates ratio of target split node within clamped range", () => {
      const root: LayoutNode = {
        type: "split",
        id: "s1",
        direction: "row",
        ratio: 0.5,
        first: { type: "leaf", id: "l1", harness: "grok" },
        second: { type: "leaf", id: "l2", harness: "claude" },
      };

      const res = resizeSplit(root, "s1", 0.7);
      expect((res as any).ratio).toBe(0.7);

      const clampedRes = resizeSplit(root, "s1", 0.99);
      expect((clampedRes as any).ratio).toBe(0.9);
    });
  });

  describe("query and serialization", () => {
    const tree: LayoutNode = {
      type: "split",
      id: "s1",
      direction: "row",
      ratio: 0.5,
      first: { type: "leaf", id: "l1", harness: "grok" },
      second: { type: "leaf", id: "l2", harness: "cursor" },
    };

    it("collects leaves and checks harness presence", () => {
      const leaves = collectLeaves(tree);
      expect(leaves.map((l) => l.harness)).toEqual(["grok", "cursor"]);

      expect(hasHarness(tree, "grok")).toBe(true);
      expect(hasHarness(tree, "cursor")).toBe(true);
      expect(hasHarness(tree, "gemini")).toBe(false);

      expect(findLeaf(tree, "cursor")?.id).toBe("l2");
      expect(findLeaf(tree, "nonexistent")).toBeNull();
    });

    it("serializes and deserializes cleanly", () => {
      const json = serializeLayout(tree);
      expect(typeof json).toBe("string");

      const parsed = deserializeLayout(json);
      expect(parsed).toEqual(tree);
    });

    it("validates layout schema and rejects invalid JSON/structure", () => {
      expect(isValidLayoutNode(null)).toBe(false);
      expect(isValidLayoutNode({})).toBe(false);
      expect(isValidLayoutNode({ type: "leaf" })).toBe(false);
      expect(isValidLayoutNode({ type: "leaf", id: "1", harness: "grok" })).toBe(true);

      expect(deserializeLayout("")).toBeNull();
      expect(deserializeLayout("invalid json {}")).toBeNull();
      expect(deserializeLayout(JSON.stringify({ type: "split", invalid: true }))).toBeNull();
    });
  });

  describe("swapLeaves", () => {
    const tree: LayoutNode = {
      type: "split",
      id: "s1",
      direction: "row",
      ratio: 0.5,
      first: { type: "leaf", id: "l1", harness: "grok" },
      second: {
        type: "split",
        id: "s2",
        direction: "col",
        ratio: 0.5,
        first: { type: "leaf", id: "l2", harness: "claude" },
        second: { type: "leaf", id: "l3", harness: "gemini" },
      },
    };

    it("swaps two leaves by id or harness", () => {
      const beforeRects = new Map(
        computeRects(tree, { x: 0, y: 0, width: 1000, height: 600 })
          .leaves.map(({ node, rect }) => [node.id, rect]),
      );
      const swapped = swapLeaves(tree, "grok", "gemini");
      expect(collectLeaves(swapped).map((l) => l.harness)).toEqual(["gemini", "claude", "grok"]);
      // Positions (leaf ids) do not move; only their harness payloads swap.
      expect(findLeaf(swapped, "gemini")?.id).toBe("l1");
      expect(findLeaf(swapped, "grok")?.id).toBe("l3");
      for (const { node, rect } of computeRects(swapped, { x: 0, y: 0, width: 1000, height: 600 }).leaves) {
        expect(rect).toEqual(beforeRects.get(node.id));
      }
    });

    it("returns tree unchanged if leaf is missing or same leaf", () => {
      expect(swapLeaves(tree, "grok", "nonexistent")).toBe(tree);
      expect(swapLeaves(tree, "grok", "grok")).toBe(tree);
      expect(swapLeaves(tree, "l1", "l1")).toBe(tree);
    });
  });

  describe("moveLeaf", () => {
    const simpleTree: LayoutNode = {
      type: "split",
      id: "s1",
      direction: "row",
      ratio: 0.5,
      first: { type: "leaf", id: "l1", harness: "grok" },
      second: { type: "leaf", id: "l2", harness: "claude" },
    };

    it("moves leaf to left edge of target (source becomes first in row split)", () => {
      const moved = moveLeaf(simpleTree, "claude", "grok", "left");
      expect(collectLeaves(moved).map((l) => l.harness)).toEqual(["claude", "grok"]);
      expect((moved as any).direction).toBe("row");
      expect((moved as any).first.harness).toBe("claude");
      expect((moved as any).second.harness).toBe("grok");
    });

    it("moves leaf to right edge of target (source becomes second in row split)", () => {
      const moved = moveLeaf(simpleTree, "grok", "claude", "right");
      expect(collectLeaves(moved).map((l) => l.harness)).toEqual(["claude", "grok"]);
      expect((moved as any).direction).toBe("row");
      expect((moved as any).first.harness).toBe("claude");
      expect((moved as any).second.harness).toBe("grok");
    });

    it("moves leaf to top edge of target (col split, source is first)", () => {
      const moved = moveLeaf(simpleTree, "claude", "grok", "top");
      expect(collectLeaves(moved).map((l) => l.harness)).toEqual(["claude", "grok"]);
      expect((moved as any).direction).toBe("col");
      expect((moved as any).first.harness).toBe("claude");
      expect((moved as any).second.harness).toBe("grok");
    });

    it("moves leaf to bottom edge of target (col split, target is first)", () => {
      const moved = moveLeaf(simpleTree, "claude", "grok", "bottom");
      expect(collectLeaves(moved).map((l) => l.harness)).toEqual(["grok", "claude"]);
      expect((moved as any).direction).toBe("col");
      expect((moved as any).first.harness).toBe("grok");
      expect((moved as any).second.harness).toBe("claude");
    });

    it("moves leaf across a complex multi-level split tree", () => {
      const complexTree: LayoutNode = {
        type: "split",
        id: "s1",
        direction: "row",
        ratio: 0.5,
        first: { type: "leaf", id: "l1", harness: "grok" },
        second: {
          type: "split",
          id: "s2",
          direction: "col",
          ratio: 0.5,
          first: { type: "leaf", id: "l2", harness: "claude" },
          second: { type: "leaf", id: "l3", harness: "gemini" },
        },
      };

      // Move grok to the bottom of gemini
      const moved = moveLeaf(complexTree, "grok", "gemini", "bottom");
      const leaves = collectLeaves(moved).map((l) => l.harness);
      expect(leaves).toEqual(["claude", "gemini", "grok"]);
    });

    it("returns root unchanged when moving same leaf or invalid leaf", () => {
      expect(moveLeaf(simpleTree, "grok", "grok", "left")).toBe(simpleTree);
      expect(moveLeaf(simpleTree, "grok", "nonexistent", "left")).toBe(simpleTree);
      expect(moveLeaf(simpleTree, "nonexistent", "grok", "left")).toBe(simpleTree);
    });
  });
});
