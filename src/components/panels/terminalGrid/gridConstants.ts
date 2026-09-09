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

export function terminalSessionId(harness: string, workspace: string): string {
  return `harness-terminal:${harness}:${workspace}`;
}
