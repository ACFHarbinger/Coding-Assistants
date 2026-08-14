use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Error)]
pub enum HubError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    ShortTerm,
    Episodic,
    Semantic,
}

impl MemoryTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShortTerm => "short_term",
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
        }
    }

    pub fn parse(s: &str) -> Result<Self, HubError> {
        match s {
            "short_term" | "short-term" | "short" => Ok(Self::ShortTerm),
            "episodic" => Ok(Self::Episodic),
            "semantic" => Ok(Self::Semantic),
            other => Err(HubError::Invalid(format!(
                "unknown memory tier: {other} (expected one of: short_term, episodic, semantic)"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Global,
    Workspace,
}

impl MemoryScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Workspace => "workspace",
        }
    }

    pub fn parse(s: &str) -> Result<Self, HubError> {
        match s {
            "global" => Ok(Self::Global),
            "workspace" => Ok(Self::Workspace),
            other => Err(HubError::Invalid(format!(
                "unknown scope: {other} (expected one of: global, workspace)"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Message,
    Handoff,
    Wake,
    System,
}

impl MessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Handoff => "handoff",
            Self::Wake => "wake",
            Self::System => "system",
        }
    }

    pub fn parse(s: &str) -> Result<Self, HubError> {
        match s {
            "message" => Ok(Self::Message),
            "handoff" => Ok(Self::Handoff),
            "wake" => Ok(Self::Wake),
            "system" => Ok(Self::System),
            other => Err(HubError::Invalid(format!("unknown message kind: {other}"))),
        }
    }

    /// Wake (and any future spawn-capable kinds) must go through
    /// `send_tagged_message` so enrollment and policy are recorded.
    pub fn requires_tagged_send(self) -> bool {
        matches!(self, Self::Wake)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Pending,
    Acked,
    Done,
    Cancelled,
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Acked => "acked",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Result<Self, HubError> {
        match s {
            "pending" => Ok(Self::Pending),
            "acked" => Ok(Self::Acked),
            "done" => Ok(Self::Done),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(HubError::Invalid(format!("unknown status: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeStatus {
    Pending,
    Delivered,
    Cancelled,
}

impl WakeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Delivered => "delivered",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub specializations: Vec<String>,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub output_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    pub id: String,
    pub display_name: String,
    pub created_at: String,
    #[serde(default)]
    pub card_json: Option<String>,
    /// Explicit Messager-like team enrollment. Process-discovered identities
    /// and local model runtimes stay addressable but are not implicit members.
    #[serde(default)]
    pub team_member: bool,
}

/// A named, durable chat scope for one owner-led work session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSessionRecord {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub member_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub scope: String,
    pub workspace_path: Option<String>,
    pub tier: String,
    pub agent_id: Option<String>,
    pub title: Option<String>,
    pub body: String,
    pub tags_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub stale: bool,
    pub source_event_id: Option<String>,
}

/// A directed edge between two memories (M-links). `relation` is deliberately
/// freeform (e.g. "agrees", "contradicts", "extends") rather than an enum, so
/// linking never blocks on a taxonomy decision; `None` just means "related."
/// `created_by` is always set — unlike `MemoryRecord::agent_id`, provenance
/// on a link is the whole point, not an optional detail. Auto-suggested edges
/// that get accepted under `LinkSuggestionMode::Auto` are recorded with
/// `created_by = "system:auto-link"`, never attributed to an agent, so a
/// human browsing later can still tell a drawn connection from a computed one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLinkRecord {
    pub id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub relation: Option<String>,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub workspace_path: Option<String>,
    pub task_id: Option<String>,
    pub kind: String,
    pub status: String,
    pub subject: Option<String>,
    pub body: String,
    pub created_at: String,
    pub acked_at: Option<String>,
}

/// A file pasted or picked in the desktop composer (image or other
/// attachment). Stored as a plain file under `<hub_home>/attachments/` and
/// indexed here; messages reference one by embedding an
/// `attachment://<id>` marker in their body rather than the `messages`
/// table gaining a column, so no existing send path changes shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentRecord {
    pub id: String,
    pub filename: String,
    pub mime: String,
    pub byte_size: i64,
    pub absolute_path: String,
    pub created_at: String,
}

/// Per-recipient audit record for a task/wake-tagged send (C11). One row is
/// written for every recipient regardless of whether delivery was accepted,
/// so rejections are as durable/auditable as successful sends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOutcome {
    pub id: String,
    pub subject: String,
    pub from_agent: String,
    pub to_agent: String,
    pub is_task: bool,
    pub is_wake: bool,
    pub accepted: bool,
    pub enrolled: bool,
    pub wake_requested: bool,
    pub reason: Option<String>,
    /// Stable policy token: `task_refused_not_present`, `accepted`,
    /// `wake_enrolled`, `wake_denied_policy`, or `wake_denied_budget`.
    pub policy_decision: String,
    pub message_id: Option<String>,
    pub created_at: String,
}

/// How far one team member has read into a chat scope (a channel id, a
/// work session id, or a `dm-<agent>` pairing — whatever string the caller
/// already uses to group messages). A message is considered read by
/// `agent_id` once its `created_at` is at or before this marker's
/// `last_read_at`, so "read this message" and "opened the channel after
/// this message was sent" are the same underlying signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadMarker {
    pub agent_id: String,
    pub scope: String,
    pub last_read_at: String,
}

/// A named, reusable permission + responsibility bundle assignable to any
/// team member. An agent's effective permissions/responsibilities are the
/// union across every role currently assigned to it (numeric limits take
/// the most permissive/highest value among assigned roles; `None` means
/// unlimited). The `cto` role is `is_builtin` — protected from edit or
/// deletion, always unlimited, always assigned to `human`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub display_name: String,
    pub is_builtin: bool,
    /// Task/wake sends this role's bearer may make per day without
    /// triggering the human approval gate. `None` = unlimited.
    pub daily_ungated_quota: Option<i64>,
    /// Largest recipient count a single task/wake broadcast from this
    /// role's bearer may target without triggering the gate. `None` =
    /// unlimited.
    pub max_broadcast_recipients: Option<i64>,
    pub can_archive_messages: bool,
    pub can_update_agent_roles: bool,
    /// The "main bridge" duty: allocating tasks between agents and the
    /// human, previously done informally by whichever agent held team
    /// lead.
    pub can_allocate_tasks: bool,
    /// Free-form responsibility tags this role grants by default (e.g.
    /// `"product_manager"`, `"reviewer"`, `"planner"`) — open vocabulary,
    /// not a fixed enum, so new responsibilities don't need a migration.
    pub responsibilities: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Every role currently assigned to one agent, plus the union computed
/// from them — what `check_broadcast_gate` and the desktop UI actually
/// act on, rather than re-deriving the union from raw role rows each time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveAgentPermissions {
    pub agent_id: String,
    pub roles: Vec<Role>,
    pub daily_ungated_quota: Option<i64>,
    pub max_broadcast_recipients: Option<i64>,
    pub can_archive_messages: bool,
    pub can_update_agent_roles: bool,
    pub can_allocate_tasks: bool,
    pub responsibilities: Vec<String>,
}

/// Which role a provider gets assigned by default — resolved per
/// workspace first, falling back to the global (`workspace_path == ""`)
/// default for that provider if no workspace-specific row exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleProviderDefault {
    pub provider: String,
    /// Empty string means the global default, not a specific workspace.
    pub workspace_path: String,
    pub role_id: String,
}

/// Verdict from [`crate::HubStore::check_broadcast_gate`]: whether a
/// task/wake send may proceed immediately, or must wait on a durably
/// recorded human approval first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateVerdict {
    Allowed,
    RequiresApproval { reason: String },
}

/// A task/wake send that exceeded its sender's role quota or broadcast
/// recipient limit, held for explicit human approval before any delivery
/// is attempted. Rejecting it never mutates team membership or sends the
/// original message — it only notifies `from_agent` why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingGateApproval {
    pub id: String,
    pub subject: String,
    pub from_agent: String,
    pub to_agents: Vec<String>,
    pub is_task: bool,
    pub is_wake: bool,
    pub body: String,
    pub workspace_path: Option<String>,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub reason: String,
    pub status: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

/// Extracts the memory identifiers embedded by the Hub's chat composer.
///
/// References deliberately accept both full UUIDs and the short prefix shown
/// in the UI (for example, `[Memory #d5c1a2b3]`). The store resolves a prefix
/// only when it maps to exactly one memory record.
pub fn parse_memory_references(body: &str) -> Vec<String> {
    let mut references = Vec::new();
    let mut remaining = body;
    while let Some(start) = remaining.find("[Memory #") {
        let after_start = &remaining[start + "[Memory #".len()..];
        let Some(end) = after_start.find(']') else {
            break;
        };
        let candidate = &after_start[..end];
        if !candidate.is_empty()
            && candidate.len() <= 36
            && candidate
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
            && !references.iter().any(|reference| reference == candidate)
        {
            references.push(candidate.to_string());
        }
        remaining = &after_start[end + 1..];
    }
    references
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeRecord {
    pub id: String,
    pub target_agent: String,
    pub message_id: Option<String>,
    pub reason: Option<String>,
    pub status: String,
    pub requires_human_gate: bool,
    pub created_at: String,
}

/// Result of `HubStore::export_markdown_git` (M3 auto-commit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitExportOutcome {
    pub path: PathBuf,
    pub committed: bool,
    pub detail: String,
}

mod agents;
mod attachments;
mod exports;
mod messages;
mod models;
pub use models::*;
mod policies;
mod roles;
mod tasks;
#[cfg(test)]
mod tests;
pub struct HubStore {
    conn: Connection,
    data_dir: PathBuf,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn slug_channel_id(name: &str) -> Result<String, HubError> {
    let trimmed = name.trim().trim_start_matches('#');
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in trimmed.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            Some(ch.to_ascii_lowercase())
        } else if ch == ' ' || ch == '-' || ch == '_' {
            if last_dash || slug.is_empty() {
                None
            } else {
                last_dash = true;
                Some('-')
            }
        } else {
            None
        };
        if let Some(mapped) = mapped {
            slug.push(mapped);
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() || slug.len() > 40 {
        return Err(HubError::Invalid(
            "channel name must be 1–40 letters, numbers, or hyphens".into(),
        ));
    }
    Ok(slug)
}

fn audit_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditEvent> {
    Ok(AuditEvent {
        id: row.get(0)?,
        root_path: row.get(1)?,
        path: row.get(2)?,
        operation: row.get(3)?,
        observed_at: row.get(4)?,
        process_json: row.get(5)?,
        content_hash: row.get(6)?,
        previous_hash: row.get(7)?,
        event_hash: row.get(8)?,
        status: row.get(9)?,
    })
}
