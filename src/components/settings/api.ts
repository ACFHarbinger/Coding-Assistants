import { invoke } from "../../lib/tauri";
import type {
  BudgetStatus,
  CreativeToolsStatus,
  EffectiveHarnessSettings,
  EffectiveSettings,
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
