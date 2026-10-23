import { describe, expect, it } from "vitest";
import { NOC_COLORS, NOC_CSS_VAR_MAP, NOC_SURFACES } from "../tokens";

async function readIndexCss(): Promise<string> {
  const fsMod = "node:fs";
  const pathMod = "node:path";
  const fs = (await import(/* @vite-ignore */ fsMod)) as { readFileSync: (p: string, enc: string) => string };
  const path = (await import(/* @vite-ignore */ pathMod)) as { resolve: (...parts: string[]) => string };
  return fs.readFileSync(path.resolve("src/index.css"), "utf-8");
}

function hexToRgb(hex: string): [number, number, number] {
  const clean = hex.replace("#", "").trim();
  const r = Number.parseInt(clean.slice(0, 2), 16);
  const g = Number.parseInt(clean.slice(2, 4), 16);
  const b = Number.parseInt(clean.slice(4, 6), 16);
  return [r, g, b];
}

function relativeLuminance([r, g, b]: [number, number, number]): number {
  const channel = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrastRatio(hexA: string, hexB: string): number {
  const lumA = relativeLuminance(hexToRgb(hexA));
  const lumB = relativeLuminance(hexToRgb(hexB));
  const lighter = Math.max(lumA, lumB);
  const darker = Math.min(lumA, lumB);
  return (lighter + 0.05) / (darker + 0.05);
}

describe("NOC design tokens (V1 / #372)", () => {
  it("keeps tokens.ts and src/index.css :root --noc-* declarations in 1:1 sync", async () => {
    const cssText = await readIndexCss();
    for (const [varName, expectedValue] of Object.entries(NOC_CSS_VAR_MAP)) {
      const escapedVar = varName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      const pattern = new RegExp(`${escapedVar}\\s*:\\s*([^;]+);`);
      const match = cssText.match(pattern);
      expect(match, `Missing CSS custom property ${varName} in src/index.css`).not.toBeNull();
      expect(match?.[1].trim()).toBe(expectedValue);
    }
  });

  it("enforces WCAG AA (>= 4.5:1) contrast for text and micro-label tokens on NOC panel backgrounds", () => {
    const backgrounds = [NOC_SURFACES.bgPanel, NOC_SURFACES.bgDeep];
    const foregrounds: Record<string, string> = {
      textPrimary: NOC_SURFACES.textPrimary,
      textSecondary: NOC_SURFACES.textSecondary,
      textMuted: NOC_SURFACES.textMuted,
      idle: NOC_COLORS.idle,
      ok: NOC_COLORS.ok,
      warn: NOC_COLORS.warn,
      crit: NOC_COLORS.crit,
      cyan: NOC_COLORS.cyan,
      amber: NOC_COLORS.amber,
      orange: NOC_COLORS.orange,
      emerald: NOC_COLORS.emerald,
    };

    for (const bg of backgrounds) {
      for (const [name, fg] of Object.entries(foregrounds)) {
        const ratio = contrastRatio(fg, bg);
        expect(
          ratio,
          `Token ${name} (${fg}) on ${bg} has contrast ${ratio.toFixed(2)}:1 (expected >= 4.5:1)`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });
});
