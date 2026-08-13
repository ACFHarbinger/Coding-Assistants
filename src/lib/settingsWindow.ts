import { isTauriRuntime } from "./tauri";

const SETTINGS_WINDOW_LABEL = "settings";
const SETTINGS_WINDOW_URL = "index.html#/settings";

// Opens the standalone Settings window (S3 / #129), or focuses it if a
// close is still in flight when this fires again. Unlike the main window,
// Settings is a plain utility dialog — the Rust `CloseRequested` handler
// lets it actually close (not hide), so on a normal reopen `existing` is
// null here and we create a fresh window rather than depending on a
// hidden window resurrecting correctly.
export async function openSettingsWindow(): Promise<void> {
  if (!isTauriRuntime()) {
    // Browser dev-mode fallback: no Tauri multiwindow API available.
    window.open(SETTINGS_WINDOW_URL, "ca-settings");
    return;
  }

  const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");

  const existing = await WebviewWindow.getByLabel(SETTINGS_WINDOW_LABEL);
  if (existing) {
    try {
      await existing.show();
      await existing.setFocus();
      return;
    } catch (error) {
      console.error("Failed to reuse the existing Settings window, recreating it:", error);
      try {
        await existing.destroy();
      } catch {
        // Best-effort — we're about to create a fresh window with the same
        // label regardless of whether the stale one tears down cleanly.
      }
    }
  }

  const settingsWindow = new WebviewWindow(SETTINGS_WINDOW_LABEL, {
    url: SETTINGS_WINDOW_URL,
    title: "Settings",
    width: 900,
    height: 700,
    minWidth: 640,
    minHeight: 480,
    resizable: true,
    focus: true,
  });
  settingsWindow.once("tauri://error", (event) => {
    console.error("Failed to open the Settings window:", event);
  });
  settingsWindow.once("tauri://created", () => {
    // Some window managers ignore a freshly-mapped window's focus request
    // (focus-stealing prevention), so the window can end up open but
    // behind the main one with no visible sign anything happened.
    void settingsWindow.show();
    void settingsWindow.setFocus();
  });
}
