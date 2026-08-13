import { invoke } from "../../lib/tauri";
import type {
  EffectiveHarnessSettings,
  EffectiveSettings,
  HarnessSettings,
  ProfileSnapshot,
  ProviderProfile,
  SettingsAuditEvent,
  SettingsField,
  SettingsLoadStatus,
  SettingsPatch,
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
