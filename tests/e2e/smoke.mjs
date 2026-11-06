// Transport proof: does tauri-driver launch the app under Xvfb and let us
// read the DOM back? Nothing app-specific — if this fails, the harness is
// broken, not the app. Run this before trusting any assertion in run.mjs.

import { setup, teardown } from "./harness.mjs";

let driver;
let failed = false;
try {
  driver = await setup();

  const title = await driver.getTitle();
  console.log(`document.title = ${JSON.stringify(title)}`);
  if (!title || !/coding assistants/i.test(title)) {
    throw new Error(`unexpected document.title: ${JSON.stringify(title)}`);
  }

  // Prove the React tree actually mounted, not just an empty shell.
  const rootHtmlLen = await driver.executeScript(
    "return (document.getElementById('root') || {}).innerHTML?.length || 0",
  );
  console.log(`#root innerHTML length = ${rootHtmlLen}`);
  if (rootHtmlLen < 200) {
    throw new Error(`#root looks empty (innerHTML ${rootHtmlLen} chars) — blank-window launch`);
  }

  console.log("\nSMOKE OK — WebDriver transport works, app renders.");
} catch (err) {
  failed = true;
  console.error(`\nSMOKE FAILED: ${err.message}`);
} finally {
  await teardown(driver);
}
process.exit(failed ? 1 : 0);
