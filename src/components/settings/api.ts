import { invoke } from "../../lib/tauri";
import type {
  BudgetStatus,
  CreativeToolsStatus,
  EffectiveHarnessSettings,
  EffectiveSettings,
  ExternalMcpStatus,
  HarnessSettings,
  OrchestrationPatch,
  ProfileSnapshot,
  ProviderProfile,
  SettingsAuditEvent,
  SettingsField,
  SettingsLoadStatus,
  SettingsPatch,
  StandingPolicySnapshot,
} from "./types";

// Typed client for the Settings S2 (#128) Tauri commands. The Settings
// window (S3) wires these into UI state; this module only owns the IPC
// contract so it stays correct independent of that UI work.

export function getEffectiveSettings(workspace: string | null = null): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_get_effective", { workspace });
}

export function getSettingsLoadStatus(): Promise<SettingsLoadStatus> {
  return invoke<SettingsLoadStatus>("settings_get_load_status");
}

// `workspace: null` updates the global default; a workspace path sets a
// workspace-local override for that exact path (never symlink-resolved).
export function updateSettings(workspace: string | null, patch: SettingsPatch): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_update", { workspace, patch });
}

export function resetSettingsField(workspace: string, field: SettingsField): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_reset_field", { workspace, field });
}

// `default_workspace` is global-only; `workspace: null` clears it.
export function setDefaultWorkspace(workspace: string | null): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_set_default_workspace", { workspace });
}

// `workspace: null` sets the global default session; a workspace path sets
// that workspace's override. `session: null` clears the value at that scope.
export function setDefaultSession(
  workspace: string | null,
  session: string | null,
): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_set_default_session", { workspace, session });
}

export function listSettingsAuditEvents(): Promise<SettingsAuditEvent[]> {
  return invoke<SettingsAuditEvent[]>("settings_list_audit_events");
}

export function listSettingsProfiles(): Promise<ProfileSnapshot[]> {
  return invoke<ProfileSnapshot[]>("settings_list_profiles");
}

export function upsertSettingsProfile(profile: ProviderProfile): Promise<ProfileSnapshot[]> {
  return invoke<ProfileSnapshot[]>("settings_upsert_profile", { profile });
}

export function renameSettingsProfile(from: string, to: string): Promise<ProfileSnapshot[]> {
  return invoke<ProfileSnapshot[]>("settings_rename_profile", { from, to });
}

export function removeSettingsProfile(name: string): Promise<ProfileSnapshot[]> {
  return invoke<ProfileSnapshot[]>("settings_remove_profile", { name });
}

// Backing commands for the danger tab (S6 remainder / #132). Each purge is
// hard and irreversible; the tab gates every one behind typed target
// confirmation, and each resolves with the deleted row count for the
// success copy. Cancellation never invokes any of these.
export function purgeWorkspaceTranscript(workspace: string): Promise<number> {
  return invoke<number>("settings_purge_workspace_transcript", { workspace });
}

export function purgeWorkspaceMemories(workspace: string): Promise<number> {
  return invoke<number>("settings_purge_workspace_memories", { workspace });
}

export interface WorkspacePurgeReport {
  messages: number;
  memories: number;
}

export function purgeWorkspaceData(workspace: string): Promise<WorkspacePurgeReport> {
  return invoke<WorkspacePurgeReport>("settings_purge_workspace_data", { workspace });
}

export function setWorkspaceDefaultProfile(
  workspace: string,
  harness: string,
  profile: string,
): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_set_workspace_default_profile", {
    workspace,
    harness,
    profile,
  });
}

export function resetWorkspaceDefaultProfile(
  workspace: string,
  harness: string,
): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_reset_workspace_default_profile", {
    workspace,
    harness,
  });
}

export function listSettingsHarnesses(workspace: string | null = null): Promise<EffectiveHarnessSettings[]> {
  return invoke<EffectiveHarnessSettings[]>("settings_list_harnesses", { workspace });
}

export function updateSettingsHarness(settings: HarnessSettings): Promise<HarnessSettings> {
  return invoke<HarnessSettings>("settings_update_harness", { settings });
}

// `workspace: null` updates the global default; a workspace path sets that
// workspace's override. `retention_days` is intentionally excluded from
// this patch — see `setRetentionDays`, which needs three-state
// (untouched/set/cleared) semantics this can't express.
export function updateOrchestrationPolicy(
  workspace: string | null,
  patch: OrchestrationPatch,
): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_update_orchestration", { workspace, patch });
}

// `workspace: null` sets the global retention window (`days: null` keeps
// records indefinitely). A workspace override always names a concrete day
// count; clear one with `resetSettingsField` instead of `days: null`.
export function setRetentionDays(workspace: string | null, days: number | null): Promise<EffectiveSettings> {
  return invoke<EffectiveSettings>("settings_set_retention_days", { workspace, days });
}

export function getStandingPolicy(workspace: string | null = null): Promise<StandingPolicySnapshot> {
  return invoke<StandingPolicySnapshot>("settings_get_standing_policy", { workspace });
}

// Global only: the wake human-gate is not a per-workspace concept today.
export function setConfirmWakes(value: boolean): Promise<StandingPolicySnapshot> {
  return invoke<StandingPolicySnapshot>("settings_set_confirm_wakes", { value });
}

// Global only, same as `setConfirmWakes`. When false, any wake attempting
// to bypass the human gate is rejected outright.
export function setAllowAutoWake(value: boolean): Promise<StandingPolicySnapshot> {
  return invoke<StandingPolicySnapshot>("settings_set_allow_auto_wake", { value });
}

export function listAgentBudgets(): Promise<BudgetStatus[]> {
  return invoke<BudgetStatus[]>("settings_list_agent_budgets");
}

export function setAgentBudget(agentId: string, limitUnits: number): Promise<BudgetStatus> {
  return invoke<BudgetStatus>("settings_set_agent_budget", { agentId, limitUnits });
}

export function getCreativeToolsStatus(workspace: string): Promise<CreativeToolsStatus> {
  return invoke<CreativeToolsStatus>("creative_tools_status", { workspace });
}

export function setCreativeToolEnabled(
  workspace: string,
  key: string,
  enabled: boolean,
): Promise<CreativeToolsStatus> {
  return invoke<CreativeToolsStatus>("creative_tools_set_enabled", { workspace, key, enabled });
}

export function reapplyCreativeTools(workspace: string): Promise<CreativeToolsStatus> {
  return invoke<CreativeToolsStatus>("creative_tools_reapply", { workspace });
}

export function getCreativeToolsCodexSnippet(workspace: string): Promise<string> {
  return invoke<string>("creative_tools_codex_snippet", { workspace });
}

export function getExternalMcpStatus(workspace: string): Promise<ExternalMcpStatus> {
  return invoke<ExternalMcpStatus>("external_mcp_status", { workspace });
}

export function setExternalMcpEnabled(
  workspace: string,
  key: string,
  enabled: boolean,
): Promise<ExternalMcpStatus> {
  return invoke<ExternalMcpStatus>("external_mcp_set_enabled", { workspace, key, enabled });
}

export function reapplyExternalMcp(workspace: string): Promise<ExternalMcpStatus> {
  return invoke<ExternalMcpStatus>("external_mcp_reapply", { workspace });
}

export function getHarnessModelOptions(
  harness: string,
  refresh: boolean = false,
): Promise<import("./types").HarnessModelCatalog> {
  return invoke<import("./types").HarnessModelCatalog>("settings_get_harness_model_options", {
    harness,
    refresh,
  });
}

export function getAllHarnessOptions(
  refresh: boolean = false,
): Promise<Record<string, import("./types").HarnessModelCatalog>> {
  return invoke<Record<string, import("./types").HarnessModelCatalog>>(
    "settings_get_all_harness_options",
    { refresh },
  );
}

export function setHarnessModel(
  harness: string,
  model: string | null,
): Promise<EffectiveHarnessSettings> {
  return invoke<EffectiveHarnessSettings>("settings_set_harness_model", { harness, model });
}

export function setHarnessEffort(
  harness: string,
  effort: string | null,
): Promise<EffectiveHarnessSettings> {
  return invoke<EffectiveHarnessSettings>("settings_set_harness_effort", { harness, effort });
}

export function setWorkspaceHarnessModel(
  workspace: string,
  harness: string,
  model: string,
): Promise<EffectiveHarnessSettings> {
  return invoke<EffectiveHarnessSettings>("settings_set_workspace_harness_model", {
    workspace,
    harness,
    model,
  });
}

export function resetWorkspaceHarnessModel(
  workspace: string,
  harness: string,
): Promise<EffectiveHarnessSettings> {
  return invoke<EffectiveHarnessSettings>("settings_reset_workspace_harness_model", {
    workspace,
    harness,
  });
}

export function setWorkspaceHarnessEffort(
  workspace: string,
  harness: string,
  effort: string,
): Promise<EffectiveHarnessSettings> {
  return invoke<EffectiveHarnessSettings>("settings_set_workspace_harness_effort", {
    workspace,
    harness,
    effort,
  });
}

export function resetWorkspaceHarnessEffort(
  workspace: string,
  harness: string,
): Promise<EffectiveHarnessSettings> {
  return invoke<EffectiveHarnessSettings>("settings_reset_workspace_harness_effort", {
    workspace,
    harness,
  });
}

// ─── Credential commands (#283 / #284) ───────────────────────────────────────
// None of these functions return or accept a stored secret value. `setCredential`
// accepts the value write-only (it is dropped in Rust after storage); every
// response carries only SecretStatus (presence, source, last-updated).

import type { FieldSpec, LinkedAccountStatus, SecretStatus } from "./types";

/** Fetch the static catalog of all known credential/config fields. */
export function listCredentialFields(): Promise<FieldSpec[]> {
  return invoke<FieldSpec[]>("settings_list_credential_fields");
}

/**
 * Store or replace a credential write-only. `value` is accepted and
 * immediately forwarded to the vault; it never leaves the Rust layer.
 * Returns only the non-secret SecretStatus.
 */
export function setCredential(fieldId: string, value: string): Promise<SecretStatus> {
  return invoke<SecretStatus>("settings_set_credential", { fieldId, value });
}

/**
 * Remove the stored vault entry for a field. After clearing, the resolver
 * falls back to the matching environment variable (if any). Idempotent.
 */
export function clearCredential(fieldId: string): Promise<SecretStatus> {
  return invoke<SecretStatus>("settings_clear_credential", { fieldId });
}

/** Return the non-secret status of a credential: presence, source, last-updated. */
export function getCredentialStatus(fieldId: string): Promise<SecretStatus> {
  return invoke<SecretStatus>("settings_get_credential_status", { fieldId });
}

// ─── Linked external accounts (#284 / #286) ─────────────────────────────────

/**
 * List all linked external provider accounts (#286), scoped to owner = "local".
 * Returns non-secret presence and connection metadata for standard providers
 * (ChatGPT, Claude, Google, etc.).
 */
export function listLinkedAccounts(): Promise<LinkedAccountStatus[]> {
  return invoke<LinkedAccountStatus[]>("hub_list_linked_accounts");
}

/**
 * Record an external provider account that uses the vendor's native CLI login
 * (#286), backed by single-user storage under provisional "local" key.
 */
export function linkAccountCli(provider: string, externalLabel?: string | null): Promise<LinkedAccountStatus> {
  return invoke<LinkedAccountStatus>("hub_link_account_cli", { provider, externalLabel: externalLabel ?? null });
}

/**
 * Disconnect / unlink an external provider account (#286).
 * Deletes the vault entry if present and removes the row from hub.db.
 */
export function unlinkAccount(provider: string): Promise<boolean> {
  return invoke<boolean>("hub_unlink_account", { provider });
}
