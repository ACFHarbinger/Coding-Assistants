import { invoke } from "../../lib/tauri";
import type {
  EffectiveSettings,
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
