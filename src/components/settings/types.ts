import type { AuditEvent } from "../panels/hub/types";

// Mirrors `hub::FieldStatus` (Settings S2 / #128).
export type SettingsFieldStatus = "inherited" | "override";

// Mirrors `hub::SettingsField`.
export type SettingsField = "backup_retention" | "default_workspace" | "default_session";

// Mirrors `hub::EffectiveSettings`. Global defaults merged with an optional
// workspace override; never carries a filesystem path or a secret value.
// `profiles`/`harnesses` are Settings S4 (#130) scope — not read by the S3
// General/Workspace & sessions tabs, kept loosely typed here on purpose so
// this file doesn't need to track that in-flight shape.
export interface EffectiveSettings {
  schema_version: number;
  workspace: string | null;
  backup_retention: number;
  backup_retention_status: SettingsFieldStatus;
  default_workspace: string | null;
  default_workspace_status: SettingsFieldStatus;
  default_session: string | null;
  default_session_status: SettingsFieldStatus;
  profiles?: unknown[];
  harnesses?: unknown[];
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
