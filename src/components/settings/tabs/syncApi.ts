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

export type ConflictChoice = "local" | "remote" | "keep-both" | "manual";

export interface ConflictDecision {
  choice: ConflictChoice;
  decided_at: string;
  local_hash: string | null;
  remote_hash: string | null;
  base_hash: string | null;
}

export interface ConflictItem {
  slug: string;
  path: string;
  reason: string;
  decision: ConflictDecision | null;
  local_hash: string | null;
  remote_hash: string | null;
  base_hash: string | null;
}

export function hubSyncConflicts(): Promise<ConflictItem[]> {
  return invoke<ConflictItem[]>("hub_sync_conflicts");
}

export function hubSyncResolve(slug: string, choice: ConflictChoice): Promise<ConflictDecision> {
  return invoke<ConflictDecision>("hub_sync_resolve", { slug, choice });
}

export interface Tombstone {
  slug: string;
  path: string;
  created_at: string;
  expires_at: string;
  content_hash: string;
  policy: string;
}

export interface CleanupCandidate {
  kind: string;
  slug: string;
  aged_at: string;
}

export interface PurgeReport {
  purged: string[];
}

export function hubSyncTombstones(): Promise<Tombstone[]> {
  return invoke<Tombstone[]>("hub_sync_tombstones");
}

export function hubSyncExpired(): Promise<CleanupCandidate[]> {
  return invoke<CleanupCandidate[]>("hub_sync_expired");
}

export function hubSyncPurgeExpired(confirm: boolean): Promise<PurgeReport> {
  return invoke<PurgeReport>("hub_sync_purge_expired", { confirm });
}

export type TrustStatus = "trusted" | "revoked";

export interface TrustEntry {
  id: string;
  folder: string;
  status: TrustStatus;
  registered_at: string;
  revoked_at: string | null;
}

export interface TrustList {
  schema: number;
  devices: TrustEntry[];
}

export interface SyncLimits {
  max_objects: number;
  max_bytes: number;
  max_concurrent: number;
}

export interface HistoryEntry {
  at: string;
  action: string;
  uploaded: number;
  downloaded: number;
  conflicts: number;
  pruned: number;
  warnings: string[];
  ok: boolean;
}

export interface TrustExport {
  folder: string;
  status: string;
}

export interface Diagnostics {
  schema: number;
  last_verified_base: string | null;
  trust: TrustExport[];
  limits: SyncLimits;
  resume_action: string | null;
  history: HistoryEntry[];
}

export function hubSyncDevices(): Promise<TrustList> {
  return invoke<TrustList>("hub_sync_devices");
}

export function hubSyncRegister(id: string): Promise<TrustEntry> {
  return invoke<TrustEntry>("hub_sync_register", { id });
}

export function hubSyncRevoke(id: string): Promise<TrustEntry> {
  return invoke<TrustEntry>("hub_sync_revoke", { id });
}

export function hubSyncTrust(id: string): Promise<TrustEntry> {
  return invoke<TrustEntry>("hub_sync_trust", { id });
}

export function hubSyncRetry(): Promise<SyncSession> {
  return invoke<SyncSession>("hub_sync_retry");
}

export function hubSyncHistory(): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("hub_sync_history");
}

export function hubSyncDiagnostics(): Promise<Diagnostics> {
  return invoke<Diagnostics>("hub_sync_diagnostics");
}

export function hubSyncLimits(): Promise<SyncLimits> {
  return invoke<SyncLimits>("hub_sync_limits");
}
