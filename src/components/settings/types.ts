import type { AuditEvent } from "../panels/hub/types";

// Mirrors `hub::FieldStatus` (Settings S2 / #128).
export type SettingsFieldStatus = "inherited" | "override";

// Mirrors `hub::SettingsField`. One variant today; later slices add more
// without changing the patch/reset shape below.
export type SettingsField = "backup_retention";

// Mirrors `hub::EffectiveSettings`. Global defaults merged with an optional
// workspace override; never carries a filesystem path or a secret value.
export interface EffectiveSettings {
  schema_version: number;
  workspace: string | null;
  backup_retention: number;
  backup_retention_status: SettingsFieldStatus;
}

// Partial update sent to `settings_update`. `undefined` fields are left
// untouched.
export interface SettingsPatch {
  backup_retention?: number;
}

// Mirrors the Rust `SettingsLoadStatusDto` — the same shape as
// `hub::LoadStatus` with its file path stripped before crossing IPC.
export type SettingsLoadStatus =
  | { status: "missing" }
  | { status: "loaded" }
  | { status: "invalid"; reason: string }
  | { status: "unreadable"; reason: string };

// Redacted settings audit row. Same shape as the Hub's `AuditEvent`; the
// dedicated settings stream is a `root_path === "settings"` filter over it.
export type SettingsAuditEvent = AuditEvent;
