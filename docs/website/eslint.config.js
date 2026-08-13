import js from "@eslint/js";
import globals from "globals";
import tseslint from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";

export default [
  { ignores: ["dist/**", "node_modules/**", "src/content/**"] },
  js.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}", "scripts/**/*.ts", "tests/**/*.{ts,tsx}", "*.config.{js,ts}"],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: { ...globals.browser, ...globals.node },
      parser: tsParser,
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
    plugins: { "@typescript-eslint": tseslint },
    rules: {
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": ["warn", { argsIgnorePattern: "^_" }],
      "no-undef": "off",
    },
  },
  {
    files: ["tests/cypress/**/*.{js,ts}"],
    languageOptions: {
      globals: {
        ...globals.node,
        cy: "readonly",
        describe: "readonly",
        it: "readonly",
      },
    },
  },
];
