import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    // Two built-output suites invoke Vite. Run files serially so a clean CI
    // runner does not compete for CPU while those hooks build `dist/`.
    fileParallelism: false,
    hookTimeout: 30_000,
    include: [
      "tests/unit/**/*.{test,spec}.{ts,tsx}",
      "tests/integration/**/*.{test,spec}.{ts,tsx}",
    ],
  },
});
