
export interface HubMessage {
  id: string;
  from_agent: string;
  to_agent: string;
  body: string;
  subject: string | null;
  kind: string;
  status: string;
  created_at: string;
  /** Client-only aggregation of the durable fan-out copies for one post. */
  recipient_agents?: string[];
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

export interface MemoryRecord {
  id: string;
  scope: string;
  tier: string;
  agent_id?: string | null;
  title?: string | null;
  body: string;
  tags_json: string;
  created_at: string;
  stale: boolean;
}

export interface DetectedProcess {
  pid: number;
  agent: string;
  provider: string;
  model: string;
  command: string;
}

export interface ChannelRecord {
  id: string;
  name: string;
  topic?: string | null;
  builtin: boolean;
  created_at: string;
}

export interface MessagerPanelProps {
  hubMessages: HubMessage[];
  hubAgents: HubAgent[];
  workSessions: WorkSession[];
  activeWorkSessionId: string | null;
  focusSessionId?: string | null;
  focusSessionToken?: number;
  workspacePath: string;
  onSelectWorkSession: (sessionId: string | null) => void;
  onRefresh: () => Promise<void>;
}

export interface TaggedSendOutcome {
  to_agent: string;
  accepted: boolean;
  message_id?: string | null;
  reason?: string | null;
}

export interface HarnessInjectResult {
  harness: string;
  status: string;
  detail: string;
}

export interface ContextMenuState {
  messageId: string;
  x: number;
  y: number;
}

export interface ReplyTarget {
  id: string;
  fromAgent: string;
  preview: string;
}
