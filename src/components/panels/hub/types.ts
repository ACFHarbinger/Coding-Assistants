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
  kind?: "balance" | "spend" | null;
  spent?: number | null;
  granted?: number | null;
  topped_up?: number | null;
  paid?: number | null;
  gift?: number | null;
}

// Raw counts a harness recorded locally (Mistral Vibe `meta.json` stats,
// Qwen Code `usage_record.jsonl`). Neither a percentage nor a currency — there is no
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

export type HubTab = "dashboard" | "memory" | "inbox" | "wakes" | "tasks" | "usage" | "journal" | "scorecard" | "channels" | "tools" | "board" | "branches" | "system";

export type FigureProvenance = "measured" | "estimated" | "unknown";
export type HarnessCoverage = "managed" | "observed" | "unregistered";

export interface ScorecardFigure {
  coverage?: { measured_requests: number; total_requests: number } | null;
  value: number | null;
  provenance: FigureProvenance;
  unit: string;
}

export interface ScorecardSecurityEvent {
  id: string;
  operation: string;
  observed_at: string;
  status: string;
  task_id?: string | null;
  work_session_id?: string | null;
  provenance: FigureProvenance;
}

export interface AgentScorecardRow {
  agent_id: string;
  display_name: string;
  oversight_role?: string | null;
  open_security_events: ScorecardSecurityEvent[];
  oldest_pending_human_decision?: string | null;
  tokens: ScorecardFigure;
  cost: ScorecardFigure;
  measured_cost_units?: number | null;
  estimated_cost_units?: number | null;
  budget_spent?: number | null;
  budget_limit?: number | null;
  budget_paused: boolean;
  provenance: string;
  task_count: ScorecardFigure;
  /** Unresolved `scorecard.disagreement` rows naming this agent (T7 / #347). */
  unresolved_disagreements?: number;
}

export interface HarnessScorecardRow {
  harness: string;
  workspace: string;
  coverage: HarnessCoverage;
  linked_agent_id?: string | null;
  disk_session_id?: string | null;
  state?: string | null;
  session_count: ScorecardFigure;
}

export interface ProviderScorecardRow {
  provider: string;
  via_gateway?: string | null;
  tokens: ScorecardFigure;
  cost: ScorecardFigure;
  request_count: ScorecardFigure;
}

export interface ScorecardView {
  agents: AgentScorecardRow[];
  harnesses: HarnessScorecardRow[];
  providers: ProviderScorecardRow[];
  aggregation_version: number;
  generated_at: string;
  /** Advisory consistency probes (T7 / #347). Owner-only payload. */
  probes?: ScorecardProbe[];
  /** Reviewer-side oversight signature (T7 / #347). Owner-private. */
  reviewers?: ReviewerSignatureRow[];
}

export type ProbeState = "ok" | "disagreement" | "unknown";

export interface ScorecardProbe {
  probe: string;
  subject: string;
  state: ProbeState;
  reason?: string | null;
  source_event_ids: string[];
  linked_agents?: string[];
}

export interface ReviewerSignatureRow {
  reviewer_id: string;
  display_name: string;
  findings_recorded: ScorecardFigure;
  findings_corrected_by_owner: ScorecardFigure;
  /** Findings per task WITH a documented finding — not a rate across all reviewed tasks. */
  findings_per_task: ScorecardFigure;
  owner_override_reversal_rate: ScorecardFigure;
  /** Always `unknown` until gate decisions record reviewer identity and evidence signals exist. */
  zero_evidence_approvals: ScorecardFigure;
  review_dwell: ScorecardFigure;
}

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
