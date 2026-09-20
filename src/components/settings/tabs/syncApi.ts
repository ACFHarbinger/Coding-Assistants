import { invoke } from "../../../lib/tauri";

export interface SyncPlan {
  action: string;
  account_connected: boolean;
  provider: string;
  local_schema: string;
  replica_schema: string | null;
  schema_warning: string | null;
  last_verified_base: string | null;
  category_counts: Record<string, number>;
  lock_held: boolean;
  errors: string[];
}

export interface SyncResult {
  uploaded: number;
  downloaded: number;
  pruned: number;
  conflicts: number;
  warnings: string[];
}

export interface SyncSession {
  plan: SyncPlan;
  result: SyncResult;
}

export interface SyncLock {
  pid: number;
  started_at: string;
  action: string;
}

export interface SyncStatus {
  plan: SyncPlan;
  lock: SyncLock | null;
}

export function hubSyncPreview(): Promise<SyncPlan> {
  return invoke<SyncPlan>("hub_sync_preview");
}

export function hubSyncStart(action: string): Promise<SyncSession> {
  return invoke<SyncSession>("hub_sync_start", { action });
}

export function hubSyncStatus(): Promise<SyncStatus> {
  return invoke<SyncStatus>("hub_sync_status");
}

export function hubSyncCancel(): Promise<void> {
  return invoke<void>("hub_sync_cancel");
}
