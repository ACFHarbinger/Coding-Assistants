import { defineConfig } from "cypress";

export default defineConfig({
  e2e: {
    baseUrl: process.env.CYPRESS_BASE_URL || 'http://localhost:5173',
    supportFile: false,
    specPattern: [
      "tests/cypress/e2e/**/*.cy.js",
      "tests/cypress/smoke/**/*.cy.js",
    ],
    video: false,
  },
});
