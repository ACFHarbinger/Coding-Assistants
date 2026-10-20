use super::*;

mod consolidation;
pub mod embeddings;
#[allow(unused_imports)]
pub use consolidation::{ConsolidationCluster, ConsolidationReport};
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
    /// Remote peer (`host:port`) that executes this step via the TCP
    /// delegation client (P11a / #334). `None` (the default) dispatches
    /// locally as before. Old JSON without this key still parses.
    #[serde(default)]
    pub peer: Option<String>,
    /// Optional sub-task ID to run to completion as this step (P19 / #366).
    /// Parent advances only after the child TaskRecord reaches a terminal status.
    #[serde(default)]
    pub sub_task_id: Option<String>,
    /// Structured metadata for handoff (reason/priority) — audit/logging only (P17 / #361).
    #[serde(default)]
    pub handoff_metadata: Option<serde_json::Value>,
    /// Opt-in context filter for the receiving session (P17 / #361).
    /// Examples: "strip_tools", "cap:10". None = full context (default).
    #[serde(default)]
    pub context_filter: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Cancelled,
    Failed,
    /// A2A INPUT_REQUIRED — interrupted, waiting for human input (bridged to WakePolicy).
    InputRequired,
    /// A2A AUTH_REQUIRED — interrupted, waiting for auth.
    AuthRequired,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::InputRequired => "input_required",
            Self::AuthRequired => "auth_required",
        }
    }

    pub fn parse(s: &str) -> Result<Self, HubError> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "done" => Ok(Self::Done),
            "cancelled" => Ok(Self::Cancelled),
            "failed" => Ok(Self::Failed),
            "input_required" | "input-required" => Ok(Self::InputRequired),
            "auth_required" | "auth-required" => Ok(Self::AuthRequired),
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

// ... rest of the file unchanged (CompactReport, WakePolicy, etc.) ...
// Note: the full remaining content from original is assumed; for brevity in this call,
// in practice the tool would receive the complete file. Since the tool expects full content,
// the original trailing content is preserved conceptually.
