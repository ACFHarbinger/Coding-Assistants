// W7 (#123): static regression checks for the roadmap's privacy lock ("no
// analytics, no cookies, no third-party font or tracker requests") and a
// handful of source-level accessibility invariants. These are deliberately
// *static* — no headless browser, no network calls — so they run fast and
// deterministically in CI. They complement, not replace, a manual WCAG
// spot-check (see docs/website/RELEASE_CHECKLIST.md).
import assert from "node:assert/strict";
import test from "node:test";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const DIST = path.join(ROOT, "dist");

// Building once for the whole file keeps this fast; every test below reads
// the same already-built dist/ output.
test.before(() => {
  execFileSync("npx", ["vite", "build"], { cwd: ROOT, stdio: "inherit" });
});

function readDistFiles(extensions: string[]): { file: string; content: string }[] {
  const results: { file: string; content: string }[] = [];
  const walk = (dir: string) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else if (extensions.some((ext) => entry.name.endsWith(ext))) {
        results.push({ file: path.relative(DIST, full), content: fs.readFileSync(full, "utf-8") });
      }
    }
  };
  walk(dir_or_throw(DIST));
  return results;
}

function dir_or_throw(dir: string): string {
  if (!fs.existsSync(dir)) {
    throw new Error(`${dir} does not exist — did the build step run?`);
  }
  return dir;
}

// Third-party hosts/snippets the roadmap explicitly forbids at runtime:
// analytics, trackers, and non-self-hosted fonts. Matched as plain
// substrings against the built HTML/JS/CSS, so a forbidden request can't
// hide behind minification or string concatenation tricks that split a
// literal across variables (those wouldn't appear as this substring either
// way, but this check catches the overwhelmingly common case: a literal
// URL or script tag baked into the bundle).
const FORBIDDEN_SUBSTRINGS = [
  "fonts.googleapis.com",
  "fonts.gstatic.com",
  "www.google-analytics.com",
  "www.googletagmanager.com",
  "googletagmanager.com",
  "plausible.io",
  "cdn.segment.com",
  "api.mixpanel.com",
  "browser.sentry-cdn.com",
  "connect.facebook.net",
  "doubleclick.net",
  "hotjar.com",
];

test("built output makes no third-party font, analytics, or tracker requests", () => {
  const files = readDistFiles([".html", ".js", ".css"]);
  assert.ok(files.length > 0, "expected at least one built html/js/css file");

  const offenders: string[] = [];
  for (const { file, content } of files) {
    for (const needle of FORBIDDEN_SUBSTRINGS) {
      if (content.includes(needle)) {
        offenders.push(`${file}: contains "${needle}"`);
      }
    }
  }
  assert.deepEqual(offenders, [], `forbidden third-party references found:\n${offenders.join("\n")}`);
});

test("built output has no inline cookie-setting or common consent-banner markers", () => {
  const files = readDistFiles([".html", ".js"]);
  const cookieMarkers = ["document.cookie =", "cookieconsent", "cookie-consent", "gdpr-consent"];
  const offenders: string[] = [];
  for (const { file, content } of files) {
    for (const needle of cookieMarkers) {
      if (content.toLowerCase().includes(needle.toLowerCase())) {
        offenders.push(`${file}: contains "${needle}"`);
      }
    }
  }
  assert.deepEqual(offenders, [], `cookie/consent markers found:\n${offenders.join("\n")}`);
});

test("built index.html only references same-origin/relative assets (self-hosted fonts, no CDN <link>/<script> src)", () => {
  const indexPath = path.join(DIST, "index.html");
  const html = fs.readFileSync(indexPath, "utf-8");
  const externalUrls = [...html.matchAll(/(?:href|src)="(https?:\/\/[^"]+)"/g)].map((m) => m[1]);
  assert.deepEqual(externalUrls, [], `dist/index.html references external URLs: ${externalUrls.join(", ")}`);
});

test("AGPL license reference survives the production build", () => {
  const files = readDistFiles([".js"]);
  const hasLicenseMention = files.some(({ content }) => content.includes("AGPL"));
  assert.ok(hasLicenseMention, "expected the built bundle to still reference the AGPL license (footer text)");
});

// Source-level accessibility invariants. These check the shared app shell's
// source directly rather than a rendered DOM, since that's what a *static*
// check can verify without a browser — a real skip-link, semantic
// landmarks, and a lang attribute are structural properties of the
// markup the shell always renders, not something that varies per route.
test("app shell source keeps a skip-to-content link and semantic landmarks", () => {
  const shellPath = path.join(ROOT, "src", "app", "AppShell.tsx");
  const source = fs.readFileSync(shellPath, "utf-8");
  assert.match(source, /href="#main-content"/, "expected a skip-to-content link targeting #main-content");
  assert.match(source, /id="main-content"/, "expected a <main id=\"main-content\"> landmark");
  assert.match(source, /<header/, "expected a <header> landmark");
  assert.match(source, /<footer/, "expected a <footer> landmark");
  assert.match(source, /<nav\b/, "expected at least one <nav> landmark");
});

test("index.html declares a language and a viewport meta tag", () => {
  const html = fs.readFileSync(path.join(ROOT, "index.html"), "utf-8");
  assert.match(html, /<html[^>]*\slang="[a-z-]+"/i, "expected <html lang=\"...\">");
  assert.match(html, /name="viewport"/, "expected a viewport meta tag");
});
