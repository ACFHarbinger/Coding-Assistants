//! Orchestration policy types (split from `model.rs` to stay under the
//! 500-LoC cap): global policy, its per-workspace override, and the
//! merged effective shape returned to the frontend.

use super::model::{
    FieldStatus, LinkSuggestionMode, SandboxStrictness, SettingsError, DEFAULT_MEMORY_RECALL_LIMIT,
    DEFAULT_QUOTA_AUTO_REFRESH_SECS, MAX_MEMORY_RECALL_LIMIT, MAX_QUOTA_AUTO_REFRESH_SECS,
    MIN_QUOTA_AUTO_REFRESH_SECS,
};
use serde::{Deserialize, Serialize};

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
    /// Whether a provider-usage snapshot may run a probe that costs the user
    /// tokens. A few harnesses expose no free usage surface — the only read
    /// is a model turn (`gemini` via `agy --print "/usage"`, `opencode` via
    /// `opencode run "/ogc-usage"`) or a minimal completion (`muse`). When
    /// this is off those three report "unavailable" instead of spending
    /// anything; every other provider reads a free endpoint regardless.
    /// Global-only: quota is an account-level concept, not per-workspace.
    pub allow_metered_quota_probes: bool,
    /// Whether the public A2A wire listener may start at all (P11b / #335).
    /// **Off by default** — nothing listens for external A2A clients until
    /// the owner explicitly enables it. Global-only: a network listener is
    /// process-wide, not per-workspace.
    pub a2a_enabled: bool,
    /// Whether the app refreshes provider-usage snapshots on a background
    /// timer (the Usage strip's poll). **Off by default** — a background
    /// timer spends tokens on the metered adapters with no user in the
    /// loop. When on, `quota_auto_refresh_interval_secs` sets the cadence.
    /// Global-only.
    pub quota_auto_refresh_enabled: bool,
    /// Seconds between background usage refreshes when
    /// `quota_auto_refresh_enabled` is on. Clamped to
    /// [`MIN_QUOTA_AUTO_REFRESH_SECS`, `MAX_QUOTA_AUTO_REFRESH_SECS`].
    pub quota_auto_refresh_interval_secs: u32,
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
            allow_metered_quota_probes: true,
            a2a_enabled: false,
            quota_auto_refresh_enabled: false,
            quota_auto_refresh_interval_secs: DEFAULT_QUOTA_AUTO_REFRESH_SECS,
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
        if !(MIN_QUOTA_AUTO_REFRESH_SECS..=MAX_QUOTA_AUTO_REFRESH_SECS)
            .contains(&self.quota_auto_refresh_interval_secs)
        {
            return Err(SettingsError::Invalid(format!(
                "orchestration.quota_auto_refresh_interval_secs must be within \
                 {MIN_QUOTA_AUTO_REFRESH_SECS}..={MAX_QUOTA_AUTO_REFRESH_SECS}"
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
    /// Global-only, so — unlike every other field here — it carries no
    /// paired `_status`: there is no workspace override to be "inherited"
    /// from or to "override".
    pub allow_metered_quota_probes: bool,
    /// Global-only (no `_status`), same rationale as
    /// `allow_metered_quota_probes`.
    pub a2a_enabled: bool,
    /// Global-only (no `_status`), same rationale as
    /// `allow_metered_quota_probes`.
    pub quota_auto_refresh_enabled: bool,
    /// Global-only (no `_status`).
    pub quota_auto_refresh_interval_secs: u32,
}
