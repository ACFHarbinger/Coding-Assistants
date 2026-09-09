export interface MemoryRecord {
  id: string;
  scope: string;
  workspace_path?: string | null;
  tier: string;
  agent_id?: string | null;
  title?: string | null;
  body: string;
  tags_json: string;
  created_at: string;
  updated_at: string;
  stale: boolean;
}

export interface ScoredMemoryRecord extends MemoryRecord {
  score: number;
}

export interface MessageRecord {
  id: string;
  from_agent: string;
  to_agent: string;
  kind: string;
  status: string;
  subject?: string | null;
  body: string;
  created_at: string;
}

export interface WakeRecord {
  id: string;
  target_agent: string;
  reason?: string | null;
  status: string;
  requires_human_gate: boolean;
  created_at: string;
}

export interface AgentRecord {
  id: string;
  display_name: string;
  avatar_attachment_id?: string | null;
}

export interface AuditEvent {
  id: string;
  root_path: string;
  path: string;
  operation: string;
  observed_at: string;
  process_json: string;
  content_hash?: string | null;
  previous_hash?: string | null;
  event_hash: string;
  status: string;
}

export interface BudgetStatus {
  agent_id: string;
  limit_units: number;
  spent_units: number;
  paused: boolean;
  updated_at: string;
}

export interface ProviderQuotaWindow {
  label: string;
  family?: string | null;
  used_percent: number;
  remaining_percent: number;
  resets_at?: number | null;
  window_minutes?: number | null;
}

export interface ProviderQuotaBalance {
  currency: string;
  total: number;
  granted?: number | null;
  topped_up?: number | null;
  paid?: number | null;
  gift?: number | null;
}

// Raw counts a harness recorded locally (Mistral Vibe's per-session
// `meta.json` stats today). Neither a percentage nor a currency — there is no
// cap to divide by — so it is its own field rather than being forced into
// `windows` or `balance_info`.
export interface ProviderQuotaLocalUsage {
  sessions: number;
  prompt_tokens: number;
  completion_tokens: number;
  cached_tokens: number;
  tool_calls_succeeded: number;
  tool_calls_failed: number;
  tool_calls_rejected: number;
  /** UNIX seconds of the oldest session counted. */
  since?: number | null;
}

export interface ProviderQuota {
  agent_id: string;
  provider: string;
  harness_title?: string;
  status: string;
  detail?: string | null;
  windows: ProviderQuotaWindow[];
  fetched_at: number;
  balance?: string | null;
  balance_info?: ProviderQuotaBalance | null;
  local_usage?: ProviderQuotaLocalUsage | null;
}

export type HubTab = "dashboard" | "memory" | "inbox" | "wakes" | "tasks" | "usage" | "journal" | "channels";

// Mirrors `hub::ChannelWorkspace` (C14.3 / #150).
export interface ChannelWorkspace {
  workspace: string;
  display_name: string;
}

export interface ActiveGrokSession {
  session_id: string;
  pid: number;
  cwd: string;
  opened_at?: string | null;
}

export interface GrokConnectResult {
  leader_socket: string;
  leader_live: boolean;
  started_leader: boolean;
  started_terminal: boolean;
  session_id?: string | null;
  live_standalone?: ActiveGrokSession | null;
  detail: string;
}
