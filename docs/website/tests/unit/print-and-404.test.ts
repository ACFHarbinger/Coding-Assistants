// W7 (#123 follow-up): print stylesheet + custom 404 page. Static checks
// against the real built dist/ output, matching the pattern established in
// privacy-a11y.test.ts — no headless browser, no mocking.
import assert from "node:assert/strict";
import { beforeAll, test } from "vitest";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "../..");
const DIST = path.join(ROOT, "dist");

beforeAll(() => {
  execFileSync("npx", ["vite", "build"], { cwd: ROOT, stdio: "inherit" });
});

function readDistFiles(extension: string): string[] {
  const results: string[] = [];
  const walk = (dir: string) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.name.endsWith(extension)) results.push(fs.readFileSync(full, "utf-8"));
    }
  };
  walk(DIST);
  return results;
}

test("built CSS includes a print media block that hides chrome and forces a light article background", () => {
  const css = readDistFiles(".css").join("\n");
  assert.match(css, /@media print/);
  // Order-independent: rollup/postcss may reorder or rename custom
  // properties, but the literal selectors and declarations below are
  // untouched pass-through text from the source stylesheet.
  assert.match(css, /header[^{]*\{[^}]*display:\s*none/);
  assert.match(css, /aside[^{]*\{[^}]*display:\s*none/);
  assert.match(css, /\.markdown-body\s*\{[^}]*background:\s*#fff/i);
});

test("built JS includes the custom 404 page copy", () => {
  const js = readDistFiles(".js").join("\n");
  assert.ok(js.includes("Page not found"), "expected the NotFoundPage copy in the built bundle");
  assert.ok(js.includes("Go to homepage"), "expected the NotFoundPage's home link in the built bundle");
});

test("main.tsx routes the catch-all path to NotFoundPage, not a blind redirect", () => {
  const source = fs.readFileSync(path.join(ROOT, "src", "main.tsx"), "utf-8");
  assert.match(source, /<Route\s+path="\*"\s+element=\{<NotFoundPage \/>\}\s*\/>/);
  assert.doesNotMatch(source, /path="\*"[^>]*Navigate/, "the catch-all route should no longer bounce to / with no explanation");
});
