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
];

test("landing and navigation chrome do not use off-palette cyan utilities", () => {
  for (const file of OWNED) {
    const source = readFileSync(file, "utf8");
    assert.doesNotMatch(
      source,
      /(?:text|bg|border|from|to|shadow|ring)-cyan-|#24C8D8/,
      `${file} still contains cyan palette classes`,
    );
  }
});
