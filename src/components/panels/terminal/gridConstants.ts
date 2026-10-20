export const ALL_HARNESSES = [
  "grok",
  "chat",
  "claude",
  "gemini",
  "muse",
  "cursor",
  "qwen",
  "kimi",
  "hermes",
  "deepseek",
] as const;
export type HarnessName = (typeof ALL_HARNESSES)[number];

/** CLI and TUI harnesses are processes. A web app is a local page. */
export type HarnessSurface = "cli" | "tui" | "web_app";

export const HARNESS_SURFACE: Record<string, HarnessSurface> = {
  grok: "tui",
  chat: "tui",
  claude: "tui",
  gemini: "tui",
  muse: "tui",
  cursor: "tui",
  qwen: "tui",
  kimi: "tui",
  hermes: "tui",
  vibe: "tui",
  opencode: "cli",
  deepseek: "web_app",
};

export function isWebHarness(harness: string): boolean {
  return HARNESS_SURFACE[harness] === "web_app";
}

export const DISPLAY_NAMES: Record<string, string> = {
  grok: "Grok",
  chat: "Codex",
  claude: "Claude",
  gemini: "Gemini",
  muse: "Muse",
  cursor: "Cursor",
  qwen: "Qwen",
  kimi: "Kimi",
  hermes: "Hermes",
  deepseek: "DeepSeek",
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

export function terminalSessionId(harness: string, workspace: string, instanceKey?: string | null): string {
  return instanceKey
    ? `harness-terminal:${harness}:${workspace}:${instanceKey}`
    : `harness-terminal:${harness}:${workspace}`;
}

export interface CanvasSize {
  width: number;
  height: number;
}

export const MIN_CANVAS_WIDTH = 360;
export const MIN_CANVAS_HEIGHT = 280;
export const MAX_CANVAS_HEIGHT = 20_000;

export function parseCanvasSize(raw: string | null | undefined): CanvasSize | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      typeof parsed.width === "number" &&
      typeof parsed.height === "number" &&
      Number.isFinite(parsed.width) &&
      Number.isFinite(parsed.height) &&
      parsed.width > 0 &&
      parsed.height > 0
    ) {
      return {
        width: Math.max(MIN_CANVAS_WIDTH, Math.round(parsed.width)),
        height: Math.min(MAX_CANVAS_HEIGHT, Math.max(MIN_CANVAS_HEIGHT, Math.round(parsed.height))),
      };
    }
    return null;
  } catch {
    return null;
  }
}
