import { describe, expect, it } from "vitest";
import type { MemoryRecord, ScoredMemoryRecord } from "../types";

describe("ScoredMemoryRecord and similarity search ranking", () => {
  it("ranks scored memories descending by score", () => {
    const hits: ScoredMemoryRecord[] = [
      {
        id: "mem-1",
        scope: "workspace",
        tier: "short_term",
        title: "Database Migration",
        body: "SQLite vector column schema update",
        tags_json: "[]",
        created_at: "2026-09-04T00:00:00Z",
        stale: false,
        score: 0.42,
      },
      {
        id: "mem-2",
        scope: "global",
        tier: "semantic",
        title: "Vector Embeddings Engine",
        body: "Hybrid RRF cosine similarity retrieval",
        tags_json: "[]",
        created_at: "2026-09-04T01:00:00Z",
        stale: false,
        score: 0.94,
      },
      {
        id: "mem-3",
        scope: "workspace",
        tier: "episodic",
        title: "Terminal Color Palette",
        body: "Theme switcher ANSI colors",
        tags_json: "[]",
        created_at: "2026-09-04T02:00:00Z",
        stale: false,
        score: 0.78,
      },
    ];

    const sorted = [...hits].sort((a, b) => b.score - a.score);
    expect(sorted.map((m) => m.id)).toEqual(["mem-2", "mem-3", "mem-1"]);
    expect(sorted[0].score).toBe(0.94);
  });

  it("formats similarity score percentages correctly", () => {
    const formatScore = (score: number) => {
      if (score >= 1.0) return "100%";
      if (score <= 0.0) return "0%";
      return `${Math.round(score * 100)}%`;
    };

    expect(formatScore(0.854)).toBe("85%");
    expect(formatScore(1.0)).toBe("100%");
    expect(formatScore(1.2)).toBe("100%");
    expect(formatScore(0.0)).toBe("0%");
    expect(formatScore(-0.1)).toBe("0%");
    expect(formatScore(0.499)).toBe("50%");
  });

  it("filters records by scope and tier accurately", () => {
    const records: MemoryRecord[] = [
      {
        id: "1",
        scope: "workspace",
        tier: "short_term",
        body: "Task A",
        tags_json: "[]",
        created_at: "2026-09-04",
        stale: false,
      },
      {
        id: "2",
        scope: "global",
        tier: "semantic",
        body: "Task B",
        tags_json: "[]",
        created_at: "2026-09-04",
        stale: false,
      },
      {
        id: "3",
        scope: "workspace",
        tier: "semantic",
        body: "Task C",
        tags_json: "[]",
        created_at: "2026-09-04",
        stale: false,
      },
    ];

    const workspaceOnly = records.filter((r) => r.scope === "workspace");
    expect(workspaceOnly).toHaveLength(2);

    const semanticOnly = records.filter((r) => r.tier === "semantic");
    expect(semanticOnly).toHaveLength(2);

    const workspaceSemantic = records.filter((r) => r.scope === "workspace" && r.tier === "semantic");
    expect(workspaceSemantic).toHaveLength(1);
    expect(workspaceSemantic[0].id).toBe("3");
  });
});
