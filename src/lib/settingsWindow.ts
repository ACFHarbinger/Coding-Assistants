import { isTauriRuntime } from "./tauri";

const SETTINGS_WINDOW_LABEL = "settings";
const SETTINGS_WINDOW_URL = "index.html#/settings";

// Opens the standalone Settings window (S3 / #129), or focuses it if it is
// already open/hidden. The app's global `CloseRequested` handler hides
// windows rather than destroying them (tray-resident behavior), so a
// reopen must `show()` before `setFocus()` — a hidden window ignores focus
// alone.
export async function openSettingsWindow(): Promise<void> {
  if (!isTauriRuntime()) {
    // Browser dev-mode fallback: no Tauri multiwindow API available.
    window.open(SETTINGS_WINDOW_URL, "ca-settings");
    return;
  }

  const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");

  const existing = await WebviewWindow.getByLabel(SETTINGS_WINDOW_LABEL);
  if (existing) {
    await existing.show();
    await existing.setFocus();
    return;
  }

  const settingsWindow = new WebviewWindow(SETTINGS_WINDOW_LABEL, {
    url: SETTINGS_WINDOW_URL,
    title: "Settings",
    width: 900,
    height: 700,
    minWidth: 640,
    minHeight: 480,
    resizable: true,
  });
  settingsWindow.once("tauri://error", (event) => {
    console.error("Failed to open the Settings window:", event);
  });
}
