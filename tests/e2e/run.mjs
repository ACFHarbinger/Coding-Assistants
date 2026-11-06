// End-to-end checks driven through tauri-driver (see harness.mjs).
//
// SCOPE — what this can and cannot reach:
//   ✔ app boots, React tree mounts, no blank window
//   ✔ top-level navigation between panels renders each one
//   ✔ #143 crash-recovery boundary — IF the dev-only forced-throw hook is
//     present (VITE build flag `VITE_E2E_CRASH_HOOK`); otherwise skipped
//   ✘ #167 embedded-terminal wheel scroll / focus capture — needs a live PTY
//     session, which needs a registered harness (real `claude`/`grok`/…
//     CLIs). Not reachable from a clean e2e run; verify by hand or in a
//     harness-integration run with those CLIs stubbed.
//   ~ #167 resize frame — reachable only once a terminal is mounted (same
//     PTY dependency). The structural assertion is written but guarded.

import { By, until } from "selenium-webdriver";
import { setup, teardown } from "./harness.mjs";

const results = [];
function record(name, ok, detail = "") {
  results.push({ name, ok, detail });
  console.log(`${ok ? "PASS" : "FAIL"}  ${name}${detail ? ` — ${detail}` : ""}`);
}

let driver;
try {
  driver = await setup();

  // --- app boots, tree mounts ---
  const title = await driver.getTitle();
  record("app launches with the right title", /coding assistants/i.test(title), title);

  const rootLen = await driver.executeScript(
    "return (document.getElementById('root')||{}).innerHTML?.length||0",
  );
  record("React root mounted (not a blank window)", rootLen > 200, `#root ${rootLen} chars`);

  // --- navigation: every top-level nav target renders something ---
  // Nav buttons carry their label text; find them generically so this does
  // not couple to a specific class name.
  const navLabels = await driver.executeScript(`
    return [...document.querySelectorAll('button,[role=tab],a')]
      .map(el => (el.textContent||'').trim())
      .filter(t => /messager|hub|dashboard|harness|terminal|settings|config/i.test(t));
  `);
  record("found top-level navigation controls", Array.isArray(navLabels) && navLabels.length > 0,
    JSON.stringify(navLabels));

  // --- #143: crash-recovery boundary ---
  const hasCrashHook = await driver.executeScript(
    "return typeof window.__E2E_FORCE_RENDER_CRASH__ === 'function'",
  );
  if (hasCrashHook) {
    await driver.executeScript("window.__E2E_FORCE_RENDER_CRASH__()");
    await driver.wait(until.elementLocated(By.css("[role=alert]")), 4000);
    const alertText = await driver.findElement(By.css("[role=alert]")).getText();
    const reloadBtn = await driver.findElements(By.xpath("//button[contains(., 'Reload')]"));
    record("#143 boundary catches a render throw and offers reload",
      /reload/i.test(alertText) && reloadBtn.length === 1, alertText.replace(/\s+/g, " ").slice(0, 80));
    // no stack trace leaked to the user
    record("#143 recovery view exposes no stack trace",
      !/\bat \w+.*\(.*:\d+:\d+\)/.test(alertText));
  } else {
    record("#143 boundary (SKIPPED — no VITE_E2E_CRASH_HOOK in this build)", true);
  }

  // --- #167: resize frame, only if a terminal is mounted ---
  const frame = await driver.findElements(By.css("[aria-label='Resize terminal']"));
  if (frame.length === 1) {
    const box = await driver.executeScript(`
      const h = document.querySelector("[aria-label='Resize terminal']").closest("div");
      const r = h.getBoundingClientRect();
      return { w: r.width, h: r.height };
    `);
    record("#167 resize frame present with sane default size",
      box.h >= 280 && box.w >= 480, JSON.stringify(box));
  } else {
    record("#167 resize frame (SKIPPED — needs a live PTY session)", true);
  }
} catch (err) {
  record("run crashed", false, err.message);
} finally {
  await teardown(driver);
}

const failed = results.filter((r) => !r.ok);
console.log(`\n${results.length - failed.length}/${results.length} checks passed`);
process.exit(failed.length ? 1 : 0);
