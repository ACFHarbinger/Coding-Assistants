use super::TuiSettings;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid settings: {0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
}

/// Current on-disk schema. Unknown or missing versions fail validation.
pub const CURRENT_SETTINGS_SCHEMA: u32 = 1;
/// Default number of timestamped last-known-good backups.
pub const DEFAULT_BACKUP_RETENTION: u32 = 3;
/// Inclusive lower bound for `storage.backup_retention`.
pub const MIN_BACKUP_RETENTION: u32 = 1;
/// Inclusive upper bound for `storage.backup_retention`.
pub const MAX_BACKUP_RETENTION: u32 = 20;
/// Default number of recalled memories injected into an agent prompt.
pub const DEFAULT_MEMORY_RECALL_LIMIT: u8 = 5;
/// A small ceiling keeps recalled context useful without crowding out the task.
pub const MAX_MEMORY_RECALL_LIMIT: u8 = 20;

/// Backend used to create memory-search embeddings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProvider {
    #[default]
    Local,
    Openai,
}

impl EmbeddingProvider {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "local" => Some(Self::Local),
            "openai" => Some(Self::Openai),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Openai => "openai",
        }
    }
}

/// Validated settings fields owned by S1. Later slices add more keys without
/// changing this load/save contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub schema_version: u32,
    pub backup_retention: u32,
    pub default_workspace: Option<String>,
    pub default_session: Option<String>,
    pub embedding_provider: EmbeddingProvider,
    pub orchestration: OrchestrationPolicy,
    pub tui: TuiSettings,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA,
            backup_retention: DEFAULT_BACKUP_RETENTION,
            default_workspace: None,
            default_session: None,
            embedding_provider: EmbeddingProvider::Local,
            orchestration: OrchestrationPolicy::default(),
            tui: TuiSettings::default(),
        }
    }
}

impl SettingsSnapshot {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.schema_version != CURRENT_SETTINGS_SCHEMA {
            return Err(SettingsError::Invalid(format!(
                "unsupported schema_version {} (expected {CURRENT_SETTINGS_SCHEMA})",
                self.schema_version
            )));
        }
        if !(MIN_BACKUP_RETENTION..=MAX_BACKUP_RETENTION).contains(&self.backup_retention) {
            return Err(SettingsError::Invalid(format!(
                "storage.backup_retention {} is outside {MIN_BACKUP_RETENTION}..={MAX_BACKUP_RETENTION}",
                self.backup_retention
            )));
        }
        self.orchestration.validate()?;
        Ok(())
    }
}

/// Whether an effective field came from the global default or a workspace
/// override (S2 / #128 scope resolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldStatus {
    Inherited,
    Override,
}

/// Fields a workspace override may set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsField {
    BackupRetention,
    DefaultWorkspace,
    DefaultSession,
    ConfirmNewEnrollment,
    ConfirmBroadcast,
    AutoEnrollmentAllowed,
    SandboxStrictness,
    RetentionDays,
    ExportEnabled,
    LinkSuggestionMode,
    MemoryRecallEnabled,
    MemoryRecallLimit,
}

/// Per-workspace overrides. Only fields present here differ from the global
/// default; an absent field means "inherited". The workspace identity is the
/// user-selected path string, kept exactly as given (not symlink-resolved),
/// so distinct paths to the same repository can carry separate overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOverride {
    pub backup_retention: Option<u32>,
    pub default_session: Option<String>,
    /// Harness id → global profile name. The workspace never copies profile
    /// fields; it only selects a named default.
    #[serde(default)]
    pub default_profiles: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub default_models: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub default_efforts: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub orchestration: OrchestrationOverride,
}

impl WorkspaceOverride {
    pub fn is_empty(&self) -> bool {
        self.backup_retention.is_none()
            && self.default_session.is_none()
            && self.default_profiles.is_empty()
            && self.default_models.is_empty()
            && self.default_efforts.is_empty()
            && self.orchestration.is_empty()
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        if let Some(retention) = self.backup_retention {
            if !(MIN_BACKUP_RETENTION..=MAX_BACKUP_RETENTION).contains(&retention) {
                return Err(SettingsError::Invalid(format!(
                    "storage.backup_retention {retention} is outside {MIN_BACKUP_RETENTION}..={MAX_BACKUP_RETENTION}"
                )));
            }
        }
        self.orchestration.validate()?;
        Ok(())
    }
}

/// Sandbox strictness for tool execution. A coarse, ordinary-tier control;
/// per-tool allow/deny lists are Advanced-tier future work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStrictness {
    Strict,
    #[default]
    Standard,
    Permissive,
}

impl SandboxStrictness {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Standard => "standard",
            Self::Permissive => "permissive",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "strict" => Some(Self::Strict),
            "standard" => Some(Self::Standard),
            "permissive" => Some(Self::Permissive),
            _ => None,
        }
    }
}

/// Whether newly written memories get candidate links to related existing
/// memories proposed automatically (M-links). `Off` means links are only
/// ever created by an explicit `link_memories` call — no proposer runs.
/// `Suggest` surfaces candidates for a human/agent to confirm before an edge
/// is written. `Auto` writes edges above a similarity/match threshold
/// immediately, attributed to `created_by = "system:auto-link"` rather than
/// whichever agent's memory triggered the suggestion, so provenance still
/// distinguishes a drawn connection from a computed one even in this mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LinkSuggestionMode {
    #[default]
    Off,
    Suggest,
    Auto,
}

impl LinkSuggestionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Suggest => "suggest",
            Self::Auto => "auto",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "suggest" => Some(Self::Suggest),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// Standing orchestration policy (S5 / #131), owned by Settings. Wake
/// human-gate approval (`allow_auto_wake` / `default_requires_human_gate`)
/// deliberately stays in `HubStore`'s existing `WakePolicy` — every
/// C10-C13 wake path already reads it — rather than being duplicated here;
/// Settings composes both into one typed command surface (see
/// `src-tauri/src/hub/commands/settings.rs`) so it remains the sole editor
/// without a risky storage migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationPolicy {
    /// Confirm before enrolling a not-yet-team agent identity via a wake.
    pub confirm_new_enrollment: bool,
    /// Confirm before a broadcast (all/team) send.
    pub confirm_broadcast: bool,
    /// Whether a wake may auto-enroll any supported harness identity at all.
    pub auto_enrollment_allowed: bool,
    pub sandbox_strictness: SandboxStrictness,
    /// Transcript/memory retention in days. `None` keeps records indefinitely.
    pub retention_days: Option<u32>,
    /// Whether non-destructive export actions are available.
    pub export_enabled: bool,
    /// Whether writing a new memory proposes/creates candidate links to
    /// related existing memories (M-links). See [`LinkSuggestionMode`].
    pub link_suggestion_mode: LinkSuggestionMode,
    /// Inject relevant workspace/global memories into orchestrated prompts.
    pub memory_recall_enabled: bool,
    /// Maximum number of memories injected for one prompt.
    pub memory_recall_limit: u8,
}

impl Default for OrchestrationPolicy {
    fn default() -> Self {
        Self {
            confirm_new_enrollment: true,
            confirm_broadcast: true,
            auto_enrollment_allowed: true,
            sandbox_strictness: SandboxStrictness::Standard,
            retention_days: None,
            export_enabled: true,
            link_suggestion_mode: LinkSuggestionMode::Off,
            memory_recall_enabled: true,
            memory_recall_limit: DEFAULT_MEMORY_RECALL_LIMIT,
        }
    }
}

impl OrchestrationPolicy {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.retention_days == Some(0) {
            return Err(SettingsError::Invalid(
                "orchestration.retention_days must be greater than 0 when set".into(),
            ));
        }
        if !(1..=MAX_MEMORY_RECALL_LIMIT).contains(&self.memory_recall_limit) {
            return Err(SettingsError::Invalid(format!(
                "orchestration.memory_recall_limit must be within 1..={MAX_MEMORY_RECALL_LIMIT}"
            )));
        }
        Ok(())
    }
}

/// Per-workspace override of [`OrchestrationPolicy`]. Same "absent field
/// means inherited" contract as [`WorkspaceOverride`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationOverride {
    pub confirm_new_enrollment: Option<bool>,
    pub confirm_broadcast: Option<bool>,
    pub auto_enrollment_allowed: Option<bool>,
    pub sandbox_strictness: Option<SandboxStrictness>,
    pub retention_days: Option<u32>,
    pub export_enabled: Option<bool>,
    pub link_suggestion_mode: Option<LinkSuggestionMode>,
    pub memory_recall_enabled: Option<bool>,
    pub memory_recall_limit: Option<u8>,
}

impl OrchestrationOverride {
    pub fn is_empty(&self) -> bool {
        self.confirm_new_enrollment.is_none()
            && self.confirm_broadcast.is_none()
            && self.link_suggestion_mode.is_none()
            && self.memory_recall_enabled.is_none()
            && self.memory_recall_limit.is_none()
            && self.auto_enrollment_allowed.is_none()
            && self.sandbox_strictness.is_none()
            && self.retention_days.is_none()
            && self.export_enabled.is_none()
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.retention_days == Some(0) {
            return Err(SettingsError::Invalid(
                "orchestration.retention_days must be greater than 0 when set".into(),
            ));
        }
        if self
            .memory_recall_limit
            .is_some_and(|limit| !(1..=MAX_MEMORY_RECALL_LIMIT).contains(&limit))
        {
            return Err(SettingsError::Invalid(format!(
                "orchestration.memory_recall_limit must be within 1..={MAX_MEMORY_RECALL_LIMIT}"
            )));
        }
        Ok(())
    }
}

/// [`OrchestrationPolicy`] merged with an optional workspace override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveOrchestrationPolicy {
    pub confirm_new_enrollment: bool,
    pub confirm_new_enrollment_status: FieldStatus,
    pub confirm_broadcast: bool,
    pub confirm_broadcast_status: FieldStatus,
    pub auto_enrollment_allowed: bool,
    pub auto_enrollment_allowed_status: FieldStatus,
    pub sandbox_strictness: SandboxStrictness,
    pub sandbox_strictness_status: FieldStatus,
    pub retention_days: Option<u32>,
    pub retention_days_status: FieldStatus,
    pub export_enabled: bool,
    pub export_enabled_status: FieldStatus,
    pub link_suggestion_mode: LinkSuggestionMode,
    pub link_suggestion_mode_status: FieldStatus,
    pub memory_recall_enabled: bool,
    pub memory_recall_enabled_status: FieldStatus,
    pub memory_recall_limit: u8,
    pub memory_recall_limit_status: FieldStatus,
}

/// Global defaults merged with an optional workspace override — the typed,
/// redacted shape returned to the frontend. Field-status pills let React
/// show "Inherited" vs "Workspace Override" without re-deriving the merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveSettings {
    pub schema_version: u32,
    pub workspace: Option<String>,
    pub backup_retention: u32,
    pub backup_retention_status: FieldStatus,
    pub default_workspace: Option<String>,
    pub default_workspace_status: FieldStatus,
    pub default_session: Option<String>,
    pub default_session_status: FieldStatus,
    #[serde(default)]
    pub profiles: Vec<ProfileSnapshot>,
    #[serde(default)]
    pub harnesses: Vec<EffectiveHarnessSettings>,
    pub orchestration: EffectiveOrchestrationPolicy,
    pub tui: TuiSettings,
}

/// How a profile obtains credentials. Never carries a secret value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretSourceKind {
    Keychain,
    EnvVar,
    ProviderLogin,
}

/// Stored secret *reference*. Compatible with a later OS keychain or
/// encrypted-vault backend; the settings file only keeps the kind plus an
/// opaque id or environment-variable name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SecretReference {
    Keychain { id: String },
    EnvVar { name: String },
    ProviderLogin,
}

impl SecretReference {
    pub fn kind(&self) -> SecretSourceKind {
        match self {
            Self::Keychain { .. } => SecretSourceKind::Keychain,
            Self::EnvVar { .. } => SecretSourceKind::EnvVar,
            Self::ProviderLogin => SecretSourceKind::ProviderLogin,
        }
    }

    /// Non-sensitive badge for Settings UI. Never includes a credential.
    pub fn badge(&self) -> String {
        match self {
            Self::Keychain { .. } => "Stored in System Keychain".into(),
            Self::EnvVar { name } => format!("Env Var ${name}"),
            Self::ProviderLogin => "Existing provider login".into(),
        }
    }
}

/// Global named provider profile. Fields are non-secret configuration plus
/// a secret reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub secret: SecretReference,
}

impl ProviderProfile {
    pub fn snapshot(&self) -> ProfileSnapshot {
        ProfileSnapshot {
            name: self.name.clone(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
            secret_source: self.secret.kind(),
            secret_badge: self.secret.badge(),
        }
    }
}

/// Redacted profile shown to clients. Env-var *names* are allowed; values
/// and keychain material are not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSnapshot {
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub secret_source: SecretSourceKind,
    pub secret_badge: String,
}

/// Global per-harness process settings. A workspace does not copy these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSettings {
    pub harness: String,
    pub executable: String,
    pub workdir: Option<String>,
    pub capture_polling: bool,
    pub inject_permission: bool,
    pub default_model: Option<String>,
    pub default_effort: Option<String>,
}

impl HarnessSettings {
    pub fn default_for(harness: &str) -> Result<Self, SettingsError> {
        let id = crate::HarnessId::parse(harness)
            .map_err(|err| SettingsError::Invalid(err.to_string()))?;
        let default_model = match id {
            crate::HarnessId::OpenCode => Some(crate::harness::DEFAULT_OPENCODE_MODEL.to_string()),
            crate::HarnessId::DeepSeek => Some(crate::harness::DEFAULT_DEEPSEEK_MODEL.to_string()),
            crate::HarnessId::Claude => Some("claude-3-7-sonnet-20250219".to_string()),
            crate::HarnessId::Chat => Some("gpt-4o".to_string()),
            crate::HarnessId::Gemini => Some("gemini-3.7-flash-medium".to_string()),
            crate::HarnessId::Grok => Some("grok-4.6".to_string()),
            crate::HarnessId::Vibe => Some("mistral-medium-3.5".to_string()),
        };
        let default_effort = match id {
            crate::HarnessId::Claude
            | crate::HarnessId::Chat
            | crate::HarnessId::Gemini
            | crate::HarnessId::Grok
            | crate::HarnessId::OpenCode
            | crate::HarnessId::DeepSeek => Some("medium".to_string()),
            crate::HarnessId::Vibe => None,
        };
        Ok(Self {
            harness: id.as_str().to_string(),
            executable: id.executable().to_string(),
            workdir: None,
            capture_polling: true,
            inject_permission: true,
            default_model,
            default_effort,
        })
    }
}

/// Effective harness view: global process settings plus the workspace's
/// selected default profile, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveHarnessSettings {
    pub harness: String,
    pub executable: String,
    pub workdir: Option<String>,
    pub capture_polling: bool,
    pub inject_permission: bool,
    pub default_profile: Option<String>,
    pub default_profile_status: FieldStatus,
    pub default_profile_badge: Option<String>,
    pub selected_model: Option<String>,
    pub selected_model_status: FieldStatus,
    pub selected_effort: Option<String>,
    pub selected_effort_status: FieldStatus,
}
