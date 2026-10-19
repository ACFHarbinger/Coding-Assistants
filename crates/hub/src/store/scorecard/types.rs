//! Shared types for Team Scorecard (T3 / #343).
//!
//! Projects stamped audit rows, tasks, work sessions, budgets, and quota
//! into three row kinds (Agent, Harness, Provider) with per-figure provenance
//! (`measured` | `estimated` | `unknown`) and harness coverage chips
//! (`managed` | `observed` | `unregistered`).

use serde::{Deserialize, Serialize};

/// Provenance tag attached to each scorecard figure (T3 / #343).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FigureProvenance {
    Measured,
    Estimated,
    Unknown,
}

impl FigureProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Estimated => "estimated",
            Self::Unknown => "unknown",
        }
    }
}

/// A numeric figure with explicit measurement provenance and units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScorecardFigure {
    pub value: Option<f64>,
    pub provenance: FigureProvenance,
    pub unit: String,
    /// Requests with complete measurements / all observed usage requests.
    pub coverage: Option<MeasurementCoverage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasurementCoverage {
    pub measured_requests: u64,
    pub total_requests: u64,
}

impl ScorecardFigure {
    pub fn measured(value: f64, unit: impl Into<String>) -> Self {
        Self {
            value: Some(value),
            provenance: FigureProvenance::Measured,
            unit: unit.into(),
            coverage: None,
        }
    }

    pub fn estimated(value: f64, unit: impl Into<String>) -> Self {
        Self {
            value: Some(value),
            provenance: FigureProvenance::Estimated,
            unit: unit.into(),
            coverage: None,
        }
    }

    pub fn unknown(unit: impl Into<String>) -> Self {
        Self {
            value: None,
            provenance: FigureProvenance::Unknown,
            unit: unit.into(),
            coverage: None,
        }
    }
}

/// Harness session registration coverage status (T3 / #343).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessCoverage {
    Managed,
    Observed,
    Unregistered,
}

impl HarnessCoverage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Observed => "observed",
            Self::Unregistered => "unregistered",
        }
    }
}

/// A security-class audit event surfaced on the agent's scorecard card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScorecardSecurityEvent {
    pub id: String,
    pub operation: String,
    pub observed_at: String,
    pub status: String,
    pub task_id: Option<String>,
    pub work_session_id: Option<String>,
    pub provenance: FigureProvenance,
}

/// One agent's scorecard summary row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentScorecardRow {
    pub agent_id: String,
    pub display_name: String,
    pub oversight_role: Option<String>,
    pub open_security_events: Vec<ScorecardSecurityEvent>,
    pub oldest_pending_human_decision: Option<String>,
    pub tokens: ScorecardFigure,
    pub cost: ScorecardFigure,
    pub measured_cost_units: Option<f64>,
    pub estimated_cost_units: Option<f64>,
    pub budget_spent: Option<f64>,
    pub budget_limit: Option<f64>,
    pub budget_paused: bool,
    pub provenance: String,
    pub task_count: ScorecardFigure,
    /// Unresolved `scorecard.disagreement` rows naming this agent (T7 / #347).
    /// A count, never a score — the badge only says "inspect me".
    #[serde(default)]
    pub unresolved_disagreements: u64,
}

/// One harness's coverage and session status row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessScorecardRow {
    pub harness: String,
    pub workspace: String,
    pub coverage: HarnessCoverage,
    pub linked_agent_id: Option<String>,
    pub disk_session_id: Option<String>,
    pub state: Option<String>,
    pub session_count: ScorecardFigure,
}

/// One model provider's usage and gateway routing row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderScorecardRow {
    pub provider: String,
    pub via_gateway: Option<String>,
    pub tokens: ScorecardFigure,
    pub cost: ScorecardFigure,
    pub request_count: ScorecardFigure,
}

/// Complete projected scorecard response for the `hub_get_scorecard` command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScorecardView {
    pub agents: Vec<AgentScorecardRow>,
    pub harnesses: Vec<HarnessScorecardRow>,
    pub providers: Vec<ProviderScorecardRow>,
    pub aggregation_version: u32,
    pub generated_at: String,
    /// Advisory consistency-probe results (T7 / #347). Owner-only: non-owner
    /// callers always receive an empty vec, even under `all_agents`.
    #[serde(default)]
    pub probes: Vec<ScorecardProbe>,
    /// Reviewer-side oversight signature (T7 / #347). Owner-private; empty
    /// for every non-owner caller.
    #[serde(default)]
    pub reviewers: Vec<ReviewerSignatureRow>,
}

/// Outcome of one advisory consistency probe (T7 / #347).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeState {
    /// Both sources exist and agree within the probe's coarse tolerance.
    Ok,
    /// Both sources exist and contradict; a `scorecard.disagreement` row
    /// links them on the audit chain.
    Disagreement,
    /// A required source is absent; `reason` says which. Unknown is the
    /// honest state — never report "ok" for an unchecked comparison.
    Unknown,
}

impl ProbeState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Disagreement => "disagreement",
            Self::Unknown => "unknown",
        }
    }
}

/// One probe/subject pair with its state. Probes are descriptive and
/// advisory; there is deliberately no per-agent roll-up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScorecardProbe {
    /// Stable probe id: `quota_vs_cost`, `git_scope_vs_touch_set`,
    /// `network_vs_provider_log`, or `claimant_consistency`.
    pub probe: String,
    /// What the probe ran against: a provider id, task id, or `global`.
    pub subject: String,
    pub state: ProbeState,
    /// Why the state is what it is — especially required for `unknown`.
    pub reason: Option<String>,
    /// The audit rows the verdict rests on, so the owner can inspect both
    /// sides of a disagreement from the card.
    pub source_event_ids: Vec<String>,
    /// Roster agents implicated in the result (drives the card badge and the
    /// disagreement row's `agents` field). Empty for global/unknown rows.
    #[serde(default)]
    pub linked_agents: Vec<String>,
}

/// One reviewer agent's descriptive oversight signature (T7 / #347).
/// Every figure is independently provenanced; the row is never collapsed
/// into a reviewer-quality score, and there are no latency-only alerts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewerSignatureRow {
    pub reviewer_id: String,
    pub display_name: String,
    /// `review.finding` rows recorded by this reviewer.
    pub findings_recorded: ScorecardFigure,
    /// Those findings later corrected by the owner (`review.finding_corrected`).
    pub findings_corrected_by_owner: ScorecardFigure,
    /// Findings per distinct reviewed task. `unknown` until at least one
    /// finding exists — never zero-by-default.
    pub fix_find_rate: ScorecardFigure,
    /// Corrected ÷ recorded. `unknown` while no findings exist.
    pub owner_override_reversal_rate: ScorecardFigure,
    /// Resolved gate approvals for slices this reviewer claimed that carry no
    /// stamped evidence rows for the linked task.
    pub zero_evidence_approvals: ScorecardFigure,
    /// Reviewer decision timestamps do not exist in the Hub yet (review
    /// happens outside the decision rows), so dwell stays `unknown` rather
    /// than being approximated from an unrelated clock.
    pub review_dwell: ScorecardFigure,
}
