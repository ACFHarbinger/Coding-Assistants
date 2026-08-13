import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const OWNED = [
  "src/app/AppShell.tsx",
  "src/components/ThemeToggle.tsx",
  "src/features/landing/LandingPage.tsx",
  "src/features/landing/CapabilityGrid.tsx",
  "src/features/landing/QuickStart.tsx",
  "src/features/landing/ArchitectureGraphic.tsx",
  "src/features/navigation/CommandPalette.tsx",
  "src/features/docs/DocsLayout.tsx",
  "src/features/docs/DocsSidebar.tsx",
  "src/features/docs/TableOfContents.tsx",
  "src/features/docs/PrevNextNav.tsx",
  "src/features/docs/MarkdownArticle.tsx",
];
const markdownArticle = readFileSync("src/features/docs/MarkdownArticle.tsx", "utf8");
const docsLayout = readFileSync("src/features/docs/DocsLayout.tsx", "utf8");

test("website chrome does not use off-palette cyan utilities", () => {
  for (const file of OWNED) {
    const source = readFileSync(file, "utf8");
    assert.doesNotMatch(
      source,
      /(?:text|bg|border|from|to|shadow|ring)-cyan-|#24C8D8/,
      `${file} still contains cyan palette classes`,
    );
  }
});

test("reader filters React Markdown internals and sends unknown docs to the error view", () => {
  assert.match(markdownArticle, /node: _node/);
  assert.match(markdownArticle, /void _node/);
  assert.doesNotMatch(markdownArticle, /rehypeRaw/);
  assert.match(docsLayout, /<NotFoundPage\s*\/>/);
  assert.doesNotMatch(docsLayout, /<Navigate/);
});
