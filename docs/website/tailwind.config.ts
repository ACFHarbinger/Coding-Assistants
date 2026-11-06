import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      blur: { glass: "20px" },
      borderRadius: { card: "16px" },
      colors: {
        canvas: "#020617",
        primary: { DEFAULT: "#6366f1", hover: "#4f46e5" },
        accent: "#a855f7",
        glass: {
          card: "rgba(15, 23, 42, 0.92)",
          tint: "rgba(255, 255, 255, 0.03)",
          border: "rgba(255, 255, 255, 0.08)",
        },
      },
      fontFamily: {
        sans: ["Inter", "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "ui-monospace", "monospace"],
      },
    },
  },
  plugins: [],
} satisfies Config;
