// Shared lifecycle for the WebDriver e2e checks.
//
// WebDriver drives the WebKitGTK webview through its inspector protocol, so it
// never needs compositor window focus or synthetic OS input — the wall that
// xdotool/spectacle hit on a live Wayland session. We still run everything
// under a throwaway Xvfb display so a run never touches the user's desktop and
// is reproducible in CI.
//
// Process tree: Xvfb :N  <-  tauri-driver :4444  ->  WebKitWebDriver :4445
//                                     |
//                                     +-> spawns the app binary
//
// Every spawned process is tracked and killed in teardown(), including on a
// failed assertion — leaving Xvfb or tauri-driver running was called out as a
// hazard, so teardown is unconditional.

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { setTimeout as sleep } from "node:timers/promises";
import { Builder } from "selenium-webdriver";
import { homedir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = new URL("../../", import.meta.url).pathname;
const DRIVER_PORT = 4444;
const DISPLAY_NUM = process.env.E2E_DISPLAY ?? ":99";

// The self-contained binary — loads the frontend from the bundled dist, not a
// vite dev server. `just test::e2e` builds it via `tauri build --debug --no-bundle`.
export const APP_BINARY =
  process.env.E2E_APP_BINARY ??
  join(REPO_ROOT, "target", "debug", "coding-assistants");

const TAURI_DRIVER =
  process.env.TAURI_DRIVER ?? join(homedir(), ".cargo", "bin", "tauri-driver");

const NATIVE_DRIVER = process.env.WEBKIT_WEBDRIVER ?? "/usr/bin/WebKitWebDriver";

const procs = [];

function track(child, name) {
  child.on("exit", (code, signal) => {
    if (process.env.E2E_VERBOSE) {
      console.error(`[harness] ${name} exited code=${code} signal=${signal}`);
    }
  });
  procs.push({ child, name });
  return child;
}

export function preflight() {
  const problems = [];
  if (!existsSync(APP_BINARY)) {
    problems.push(
      `app binary not found: ${APP_BINARY}\n` +
        `  build it first:  npx tauri build --debug --no-bundle`,
    );
  }
  if (!existsSync(TAURI_DRIVER)) {
    problems.push(
      `tauri-driver not found: ${TAURI_DRIVER}\n` +
        `  install it:  cargo install tauri-driver --locked`,
    );
  }
  if (!existsSync(NATIVE_DRIVER)) {
    problems.push(
      `WebKitWebDriver not found: ${NATIVE_DRIVER}\n` +
        `  install it:  sudo apt-get install -y webkitgtk-webdriver`,
    );
  }
  if (problems.length) {
    console.error("e2e preflight failed:\n\n" + problems.join("\n\n") + "\n");
    process.exit(2);
  }
}

async function startXvfb() {
  if (process.env.E2E_NO_XVFB) return null;
  const xvfb = track(
    spawn("Xvfb", [DISPLAY_NUM, "-screen", "0", "1600x1000x24", "-nolisten", "tcp"], {
      stdio: "ignore",
    }),
    "Xvfb",
  );
  await sleep(700);
  return xvfb;
}

async function startTauriDriver(display) {
  const env = {
    ...process.env,
    DISPLAY: display,
    // Usual WebKitGTK-on-virtual-display culprits for a blank window.
    WEBKIT_DISABLE_COMPOSITING_MODE: "1",
    WEBKIT_DISABLE_DMABUF_RENDERER: "1",
  };
  const driver = track(
    spawn(
      TAURI_DRIVER,
      ["--port", String(DRIVER_PORT), "--native-driver", NATIVE_DRIVER],
      { env, stdio: process.env.E2E_VERBOSE ? "inherit" : "ignore" },
    ),
    "tauri-driver",
  );
  await sleep(1200);
  return driver;
}

/**
 * Boots Xvfb + tauri-driver + the app and returns a connected WebDriver.
 * Always pair with teardown() in a finally block.
 */
export async function setup() {
  preflight();
  const display = DISPLAY_NUM;
  await startXvfb();
  await startTauriDriver(display);

  const capabilities = {
    browserName: "wry",
    "tauri:options": { application: APP_BINARY },
  };

  const driver = await new Builder()
    .usingServer(`http://127.0.0.1:${DRIVER_PORT}/`)
    .withCapabilities(capabilities)
    .build();

  return driver;
}

export async function teardown(driver) {
  try {
    if (driver) await driver.quit();
  } catch {
    /* driver may already be gone */
  }
  for (const { child } of procs.reverse()) {
    try {
      child.kill("SIGTERM");
    } catch {
      /* already dead */
    }
  }
  await sleep(300);
  for (const { child } of procs) {
    try {
      child.kill("SIGKILL");
    } catch {
      /* already dead */
    }
  }
}

// A run that throws must still not leak processes.
process.on("SIGINT", () => teardown().then(() => process.exit(130)));
process.on("SIGTERM", () => teardown().then(() => process.exit(143)));
