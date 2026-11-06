/**
 * Tauri IPC API bindings for Scheduled Goals (S7/S8, #350/#353).
 */

import { invoke } from "../../../lib/tauri";
import type { ScheduleCreateArgs, ScheduledTask, TickSummary } from "./types";

export async function createScheduledTask(
  args: ScheduleCreateArgs,
  callerAgentId: string | null = null,
): Promise<ScheduledTask> {
  return invoke<ScheduledTask>("hub_schedule_create", {
    args,
    callerAgentId,
  });
}

export async function listScheduledTasks(): Promise<ScheduledTask[]> {
  return invoke<ScheduledTask[]>("hub_schedule_list");
}

export async function getScheduledTask(id: string): Promise<ScheduledTask | null> {
  return invoke<ScheduledTask | null>("hub_schedule_get", { id });
}

export async function cancelScheduledTask(id: string): Promise<ScheduledTask> {
  return invoke<ScheduledTask>("hub_schedule_cancel", { id });
}

export async function tickScheduledTasks(): Promise<TickSummary> {
  return invoke<TickSummary>("hub_schedule_tick");
}
