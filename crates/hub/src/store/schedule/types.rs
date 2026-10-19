//! Scheduled Goals types (`roadmaps/schedule.md` S1, #350/#351).
//!
//! `max_attempts`/`deadline` are mandatory, unconditional stop conditions
//! on every [`ScheduledTask`] — verified gap: neither `gate_provider_call`
//! nor `record_agent_metrics` are wired into the harness-CLI/PTY bridge
//! path this app actually dispatches work through, so budget/token-written
//! gates alone cannot be trusted to ever stop a loop for that target kind.

use serde::{Deserialize, Serialize};

/// Who a scheduled dispatch is addressed to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScheduleTarget {
    Agent {
        agent_id: String,
    },
    /// The standing roster (`agents.team_member`) — same set
    /// `send_message_to_team` already broadcasts to, not a work session's
    /// members.
    Team,
}

/// How a gate command's outcome decides pass/fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateCheck {
    /// Exit code 0 is a pass, anything else (including a timeout) fails.
    ExitCodeZero,
    /// Stdout parses as one non-negative integer; the value must be `<=
    /// max`. Used by [`GateCriterion::MaxLinesPerFile`] (one `wc -l
    /// <file>` invocation per matched file — never a multi-file `wc`
    /// call, which appends a `total` line that would break this parse).
    AtMost { max: u64 },
}

/// One pass/fail condition. `GateCriterion::Command` is the general,
/// always-trustworthy mechanism (reads live filesystem/process state
/// through the P5 `os_tool` sandbox, same policy as any other `os_tool`
/// call — no bypass). `MinTokensWritten` is a convenience built on
/// `agent_metrics`, which today only reflects `AgentSystem`'s in-app
/// `chat_completion` pipeline, not harness-CLI agents — see the roadmap's
/// "Known gap" section before relying on it as a sole stop condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GateCriterion {
    Command {
        program: String,
        args: Vec<String>,
        check: GateCheck,
    },
    MaxLinesPerFile {
        max_lines: u64,
        globs: Vec<String>,
    },
    MinTokensWritten {
        min_tokens: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateCombinator {
    #[default]
    All,
    Any,
}

impl GateCombinator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Any => "any",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "any" => Self::Any,
            _ => Self::All,
        }
    }
}

/// Spend tracking independent of the live provider-quota probe (which
/// lives in `src-tauri`, not `hub` — see the roadmap's cross-layer
/// note). `None` means no credit ceiling is enforced for this task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetPolicy {
    #[serde(default)]
    pub credit_allowance: Option<f64>,
    #[serde(default)]
    pub credit_spent: f64,
    /// Whether the in-app poller (S6) should also pause this task when a
    /// target's live session/weekly quota window is exhausted. No effect
    /// on headless `ca schedule tick` (S4), which cannot see that probe.
    #[serde(default)]
    pub stop_on_quota_exhausted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleStatus {
    Scheduled,
    Active,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

impl ScheduleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "scheduled" => Some(Self::Scheduled),
            "active" => Some(Self::Active),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub instruction: String,
    pub target: ScheduleTarget,
    pub workspace: String,
    pub fire_at: String,
    /// `None` = fire once, no gate loop. `Some(secs)` = after a response
    /// (or response-timeout) keep re-prompting at least this often while
    /// gates are unmet and no stop condition has tripped.
    #[serde(default)]
    pub repeat_interval_secs: Option<i64>,
    /// How long to wait for a response before counting the attempt as
    /// failed (against `max_attempts`) and re-prompting anyway. Only
    /// meaningful when `repeat_interval_secs` is set.
    #[serde(default = "default_response_timeout_secs")]
    pub response_timeout_secs: i64,
    #[serde(default)]
    pub gate_criteria: Vec<GateCriterion>,
    #[serde(default)]
    pub gate_combinator: GateCombinator,
    #[serde(default)]
    pub budget: BudgetPolicy,
    /// Mandatory, unconditional stop condition — see module docs.
    pub max_attempts: u32,
    #[serde(default)]
    pub deadline: Option<String>,
    pub status: ScheduleStatus,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub last_fired_at: Option<String>,
    #[serde(default)]
    pub next_fire_at: Option<String>,
    #[serde(default)]
    pub attempts: u32,
    /// Per-agent `agent_metrics.tokens_used` snapshot taken at creation
    /// time, so `MinTokensWritten` measures a delta, not a lifetime total.
    #[serde(default)]
    pub token_baseline: std::collections::HashMap<String, i64>,
}

fn default_response_timeout_secs() -> i64 {
    3600
}
