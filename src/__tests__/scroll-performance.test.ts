// @ts-nocheck — reads source CSS via node:fs; the app tsconfig has no Node types.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const src = (name: string) => readFileSync(resolve(import.meta.dirname, "..", name), "utf8");

function section(css: string, startMarker: string, endMarker: string): string {
  const start = css.indexOf(startMarker);
  const end = css.indexOf(endMarker, start + startMarker.length);
  expect(start).toBeGreaterThanOrEqual(0);
  expect(end).toBeGreaterThan(start);
  return css.slice(start, end);
}

describe("desktop scroll performance CSS (#213)", () => {
  const indexCss = src("index.css");
  const scrollCss = src("scroll-performance.css");
  const appTsx = src("App.tsx");
  const combined = `${indexCss}\n${scrollCss}`;

  it("does not use content-visibility or contain-intrinsic-size", () => {
    expect(combined).not.toMatch(/content-visibility\s*:/);
    expect(combined).not.toMatch(/contain-intrinsic-size\s*:/);
  });

  it("keeps .main-content on instant scroll with no smooth behavior", () => {
    const main = section(indexCss, ".main-content {", "\n}");
    expect(main).toMatch(/scroll-behavior:\s*auto/);
    expect(main).toMatch(/overflow-anchor:\s*none/);
    expect(main).toMatch(/overscroll-behavior-y:\s*contain/);
    expect(combined).not.toMatch(/scroll-behavior\s*:\s*smooth/);
  });

  it("does not leave a transform containing block on fade-in cards", () => {
    const fadeInFrames = section(indexCss, "@keyframes fadeIn", "@keyframes pulse");
    expect(fadeInFrames).not.toMatch(/transform\s*:/);
    expect(fadeInFrames).not.toMatch(/animation:[^;]*forwards/);
    expect(scrollCss).toMatch(/\.main-content\s+\.fade-in\s*\{[^}]*transform:\s*none/s);
  });

  it("does not put backdrop-filter on the app header inline", () => {
    expect(appTsx).not.toMatch(/backdropFilter/);
    expect(scrollCss).toMatch(/backdrop-filter:\s*none\s*!important/);
  });
});
