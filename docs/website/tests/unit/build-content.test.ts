import assert from "node:assert/strict";
import { test } from "vitest";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  curatedCorpusFiles,
  extractHeaders,
  extractTitle,
  parseDoc,
  slugFor,
  resolveLinkTarget,
  validateAndRewriteLinks,
  makeFileExistsChecker,
  type ParsedDoc,
} from "../../scripts/build-content.ts";

function makeFixtureDocsRoot(files: Record<string, string>): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "ca-docs-fixture-"));
  for (const [relativePath, content] of Object.entries(files)) {
    const full = path.join(root, relativePath);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content);
  }
  return root;
}

test("slugFor preserves nested slashes and lowercases, matching the roadmap example", () => {
  assert.equal(slugFor("moon/roadmaps/ui.md"), "moon/roadmaps/ui");
  assert.equal(slugFor("DOCUMENTATION_STANDARDS.md"), "documentation_standards");
});

test("curatedCorpusFiles only includes the locked globs, excluding archive/research/reports", () => {
  const root = makeFixtureDocsRoot({
    "index.md": "# Index",
    "adr/0001-decision.md": "# Decision",
    "adr/nested/0002-decision.md": "# Nested decision",
    "moon/ROADMAP.md": "# Roadmap",
    "moon/CHANGELOG.md": "# Changelog",
    "moon/roadmaps/ui.md": "# UI",
    "moon/archive/old-report.md": "# Old",
    "moon/research/notes.md": "# Notes",
    "moon/reports/q1.md": "# Q1",
    "moon/some-other-file.md": "# Not curated",
  });
  const files = curatedCorpusFiles(root).sort();
  assert.deepEqual(files, [
    "adr/0001-decision.md",
    "adr/nested/0002-decision.md",
    "index.md",
    "moon/CHANGELOG.md",
    "moon/ROADMAP.md",
    "moon/roadmaps/ui.md",
  ].sort());
});

test("frontmatter overrides title, nav_group, order, and description", () => {
  const root = makeFixtureDocsRoot({
    "index.md": [
      "---",
      "title: Custom Title",
      "description: A custom description.",
      "nav_group: Start Here",
      "order: 9",
      "---",
      "",
      "# Fallback Title",
      "",
      "Body text.",
    ].join("\n"),
  });
  const doc = parseDoc(root, "index.md");
  assert.equal(doc.title, "Custom Title");
  assert.equal(doc.description, "A custom description.");
  assert.equal(doc.summary, "A custom description.");
  assert.equal(doc.category, "Start Here");
  assert.equal(doc.order, 9);
});

test("title falls back to the first H1 when frontmatter omits it", () => {
  const root = makeFixtureDocsRoot({ "index.md": "# Real Title\n\nBody." });
  const doc = parseDoc(root, "index.md");
  assert.equal(doc.title, "Real Title");
});

test("extractTitle falls back to a humanized filename when there is no H1", () => {
  assert.equal(extractTitle("no heading here", "some_file-name"), "some file name");
});

test("extractHeaders produces GitHub-style slugs, including duplicate disambiguation", () => {
  const headers = extractHeaders([
    "# Title",
    "## Overview",
    "## Overview",
    "### Sub Heading!",
    "```",
    "## not a real heading inside a code fence",
    "```",
  ].join("\n"));
  assert.deepEqual(
    headers.map((h) => h.id),
    ["title", "overview", "overview-1", "sub-heading"],
  );
});

test("draft frontmatter is exposed on the parsed doc for the caller to reject", () => {
  const root = makeFixtureDocsRoot({
    "draft.md": ["---", "draft: true", "---", "", "# Draft"].join("\n"),
  });
  const doc = parseDoc(root, "draft.md");
  assert.equal(doc.frontmatter.draft, true);
});

function fakeDoc(overrides: Partial<ParsedDoc>): ParsedDoc {
  return {
    slug: "a",
    relativePath: "a.md",
    dir: ".",
    frontmatter: {},
    body: "",
    headers: [],
    title: "A",
    summary: "A",
    category: "Core & Architecture",
    order: 1,
    ...overrides,
  };
}

test("resolveLinkTarget ignores external, mailto, and same-page anchor links", () => {
  assert.equal(resolveLinkTarget(".", "https://example.com"), null);
  assert.equal(resolveLinkTarget(".", "mailto:team@example.com"), null);
  assert.equal(resolveLinkTarget(".", "#section"), null);
  assert.equal(resolveLinkTarget(".", "other.md"), "other.md");
  assert.equal(resolveLinkTarget("moon", "roadmaps/ui.md"), "moon/roadmaps/ui.md");
});

test("validateAndRewriteLinks rewrites in-corpus links to HashRouter paths", () => {
  const from = fakeDoc({
    slug: "a",
    relativePath: "a.md",
    dir: ".",
    body: "See [B](b.md) for details.",
  });
  const to = fakeDoc({ slug: "b", relativePath: "b.md", headers: [{ id: "intro", text: "Intro", level: 1 }] });
  const slugSet = new Set(["a", "b"]);
  const headerIdsBySlug = new Map([
    ["a", new Set<string>()],
    ["b", new Set(["intro"])],
  ]);
  const result = validateAndRewriteLinks([from, to], slugSet, headerIdsBySlug, () => false);
  assert.equal(result.brokenLinks.length, 0);
  assert.equal(result.brokenAnchors.length, 0);
  assert.equal(result.rewritten.get("a"), "See [B](/#/docs/b) for details.");
});

test("validateAndRewriteLinks records a real-but-excluded target as unpublished instead of failing", () => {
  const doc = fakeDoc({ slug: "a", relativePath: "a.md", dir: ".", body: "See [old](../moon/archive/old.md)." });
  const result = validateAndRewriteLinks(
    [doc],
    new Set(["a"]),
    new Map([["a", new Set<string>()]]),
    (relativePath) => relativePath === "../moon/archive/old.md",
  );
  assert.equal(result.brokenLinks.length, 0);
  assert.deepEqual(result.unpublishedLinks, [{ fromSlug: "a", targetPath: "../moon/archive/old.md" }]);
  assert.equal(result.rewritten.has("a"), false);
});

test("validateAndRewriteLinks fails the build on a link to a file that exists nowhere", () => {
  const doc = fakeDoc({ slug: "a", relativePath: "a.md", dir: ".", body: "See [ghost](nowhere.md)." });
  const result = validateAndRewriteLinks([doc], new Set(["a"]), new Map([["a", new Set<string>()]]), () => false);
  assert.equal(result.brokenLinks.length, 1);
  assert.match(result.brokenLinks[0], /does not resolve to any real file/);
});

test("validateAndRewriteLinks flags a broken in-corpus heading anchor", () => {
  const from = fakeDoc({ slug: "a", relativePath: "a.md", dir: ".", body: "See [B](b.md#missing)." });
  const to = fakeDoc({ slug: "b", relativePath: "b.md" });
  const result = validateAndRewriteLinks(
    [from, to],
    new Set(["a", "b"]),
    new Map([
      ["a", new Set<string>()],
      ["b", new Set(["real-heading"])],
    ]),
    () => false,
  );
  assert.equal(result.brokenAnchors.length, 1);
  assert.match(result.brokenAnchors[0], /anchor "#missing"/);
});

test("validateAndRewriteLinks flags a broken same-page anchor", () => {
  const doc = fakeDoc({ slug: "a", relativePath: "a.md", dir: ".", body: "Jump to [it](#nope)." });
  const result = validateAndRewriteLinks([doc], new Set(["a"]), new Map([["a", new Set(["real"])]]), () => false);
  assert.equal(result.brokenAnchors.length, 1);
  assert.match(result.brokenAnchors[0], /same-page anchor "#nope"/);
});

test("makeFileExistsChecker resolves paths that escape docs/ via ../ against the real filesystem", () => {
  const root = makeFixtureDocsRoot({ "index.md": "# Index" });
  const siblingDir = path.join(path.dirname(root), "sibling-fixture");
  fs.mkdirSync(siblingDir, { recursive: true });
  fs.writeFileSync(path.join(siblingDir, "README.md"), "# Sibling");
  const relative = path.relative(root, path.join(siblingDir, "README.md"));
  const exists = makeFileExistsChecker(root);
  assert.equal(exists(relative), true);
  assert.equal(exists("does-not-exist.md"), false);
});
