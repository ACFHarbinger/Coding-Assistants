import { invoke } from "../../../lib/tauri";

export interface ScheduleTriggerStatus {
  available: boolean;
  installed: boolean;
  enabled: boolean;
  active: boolean;
  service_path: string;
  timer_path: string;
  exec_start: string | null;
  detail: string | null;
}

export function getScheduleTriggerStatus(): Promise<ScheduleTriggerStatus> {
  return invoke<ScheduleTriggerStatus>("settings_schedule_trigger_status");
}

export function installScheduleTrigger(args: {
  ca_bin: string | null;
  interval_minutes: number;
  workspace: string;
}): Promise<ScheduleTriggerStatus> {
  return invoke<ScheduleTriggerStatus>("settings_schedule_trigger_install", args);
}

export function removeScheduleTrigger(): Promise<ScheduleTriggerStatus> {
  return invoke<ScheduleTriggerStatus>("settings_schedule_trigger_remove");
}
