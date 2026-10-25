export type TimelineEventType =
  | "task_created"
  | "workflow_step"
  | "transition"
  | "message"
  | "wake"
  | "handoff"
  | "tool"
  | "file"
  | "provider_call"
  | "gate";

export type TimelineEventStatus =
  | "completed"
  | "in_progress"
  | "paused"
  | "failed"
  | "pending";

export interface TaskTimelineEvent {
  id: string;
  task_id: string;
  timestamp: string;
  event_type: TimelineEventType;
  actor: string;
  target?: string | null;
  summary: string;
  detail?: string | null;
  status: TimelineEventStatus;
  step_index?: number | null;
  metadata?: Record<string, unknown> | null;
}

export interface TaskRecord {
  id: string;
  title: string;
  status: string;
  updated_at: string;
  created_at?: string;
  current_step?: number;
  total_steps?: number;
}

export interface TaskTimelineView {
  task: TaskRecord;
  events: TaskTimelineEvent[];
  involved_agents: string[];
  duration_seconds?: number | null;
  status: string;
  failure_reason?: string | null;
  is_paused: boolean;
}

export interface AgentRecord {
  id: string;
  display_name: string;
  avatar_attachment_id?: string | null;
}

export interface TimelineFilter {
  agent?: string;
  eventType?: TimelineEventType | "all";
  status?: TimelineEventStatus | "all";
  query?: string;
}
