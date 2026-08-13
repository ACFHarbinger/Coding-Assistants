import type { DetectedProcess, HubAgent, HubMessage } from "./types";

export const AGENT_COLORS: Record<string, { bg: string; text: string; role: string }> = {
  human: { bg: "linear-gradient(135deg, #3b82f6, #1d4ed8)", text: "#93c5fd", role: "Human Developer" },
  grok: { bg: "linear-gradient(135deg, #10b981, #047857)", text: "#a7f3d0", role: "Lead Orchestrator" },
  chat: { bg: "linear-gradient(135deg, #06b6d4, #0e7490)", text: "#cffafe", role: "Co-Lead / Codex" },
  codex: { bg: "linear-gradient(135deg, #06b6d4, #0e7490)", text: "#cffafe", role: "Co-Lead / Codex" },
  claude: { bg: "linear-gradient(135deg, #f97316, #c2410c)", text: "#ffedd5", role: "Code Agent" },
  gemini: { bg: "linear-gradient(135deg, #a855f7, #7e22ce)", text: "#e9d5ff", role: "Supporting" },
};

export const DEFAULT_CHANNELS = [
  { id: "general", name: "#general", topic: "Team-wide coordination and announcement hub" },
  { id: "team-coordination", name: "#team-coordination", topic: "Inter-agent task claims, handoffs, and bus updates" },
  { id: "agent-memory", name: "#agent-memory", topic: "Shared memory insights, context tags, and audit events" },
  { id: "wakes-alerts", name: "#wakes-alerts", topic: "System wake requests and human approval gates" },
];

export const FALLBACK_ROSTER = ["human", "grok", "chat", "claude", "gemini"];

export function rosterAgentIds(hubAgents: HubAgent[]): string[] {
  const enrolled = hubAgents
    .filter(agent => agent.team_member && agent.id !== "system")
    .map(agent => agent.id);
  const ids = enrolled.length > 0 ? enrolled : FALLBACK_ROSTER;
  const rest = ids.filter(id => id !== "human");
  return ids.includes("human") ? ["human", ...rest] : ids;
}

export function teamWakeTargets(hubAgents: HubAgent[]): string[] {
  return rosterAgentIds(hubAgents).filter(id => id !== "human" && id !== "system");
}

export function agentInfo(
  agentId: string,
  hubAgents: HubAgent[],
  runningProcesses: DetectedProcess[],
) {
  const key = agentId.toLowerCase();
  const info = AGENT_COLORS[key] || {
    bg: "linear-gradient(135deg, #64748b, #334155)",
    text: "#e2e8f0",
    role: "Agent Participant",
  };
  const displayName = agentId === "human"
    ? "Harbinger (Human Dev)"
    : hubAgents.find((agent) => agent.id === agentId)?.display_name || agentId;
  const isRunning = runningProcesses.some((process) => {
    const detected = process.agent.toLowerCase();
    return detected === key || (key === "chat" && detected === "codex");
  });
  return { ...info, displayName, isRunning };
}

export const NEAR_BOTTOM_PX = 96;

export function isNearBottom(el: HTMLElement): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight <= NEAR_BOTTOM_PX;
}

/** Sorts by actual `created_at` timestamp — never assume the caller's
 * array is already in any particular order (the Hub's own message
 * queries return newest-first, `ORDER BY created_at DESC`). Ties (equal
 * timestamps, e.g. a team broadcast fan-out) preserve their relative
 * input order (stable sort). */
export function sortByCreatedAt<T extends { created_at: string }>(
  messages: T[],
  order: "asc" | "desc",
): T[] {
  const withIndex = messages.map((msg, index) => ({ msg, index }));
  withIndex.sort((a, b) => {
    const diff = Date.parse(a.msg.created_at) - Date.parse(b.msg.created_at);
    if (diff !== 0) return order === "asc" ? diff : -diff;
    return a.index - b.index;
  });
  return withIndex.map(({ msg }) => msg);
}

/** Whichever scroll edge the newest message currently renders at: the
 * bottom when oldest-first (ascending), the top when newest-first
 * (descending). */
export function isNearNewestEdge(el: HTMLElement, sortOrder: "asc" | "desc"): boolean {
  return sortOrder === "desc" ? el.scrollTop <= NEAR_BOTTOM_PX : isNearBottom(el);
}

/** The scrollTop value that pins the view to the newest message. */
export function newestEdgeScrollTop(el: HTMLElement, sortOrder: "asc" | "desc"): number {
  return sortOrder === "desc" ? 0 : el.scrollHeight;
}

/** Collapse team fan-out copies of one post without merging later distinct sends. */
export function channelDedupeKey(msg: HubMessage, channel: string): string {
  const prefix = `channel:${channel}`;
  if (msg.subject && msg.subject.startsWith(`${prefix}:`) && msg.subject.length > prefix.length + 1) {
    return msg.subject;
  }
  return `${msg.from_agent}|${msg.body}|${(msg.created_at || "").slice(0, 19)}`;
}

export const LAST_READ_STORAGE_KEY = "ca-messager-last-read";

export function loadLastRead(): Record<string, string> {
  try {
    const raw = localStorage.getItem(LAST_READ_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, string>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

export function belongsToChannel(message: HubMessage, channelId: string): boolean {
  if (message.status === "cancelled") return false;
  if (channelId === "general" && !message.subject?.startsWith("channel:")) {
    return true;
  }
  return message.subject === `channel:${channelId}`
    || Boolean(message.subject?.startsWith(`channel:${channelId}:`));
}

export function uniqueChannelPosts(messages: HubMessage[], channelId: string): HubMessage[] {
  const postsByKey = new Map<string, HubMessage>();
  const posts: HubMessage[] = [];
  for (const message of messages) {
    if (!belongsToChannel(message, channelId)) continue;
    const key = channelDedupeKey(message, channelId);
    const existing = postsByKey.get(key);
    if (existing) {
      if (message.to_agent && message.to_agent !== "team") {
        const recipients = existing.recipient_agents ?? [existing.to_agent].filter(Boolean);
        if (!recipients.includes(message.to_agent)) recipients.push(message.to_agent);
        existing.recipient_agents = recipients;
      }
      continue;
    }
    const post = {
      ...message,
      recipient_agents: message.to_agent && message.to_agent !== "team" ? [message.to_agent] : [],
    };
    postsByKey.set(key, post);
    posts.push(post);
  }
  return posts;
}

export function persistLastRead(next: Record<string, string>): Record<string, string> {
  try {
    localStorage.setItem(LAST_READ_STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* ignore quota / private-mode failures */
  }
  return next;
}

export function unreadPosts(messages: HubMessage[], channelId: string, watermark: string | undefined): HubMessage[] {
  return uniqueChannelPosts(messages, channelId).filter(message =>
    message.from_agent !== "human" && message.created_at > (watermark || "")
  );
}

export function latestCreatedAt(messages: HubMessage[]): string | null {
  if (messages.length === 0) return null;
  return messages.reduce((latest, message) =>
    message.created_at > latest ? message.created_at : latest,
  messages[0].created_at);
}

export function threadRootId(message: HubMessage, channel: string): string | null {
  const prefix = `channel:${channel}:thread:`;
  if (!message.subject?.startsWith(prefix)) return null;
  const rootId = message.subject.slice(prefix.length).split(":", 1)[0];
  return rootId || null;
}
