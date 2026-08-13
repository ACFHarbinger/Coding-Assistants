import assert from "node:assert/strict";
import { test } from "vitest";
import { createDocSearch, rankQuery, SEARCH_BOOST } from "../../src/features/navigation/searchIndex.ts";

test("title matches rank above body-only matches", () => {
  const search = createDocSearch([
    {
      id: "body-hit",
      title: "Unrelated title",
      category: "Core",
      summary: "Nothing here",
      content: "The hub stores wakes and tasks in SQLite.",
    },
    {
      id: "title-hit",
      title: "Hub overview",
      category: "Core",
      summary: "Introduction",
      content: "Other words.",
    },
  ]);
  const ranked = rankQuery(search, "hub");
  assert.ok(ranked.length >= 1);
  assert.equal(ranked[0].id, "title-hit");
  assert.ok(SEARCH_BOOST.title > SEARCH_BOOST.content);
});
