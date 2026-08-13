export const PROVIDERS = {
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
}

export interface WorkSession {
  id: string;
  name: string;
  created_at: string;
  member_ids: string[];
}

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
      && agent.team_member === other.team_member;
  });
}

export function loadWorkspaceRoot(): string {
  try {
    return localStorage.getItem("ca.workspaceRoot") || "./workspace";
  } catch {
    return "./workspace";
  }
}
