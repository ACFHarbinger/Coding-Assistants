import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri` and the Cargo workspace's
      // shared build output. This is a multi-crate workspace (crates/hub,
      // crates/cli, crates/tui, crates/claude, src-tauri all share one
      // `target/` at the repo root, not a per-crate target/ under
      // src-tauri/), so ignoring only src-tauri/** still leaves the root
      // target/ tree watched. On Windows that's fatal, not just wasteful:
      // watching an object file mid-write under a concurrent `cargo build`
      // hits a hard file lock, which Node's fs.watch surfaces as an
      // uncaught EBUSY that crashes the whole `tauri dev` process (confirmed
      // live, 2026-08-16 — `target\debug\build\libsqlite3-sys-*\out\*.o`).
      // Linux/macOS don't lock like this, so this went unnoticed there.
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },
}));
