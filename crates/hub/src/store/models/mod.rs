use super::*;

pub mod embeddings;
mod memories;
mod memory_links;
pub use memory_links::{LinkSuggestion, UNATTRIBUTED_AUTHOR};
/// One step in a multi-agent workflow (C5).
///
/// Consecutive steps that share the same non-empty `parallel_group` form a
/// **parallel stage** (bounded by the task's `max_parallel`). Steps with
/// `parallel_group = null` are sequential one-agent stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub agent: String,
    #[serde(default)]
    pub role: Option<String>,
    pub instruction: String,
    /// How many times this step may be re-dispatched after `retry_task` (default 0).
    #[serde(default)]
    pub max_retries: u32,
    /// When set, adjacent steps with the same group run as one parallel stage.
    #[serde(default)]
    pub parallel_group: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Cancelled,
    Failed,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Result<Self, HubError> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "done" => Ok(Self::Done),
            "cancelled" => Ok(Self::Cancelled),
            "failed" => Ok(Self::Failed),
            other => Err(HubError::Invalid(format!("unknown task status: {other}"))),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub workspace_path: Option<String>,
    pub status: String,
    /// Index into the list of **stages** (sequential units / parallel groups).
    pub step_index: i64,
    pub steps: Vec<WorkflowStep>,
    pub created_at: String,
    pub updated_at: String,
    /// Last handoff message id produced by advance/retry, if any.
    pub last_message_id: Option<String>,
    /// Per-stage attempt counts (`"0" → 1` after first dispatch).
    pub attempts: std::collections::HashMap<String, u32>,
    /// Agents still outstanding in the current parallel stage (empty when sequential).
    pub open_agents: Vec<String>,
    /// Agents in the current stage not yet woken (queued behind max_parallel).
    pub pending_agents: Vec<String>,
    /// Max concurrent wakes inside a parallel stage (default 4).
    pub max_parallel: u32,
    /// Whether this task requires human approval for delegation/wakes (C4).
    pub require_human_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactReport {
    pub examined: usize,
    pub promoted: usize,
    pub kept: usize,
    pub skipped: usize,
}

/// Standing policy for wake/delegation human gates (C4 skeleton).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakePolicy {
    /// When true, every wake request requires human approval.
    pub default_requires_human_gate: bool,
    /// When false, auto-wake without a gate is rejected.
    pub allow_auto_wake: bool,
}

impl Default for WakePolicy {
    fn default() -> Self {
        Self {
            default_requires_human_gate: true,
            allow_auto_wake: true,
        }
    }
}

/// Per-agent provider-call/spend budget (C6). Units are caller-defined
/// (call count, USD, tokens, ...) — the store only compares totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub agent_id: String,
    pub limit_units: f64,
    pub spent_units: f64,
    /// True once spend has reached or exceeded the limit; wakes are blocked
    /// for this agent until a human explicitly `resume_agent`s it.
    pub paused: bool,
    pub updated_at: String,
}

/// Cumulative usage counters used by the local observability dashboard.
/// Token counts are estimates unless a provider adapter reports exact values;
/// cache hits remain zero until such an adapter is connected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_id: String,
    pub lines_written: i64,
    pub tokens_used: i64,
    pub tokens_cached: i64,
    pub provider_calls: i64,
    pub output_chars: i64,
    pub updated_at: String,
}

/// Result of `HubStore::pause_for_budget` (C6): the exhaustion handoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPauseOutcome {
    pub status: BudgetStatus,
    /// Markdown handoff summary path under `markdown/handoffs/`.
    pub summary_path: PathBuf,
    /// The durable handoff message id sent to `delegate_to` (or `"human"`).
    pub handoff_message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownOutcome {
    pub summary_path: PathBuf,
    pub handoff_message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub root_path: String,
    pub path: String,
    pub operation: String,
    pub observed_at: String,
    pub process_json: String,
    pub content_hash: Option<String>,
    pub previous_hash: Option<String>,
    pub event_hash: String,
    pub status: String,
}

/// A Chat & Memory channel (`#general`, custom names). Work sessions and
/// DMs are not stored here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRecord {
    pub id: String,
    pub name: String,
    pub topic: Option<String>,
    pub builtin: bool,
    pub created_at: String,
}

/// Whether the Hub owns the provider process/session or only observes it.
///
/// Observed sessions retain C12's conservative safety boundary: they may be
/// captured and, only where the provider publishes a safe bridge, messaged.
/// Managed sessions are explicitly created by Coding-Assistants and may claim
/// the per-session writer lease required by C14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessSessionMode {
    Observed,
    Managed,
}

impl HarnessSessionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Managed => "managed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, HubError> {
        match value {
            "observed" => Ok(Self::Observed),
            "managed" => Ok(Self::Managed),
            other => Err(HubError::Invalid(format!(
                "unknown harness session mode: {other}"
            ))),
        }
    }
}

/// Truthful provider readiness used by managed-session UI and delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessSessionState {
    Ready,
    Busy,
    Queued,
    Unavailable,
    Stopped,
}

impl HarnessSessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Busy => "busy",
            Self::Queued => "queued",
            Self::Unavailable => "unavailable",
            Self::Stopped => "stopped",
        }
    }

    pub fn parse(value: &str) -> Result<Self, HubError> {
        match value {
            "ready" => Ok(Self::Ready),
            "busy" => Ok(Self::Busy),
            "queued" => Ok(Self::Queued),
            "unavailable" => Ok(Self::Unavailable),
            "stopped" => Ok(Self::Stopped),
            other => Err(HubError::Invalid(format!(
                "unknown harness session state: {other}"
            ))),
        }
    }
}

/// An explicitly registered harness session (C12 / C14 bridge).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessSessionRegistration {
    pub harness: String,
    pub workspace: String,
    pub disk_session_id: String,
    pub leader_socket: Option<String>,
    pub registered_at: String,
    pub mode: HarnessSessionMode,
    pub state: HarnessSessionState,
    pub managed_pid: Option<u32>,
    pub writer_owner: Option<String>,
    pub writer_acquired_at: Option<String>,
}
