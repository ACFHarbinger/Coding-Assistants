import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import manifest from "../../src/content/docs-manifest.json";
import searchIndex from "../../src/content/search-index.json";
import { createDocSearch, rankQuery, type SearchDoc } from "../../src/features/navigation/searchIndex";

describe("documentation-content integration", () => {
  it("connects the generated manifest, MiniSearch index, and canonical roadmap", () => {
    const roadmap = Object.values(manifest.docs).find((document) => document.slug === "moon/roadmaps/documentation");
    expect(roadmap).toBeDefined();
    expect(roadmap?.title).toContain("Documentation");

    const searchable = searchIndex.find((document) => document.id === roadmap?.slug);
    expect(searchable).toBeDefined();
    expect(readFileSync("../moon/roadmaps/documentation.md", "utf8")).toContain("Documentation & Website Roadmap");
  });

  it("returns the canonical roadmap when searching its title", () => {
    const search = createDocSearch(searchIndex as SearchDoc[]);
    const results = rankQuery(search, "documentation website roadmap");
    expect(results[0]?.id).toBe("moon/roadmaps/documentation");
  });
});
