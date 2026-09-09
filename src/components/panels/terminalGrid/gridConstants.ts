export const ALL_HARNESSES = ["grok", "chat", "claude", "gemini", "muse", "cursor"] as const;
export type HarnessName = (typeof ALL_HARNESSES)[number];

export const DISPLAY_NAMES: Record<string, string> = {
  grok: "Grok",
  chat: "Codex",
  claude: "Claude",
  gemini: "Gemini",
  muse: "Muse",
  cursor: "Cursor",
};

export function storageKey(workspace: string): string {
  return `ca.terminalGrid.layout.${workspace || "default"}`;
}

export function maxKey(workspace: string): string {
  return `ca.terminalGrid.maximized.${workspace || "default"}`;
}

export function canvasSizeKey(workspace: string): string {
  return `ca.terminalGrid.canvasSize.${workspace || "default"}`;
}

export function terminalSessionId(harness: string, workspace: string): string {
  return `harness-terminal:${harness}:${workspace}`;
}

export interface CanvasSize {
  width: number;
  height: number;
}

export function parseCanvasSize(raw: string | null | undefined): CanvasSize | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      typeof parsed.width === "number" &&
      typeof parsed.height === "number" &&
      parsed.width >= 360 &&
      parsed.height >= 280
    ) {
      return {
        width: Math.round(parsed.width),
        height: Math.round(parsed.height),
      };
    }
    return null;
  } catch {
    return null;
  }
}
