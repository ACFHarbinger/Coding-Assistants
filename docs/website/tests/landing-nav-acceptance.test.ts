import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const html = readFileSync("index.html", "utf8");
const shell = readFileSync("src/app/AppShell.tsx", "utf8");
const landing = readFileSync("src/features/landing/LandingPage.tsx", "utf8");
const palette = readFileSync("src/features/navigation/CommandPalette.tsx", "utf8");
const theme = readFileSync("src/app/ThemeProvider.tsx", "utf8");

test("entry HTML defaults dark and boots stored theme before paint", () => {
  assert.match(html, /<html lang="en" class="dark"/);
  assert.match(html, /localStorage\.getItem\("ca-website-theme"\)/);
  assert.doesNotMatch(html, /fonts\.googleapis\.com|fonts\.gstatic\.com/);
});

test("landing keeps product CTAs and the Hub graphic", () => {
  assert.match(landing, /to="\/docs"/);
  assert.match(landing, /ArchitectureGraphic/);
  assert.match(landing, /github\.com\/ACFHarbinger\/Coding-Assistants/);
  assert.match(landing, /#6366f1|#a855f7|indigo-500|purple-300/);
  assert.match(landing, /local Messager hub/);
  assert.doesNotMatch(landing, /slack/i);
});

test("navigation chrome uses slash-based W2 slugs and skip link", () => {
  assert.match(shell, /href="#main-content"/);
  assert.match(shell, /\/docs\/moon\/roadmaps\/documentation/);
  assert.doesNotMatch(shell, /moon-roadmaps-documentation/);
  assert.match(shell, /aria-expanded=\{menuOpen\}/);
  assert.match(shell, /aria-controls="mobile-nav"/);
});

test("search and theme controls stay local and keyboarded", () => {
  assert.match(palette, /metaKey \|\| event\.ctrlKey/);
  assert.match(palette, /createDocSearch/);
  assert.match(theme, /ca-website-theme/);
  assert.match(theme, /"dark" \| "light" \| "system"/);
});
