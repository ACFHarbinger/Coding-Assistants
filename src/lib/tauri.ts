import { invoke as tauriInvoke } from "@tauri-apps/api/core";

type TauriWindow = Window & {
  __TAURI_INTERNALS__?: unknown;
};

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && Boolean((window as TauriWindow).__TAURI_INTERNALS__);
}

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error(`Tauri runtime unavailable; command '${command}' requires the desktop app`);
  }
  return tauriInvoke<T>(command, args);
}
