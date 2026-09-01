/**
 * Display-name labels for known providers. This is only a label map — the
 * actual list of selectable providers is driven by what
 * `get_available_models` returns (see `providerOptions` in ModelSelect).
 */
export const PROVIDERS: Record<string, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  claude: "Claude",
  gemini: "Gemini",
  google: "Google",
  grok: "Grok",
  opencode: "OpenCode",
  deepseek: "DeepSeek",
  vibe: "Vibe",
  chat: "Chat",
  codex: "Codex",
  github_copilot: "GitHub Copilot",
};

export interface HubMessage {
  id: string;
  from_agent: string;
  to_agent: string;
  body: string;
  subject: string | null;
  kind: string;
  status: string;
  created_at: string;
}

export interface HubAgent {
  id: string;
  display_name: string;
  team_member?: boolean;
  /** Attachment id of the agent's profile image, when set. */
  avatar_attachment_id?: string | null;
}

export interface WorkSession {
  id: string;
  name: string;
  created_at: string;
  member_ids: string[];
}

/** Chat refresh. Capture is the four-provider on-disk transcript scan. */
export type HubRefreshOptions = {
  includeCapture?: boolean;
};

export function sameHubMessages(left: HubMessage[], right: HubMessage[]): boolean {
  if (left.length !== right.length) return false;
  return left.every((message, index) => {
    const other = right[index];
    return message.id === other.id
      && message.body === other.body
      && message.status === other.status
      && message.subject === other.subject;
  });
}

export function sameHubAgents(left: HubAgent[], right: HubAgent[]): boolean {
  if (left.length !== right.length) return false;
  return left.every((agent, index) => {
    const other = right[index];
    return agent.id === other.id
      && agent.display_name === other.display_name
      && agent.team_member === other.team_member
      && agent.avatar_attachment_id === other.avatar_attachment_id;
  });
}

export function loadWorkspaceRoot(): string {
  try {
    return localStorage.getItem("ca.workspaceRoot") || "./workspace";
  } catch {
    return "./workspace";
  }
}
