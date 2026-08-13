import type { AuditEvent } from "../panels/hub/types";

// Mirrors `hub::FieldStatus` (Settings S2 / #128).
export type SettingsFieldStatus = "inherited" | "override";

// Mirrors `hub::SettingsField`.
export type SettingsField = "backup_retention" | "default_workspace" | "default_session";

// Mirrors `hub::SecretSourceKind`. Never a credential value.
export type SecretSourceKind = "keychain" | "env_var" | "provider_login";

// Mirrors `hub::SecretReference`. Keychain ids and env-var *names* only.
export type SecretReference =
  | { kind: "keychain"; id: string }
  | { kind: "env_var"; name: string }
  | { kind: "provider_login" };

// Mirrors `hub::ProfileSnapshot`.
export interface ProfileSnapshot {
  name: string;
  provider: string;
  model: string | null;
  base_url: string | null;
  secret_source: SecretSourceKind;
  secret_badge: string;
}

// Mirrors `hub::ProviderProfile` for upsert. No secret value field exists.
export interface ProviderProfile {
  name: string;
  provider: string;
  model: string | null;
  base_url: string | null;
  secret: SecretReference;
}

// Mirrors `hub::HarnessSettings`.
export interface HarnessSettings {
  harness: string;
  executable: string;
  workdir: string | null;
  capture_polling: boolean;
  inject_permission: boolean;
}

// Mirrors `hub::EffectiveHarnessSettings`.
export interface EffectiveHarnessSettings {
  harness: string;
  executable: string;
  workdir: string | null;
  capture_polling: boolean;
  inject_permission: boolean;
  default_profile: string | null;
  default_profile_status: SettingsFieldStatus;
  default_profile_badge: string | null;
}

// Mirrors `hub::EffectiveSettings`. Global defaults merged with an optional
// workspace override; never carries a filesystem path or a secret value.
export interface EffectiveSettings {
  schema_version: number;
  workspace: string | null;
  backup_retention: number;
  backup_retention_status: SettingsFieldStatus;
  default_workspace: string | null;
  default_workspace_status: SettingsFieldStatus;
  default_session: string | null;
  default_session_status: SettingsFieldStatus;
  profiles: ProfileSnapshot[];
  harnesses: EffectiveHarnessSettings[];
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
