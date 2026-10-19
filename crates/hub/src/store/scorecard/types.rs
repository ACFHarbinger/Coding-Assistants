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
}
