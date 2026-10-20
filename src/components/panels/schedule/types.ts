/**
 * TypeScript types for Scheduled Goals (Roadmap S1/S7/S8, #350/#353).
 *
 * Aligned with Rust types in `crates/hub/src/store/schedule/types.rs`.
 */

export type ScheduleTarget =
  | { kind: "agent"; agent_id: string }
  | { kind: "team" };

export type GateCheck =
  | "exit_code_zero"
  | { at_most: { max: number } };

export type GateCriterion =
  | { kind: "command"; program: string; args: string[]; check: GateCheck }
  | { kind: "max_lines_per_file"; max_lines: number; globs: string[] }
  | { kind: "min_tokens_written"; min_tokens: number };

export type GateCombinator = "all" | "any";

export interface BudgetPolicy {
  credit_allowance?: number | null;
  credit_spent: number;
  stop_on_quota_exhausted: boolean;
}

export type ScheduleStatus =
  | "scheduled"
  | "active"
  | "paused"
  | "completed"
  | "cancelled"
  | "failed";

export interface ScheduledTask {
  id: string;
  name: string;
  instruction: string;
  target: ScheduleTarget;
  workspace: string;
  fire_at: string;
  repeat_interval_secs?: number | null;
  response_timeout_secs: number;
  gate_criteria: GateCriterion[];
  gate_combinator: GateCombinator;
  budget: BudgetPolicy;
  max_attempts: number;
  deadline?: string | null;
  status: ScheduleStatus;
  created_by: string;
  created_at: string;
  updated_at: string;
  last_fired_at?: string | null;
  next_fire_at?: string | null;
  attempts: number;
  token_baseline?: Record<string, number>;
}

export interface ScheduleCreateArgs {
  name: string;
  instruction: string;
  target: ScheduleTarget;
  workspace: string;
  fire_at: string;
  repeat_interval_secs?: number | null;
  response_timeout_secs?: number;
  gate_criteria: GateCriterion[];
  gate_combinator: GateCombinator;
  budget: BudgetPolicy;
  max_attempts: number;
  deadline?: string | null;
}

export interface TickSummary {
  evaluated_at: string;
  tasks_checked: number;
  dispatched: number;
  completed: number;
  re_prompted: number;
  paused: number;
  failed: number;
}
