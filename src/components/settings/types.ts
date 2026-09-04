import type { AuditEvent } from "../panels/hub/types";

// Mirrors `hub::FieldStatus` (Settings S2 / #128).
export type SettingsFieldStatus = "inherited" | "override";

// Mirrors `hub::SettingsField`.
export type SettingsField =
  | "backup_retention"
  | "default_workspace"
  | "default_session"
  | "confirm_new_enrollment"
  | "confirm_broadcast"
  | "auto_enrollment_allowed"
  | "sandbox_strictness"
  | "retention_days"
  | "export_enabled"
  | "link_suggestion_mode"
  | "memory_recall_enabled"
  | "memory_recall_limit";

// Mirrors `hub::SandboxStrictness`.
export type SandboxStrictness = "strict" | "standard" | "permissive";

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
  default_model?: string | null;
  default_effort?: string | null;
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
  selected_model?: string | null;
  selected_model_status?: SettingsFieldStatus;
  selected_effort?: string | null;
  selected_effort_status?: SettingsFieldStatus;
}

export interface HarnessModelCatalog {
  harness: string;
  models: string[];
  effort_options: string[];
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
  orchestration: EffectiveOrchestrationPolicy;
}

// Mirrors `hub::EffectiveOrchestrationPolicy` (Settings S5 / #131).
export interface EffectiveOrchestrationPolicy {
  confirm_new_enrollment: boolean;
  confirm_new_enrollment_status: SettingsFieldStatus;
  confirm_broadcast: boolean;
  confirm_broadcast_status: SettingsFieldStatus;
  auto_enrollment_allowed: boolean;
  auto_enrollment_allowed_status: SettingsFieldStatus;
  sandbox_strictness: SandboxStrictness;
  sandbox_strictness_status: SettingsFieldStatus;
  retention_days: number | null;
  retention_days_status: SettingsFieldStatus;
  export_enabled: boolean;
  export_enabled_status: SettingsFieldStatus;
  memory_recall_enabled: boolean;
  memory_recall_enabled_status: SettingsFieldStatus;
  memory_recall_limit: number;
  memory_recall_limit_status: SettingsFieldStatus;
}

// Partial update sent to `settings_update_orchestration`. `retention_days`
// is excluded there — see `setRetentionDays` in `api.ts`.
export interface OrchestrationPatch {
  confirm_new_enrollment?: boolean;
  confirm_broadcast?: boolean;
  auto_enrollment_allowed?: boolean;
  sandbox_strictness?: SandboxStrictness;
  export_enabled?: boolean;
  memory_recall_enabled?: boolean;
  memory_recall_limit?: number;
}

// Composes the orchestration policy above with the Hub's existing
// WakePolicy human-gate bit, so Settings reads/writes standing policy as
// one view even though wake-gate storage stays in HubStore.
export interface StandingPolicySnapshot {
  confirm_wakes: boolean;
  allow_auto_wake: boolean;
  orchestration: EffectiveOrchestrationPolicy;
}

// Mirrors `hub::BudgetStatus`.
export interface BudgetStatus {
  agent_id: string;
  limit_units: number;
  spent_units: number;
  paused: boolean;
  updated_at: string;
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

// Mirrors Rust `CreativeToolStatus`.
export interface CreativeToolStatus {
  key: string;
  displayName: string;
  transport: "socket" | "subprocess" | "file-parse" | string;
  port: number | null;
  gatedFlag: string | null;
  binaryFound: boolean;
  binaryPath: string | null;
  appRunning: boolean;
  enabled: boolean;
}

// Mirrors Rust `CreativeToolsStatus`.
export interface CreativeToolsStatus {
  workspace: string;
  tools: CreativeToolStatus[];
  writtenConfigs: string[];
}
