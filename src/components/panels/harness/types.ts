export type HarnessSessionMode = "observed" | "managed";
export type HarnessSessionState = "ready" | "busy" | "queued" | "unavailable" | "stopped";

export interface HarnessSessionRegistration {
  harness: string;
  workspace: string;
  disk_session_id: string;
  leader_socket: string | null;
  registered_at: string;
  mode: HarnessSessionMode;
  state: HarnessSessionState;
  managed_pid: number | null;
  writer_owner: string | null;
  writer_acquired_at: string | null;
}

export interface RelaunchOutcome {
  harness: string;
  killed_pid: number | null;
  resumed_session_id: string | null;
  detail: string;
}

export interface HarnessDeliveryNotice {
  harness: string;
  status: string;
  detail: string;
  retryable: boolean;
  messageId?: string | null;
  body?: string;
  isTask?: boolean;
  isWake?: boolean;
}

export const HARNESS_PREREQUISITES: Record<string, string> = {
  grok: "Hub inject needs a live `grok --leader` session (`~/.grok/leader.sock`). A standalone TUI cannot receive Hub messages. Use Connect / Resume in Shared Hub → Channels or here. Capture still reads the on-disk session.",
  chat: "Inject needs a persisted Codex thread id. If the writer is busy, retry later — do not start a second writer.",
  claude: "Two-way delivery needs an opt-in Claude Channel. Without it this session is capture-only.",
  gemini: "Managed delivery needs an app-owned agy stream-json worker. Do not attach to an interactive TUI.",
};

export function harnessTone(mode: HarnessSessionMode, state: HarnessSessionState): { label: string; color: string; border: string; bg: string } {
  if (state === "unavailable") {
    return { label: "unavailable", color: "#fecaca", border: "rgba(248, 113, 113, 0.75)", bg: "rgba(127, 29, 29, 0.55)" };
  }
  if (state === "busy") {
    return { label: "busy", color: "#fde68a", border: "rgba(251, 191, 36, 0.8)", bg: "rgba(120, 53, 15, 0.55)" };
  }
  if (state === "queued") {
    return { label: "queued", color: "#fdba74", border: "rgba(251, 146, 60, 0.85)", bg: "rgba(124, 45, 18, 0.5)" };
  }
  if (state === "stopped") {
    return { label: "stopped", color: "#e2e8f0", border: "rgba(148, 163, 184, 0.6)", bg: "rgba(30, 41, 59, 0.7)" };
  }
  if (mode === "managed") {
    return { label: "managed", color: "#bbf7d0", border: "rgba(52, 211, 153, 0.8)", bg: "rgba(6, 78, 59, 0.55)" };
  }
  return { label: "observed", color: "#a5f3fc", border: "rgba(34, 211, 238, 0.75)", bg: "rgba(14, 116, 144, 0.4)" };
}

export const HARNESS_STATE_LEGEND: Array<{ mode: HarnessSessionMode; state: HarnessSessionState }> = [
  { mode: "managed", state: "ready" },
  { mode: "observed", state: "ready" },
  { mode: "managed", state: "busy" },
  { mode: "managed", state: "queued" },
  { mode: "observed", state: "unavailable" },
];

export function isSuccessfulInject(status: string): boolean {
  const lowered = status.toLowerCase();
  return lowered === "delivered" || lowered === "started" || lowered === "ok" || lowered === "spawned";
}

export function injectNotice(status: string, _detail: string): { retryable: boolean; tone: "ok" | "warn" | "bad" } {
  const lowered = status.toLowerCase();
  if (isSuccessfulInject(lowered)) return { retryable: false, tone: "ok" };
  if (lowered === "queued" || lowered === "busy") return { retryable: true, tone: "warn" };
  return { retryable: lowered === "unavailable", tone: "bad" };
}
