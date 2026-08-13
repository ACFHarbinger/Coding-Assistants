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
    /// Explicit Slack-like team enrollment. Process-discovered identities
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
    pub message_id: Option<String>,
    pub created_at: String,
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

pub struct HubStore {
    conn: Connection,
    data_dir: PathBuf,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
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

impl HubStore {
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self, HubError> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(data_dir.join("journals"))?;
        fs::create_dir_all(data_dir.join("markdown"))?;
        fs::create_dir_all(data_dir.join("wake"))?;

        let db_path = data_dir.join("hub.db");
        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        let store = Self { conn, data_dir };
        store.migrate()?;
        Ok(store)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Record an observed filesystem change. The hash chain makes later
    /// deletion or reordering of rows detectable by `verify_audit_chain`.
    pub fn record_audit_event(
        &self,
        root_path: &Path,
        path: &Path,
        operation: &str,
        process_json: &str,
        content_hash: Option<&str>,
    ) -> Result<AuditEvent, HubError> {
        if operation.trim().is_empty() || process_json.trim().is_empty() {
            return Err(HubError::Invalid(
                "audit operation and process metadata are required".into(),
            ));
        }
        let root_path = root_path.to_string_lossy().to_string();
        let path = path.to_string_lossy().to_string();
        let observed_at = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let tx = self.conn.unchecked_transaction()?;
        let previous_hash: Option<String> = tx
            .query_row(
                "SELECT event_hash FROM audit_events ORDER BY rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let canonical = serde_json::json!({
            "id": id,
            "root_path": root_path,
            "path": path,
            "operation": operation,
            "observed_at": observed_at,
            "process_json": process_json,
            "content_hash": content_hash,
            "previous_hash": previous_hash,
        });
        let event_hash = sha256_hex(
            &serde_json::to_vec(&canonical).map_err(|e| HubError::Invalid(e.to_string()))?,
        );
        tx.execute(
            "INSERT INTO audit_events(id, root_path, path, operation, observed_at, process_json, content_hash, previous_hash, event_hash, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
            params![
                id,
                root_path,
                path,
                operation,
                observed_at,
                process_json,
                content_hash,
                previous_hash,
                event_hash
            ],
        )?;
        tx.commit()?;
        Ok(AuditEvent {
            id,
            root_path,
            path,
            operation: operation.into(),
            observed_at,
            process_json: process_json.into(),
            content_hash: content_hash.map(str::to_string),
            previous_hash,
            event_hash,
            status: "pending".into(),
        })
    }

    pub fn list_audit_events(&self, pending_only: bool) -> Result<Vec<AuditEvent>, HubError> {
        let sql = if pending_only {
            "SELECT id, root_path, path, operation, observed_at, process_json, content_hash, previous_hash, event_hash, status FROM audit_events WHERE status = 'pending' ORDER BY rowid"
        } else {
            "SELECT id, root_path, path, operation, observed_at, process_json, content_hash, previous_hash, event_hash, status FROM audit_events ORDER BY rowid"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], audit_event_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn set_audit_status(&self, id: &str, status: &str) -> Result<(), HubError> {
        if !matches!(status, "approved" | "quarantined" | "pending") {
            return Err(HubError::Invalid(format!("unknown audit status: {status}")));
        }
        let changed = self.conn.execute(
            "UPDATE audit_events SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        if changed == 0 {
            return Err(HubError::NotFound(format!("audit event {id}")));
        }
        Ok(())
    }

    pub fn verify_audit_chain(&self) -> Result<usize, HubError> {
        let events = self.list_audit_events(false)?;
        let mut previous = None;
        for event in &events {
            if event.previous_hash != previous {
                return Err(HubError::Invalid(format!(
                    "audit chain link broken at {}",
                    event.id
                )));
            }
            let canonical = serde_json::json!({
                "id": event.id,
                "root_path": event.root_path,
                "path": event.path,
                "operation": event.operation,
                "observed_at": event.observed_at,
                "process_json": event.process_json,
                "content_hash": event.content_hash,
                "previous_hash": event.previous_hash,
            });
            let expected = sha256_hex(
                &serde_json::to_vec(&canonical).map_err(|e| HubError::Invalid(e.to_string()))?,
            );
            if expected != event.event_hash {
                return Err(HubError::Invalid(format!(
                    "audit event hash mismatch at {}",
                    event.id
                )));
            }
            previous = Some(event.event_hash.clone());
        }
        Ok(events.len())
    }

    fn migrate(&self) -> Result<(), HubError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY NOT NULL,
                display_name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                card_json TEXT,
                team_member INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY NOT NULL,
                scope TEXT NOT NULL,
                workspace_path TEXT,
                tier TEXT NOT NULL,
                agent_id TEXT,
                title TEXT,
                body TEXT NOT NULL,
                tags_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                stale INTEGER NOT NULL DEFAULT 0,
                source_event_id TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_memories_scope_tier
                ON memories(scope, tier, stale);
            CREATE INDEX IF NOT EXISTS idx_memories_workspace
                ON memories(workspace_path);

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY NOT NULL,
                from_agent TEXT NOT NULL,
                to_agent TEXT NOT NULL,
                workspace_path TEXT,
                task_id TEXT,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                subject TEXT,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL,
                acked_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_messages_to_status
                ON messages(to_agent, status, created_at);

            CREATE TABLE IF NOT EXISTS wake_requests (
                id TEXT PRIMARY KEY NOT NULL,
                target_agent TEXT NOT NULL,
                message_id TEXT,
                reason TEXT,
                status TEXT NOT NULL,
                requires_human_gate INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_wake_target_status
                ON wake_requests(target_agent, status);

            CREATE TABLE IF NOT EXISTS work_sessions (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS work_session_members (
                session_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY(session_id, agent_id),
                FOREIGN KEY(session_id) REFERENCES work_sessions(id) ON DELETE CASCADE,
                FOREIGN KEY(agent_id) REFERENCES agents(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                workspace_path TEXT,
                status TEXT NOT NULL,
                step_index INTEGER NOT NULL DEFAULT 0,
                steps_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_message_id TEXT,
                attempts_json TEXT NOT NULL DEFAULT '{}',
                open_agents_json TEXT NOT NULL DEFAULT '[]',
                pending_agents_json TEXT NOT NULL DEFAULT '[]',
                max_parallel INTEGER NOT NULL DEFAULT 4,
                require_human_approval INTEGER NOT NULL DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_status
                ON tasks(status, updated_at);

            CREATE TABLE IF NOT EXISTS agent_budgets (
                agent_id TEXT PRIMARY KEY NOT NULL,
                limit_units REAL NOT NULL,
                spent_units REAL NOT NULL DEFAULT 0,
                paused INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS agent_metrics (
                agent_id TEXT PRIMARY KEY NOT NULL,
                lines_written INTEGER NOT NULL DEFAULT 0,
                tokens_used INTEGER NOT NULL DEFAULT 0,
                tokens_cached INTEGER NOT NULL DEFAULT 0,
                provider_calls INTEGER NOT NULL DEFAULT 0,
                output_chars INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY NOT NULL,
                root_path TEXT NOT NULL,
                path TEXT NOT NULL,
                operation TEXT NOT NULL,
                observed_at TEXT NOT NULL,
                process_json TEXT NOT NULL,
                content_hash TEXT,
                previous_hash TEXT,
                event_hash TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL DEFAULT 'pending'
            );

            CREATE INDEX IF NOT EXISTS idx_audit_status_time
                ON audit_events(status, observed_at);

            CREATE TABLE IF NOT EXISTS tagged_send_outcomes (
                id TEXT PRIMARY KEY NOT NULL,
                subject TEXT NOT NULL,
                from_agent TEXT NOT NULL,
                to_agent TEXT NOT NULL,
                is_task INTEGER NOT NULL DEFAULT 0,
                is_wake INTEGER NOT NULL DEFAULT 0,
                accepted INTEGER NOT NULL,
                enrolled INTEGER NOT NULL DEFAULT 0,
                wake_requested INTEGER NOT NULL DEFAULT 0,
                reason TEXT,
                message_id TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tagged_send_outcomes_subject
                ON tagged_send_outcomes(subject, created_at);
            "#,
        )?;

        // Soft-migrate columns for DBs created before C5 retries/parallel.
        for ddl in [
            "ALTER TABLE agents ADD COLUMN card_json TEXT",
            "ALTER TABLE agents ADD COLUMN team_member INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE tasks ADD COLUMN attempts_json TEXT NOT NULL DEFAULT '{}'",
            "ALTER TABLE tasks ADD COLUMN open_agents_json TEXT NOT NULL DEFAULT '[]'",
            "ALTER TABLE tasks ADD COLUMN pending_agents_json TEXT NOT NULL DEFAULT '[]'",
            "ALTER TABLE tasks ADD COLUMN max_parallel INTEGER NOT NULL DEFAULT 4",
            "ALTER TABLE tasks ADD COLUMN require_human_approval INTEGER NOT NULL DEFAULT 1",
        ] {
            let _ = self.conn.execute(ddl, []);
        }

        let version: Option<i64> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .and_then(|s| s.parse().ok());

        if version.is_none() {
            self.conn.execute(
                "INSERT INTO meta(key, value) VALUES ('schema_version', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
        }

        // Seed well-known agents if empty.
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM agents", [], |r| r.get(0))?;
        if count == 0 {
            for (id, name) in [
                ("human", "Human"),
                ("claude", "Claude Code"),
                ("chat", "Codex / Chat"),
                ("gemini", "Gemini / Antigravity"),
                ("grok", "Grok Build"),
                ("opencode", "OpenCode"),
                ("ollama", "Ollama"),
                ("llamacpp", "llama.cpp"),
                ("system", "System"),
            ] {
                self.upsert_agent(id, name)?;
            }
        }

        // One-time default roster after agents exist. Later enroll/unenroll persists.
        let roster_seeded: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'team_roster_seeded'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if roster_seeded.is_none() {
            self.conn.execute(
                "UPDATE agents SET team_member = 1 WHERE id IN ('human', 'claude', 'chat', 'gemini', 'grok')",
                [],
            )?;
            self.conn.execute(
                "INSERT OR REPLACE INTO meta(key, value) VALUES ('team_roster_seeded', '1')",
                [],
            )?;
        }

        Ok(())
    }

    pub fn upsert_agent(&self, id: &str, display_name: &str) -> Result<(), HubError> {
        self.conn.execute(
            "INSERT INTO agents (id, display_name, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET display_name = ?2",
            params![id, display_name, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn upsert_agent_card(&self, id: &str, card: &AgentCard) -> Result<(), HubError> {
        let card_json =
            serde_json::to_string(card).map_err(|e| HubError::Invalid(e.to_string()))?;
        self.conn.execute(
            "INSERT INTO agents (id, display_name, created_at, card_json) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET display_name = ?2, card_json = ?4",
            params![id, card.name, Utc::now().to_rfc3339(), card_json],
        )?;
        Ok(())
    }

    pub fn list_agents(&self) -> Result<Vec<AgentRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, display_name, created_at, card_json, team_member FROM agents ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AgentRecord {
                id: r.get(0)?,
                display_name: r.get(1)?,
                created_at: r.get(2)?,
                card_json: r.get(3)?,
                team_member: r.get::<_, i64>(4)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_team_members(&self) -> Result<Vec<AgentRecord>, HubError> {
        Ok(self
            .list_agents()?
            .into_iter()
            .filter(|agent| agent.team_member)
            .collect())
    }

    pub fn set_team_member(&self, id: &str, enrolled: bool) -> Result<AgentRecord, HubError> {
        let updated = self.conn.execute(
            "UPDATE agents SET team_member = ?1 WHERE id = ?2",
            params![if enrolled { 1 } else { 0 }, id],
        )?;
        if updated == 0 {
            return Err(HubError::NotFound(id.to_string()));
        }
        self.list_agents()?
            .into_iter()
            .find(|agent| agent.id == id)
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }

    /// Creates a named work-session chat and enrolls the current persisted team.
    pub fn create_work_session(&self, name: &str) -> Result<WorkSessionRecord, HubError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 120 {
            return Err(HubError::Invalid(
                "work session name must be between 1 and 120 characters".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO work_sessions(id, name, created_at) VALUES (?1, ?2, ?3)",
            params![id, name, created_at],
        )?;
        tx.execute(
            "INSERT INTO work_session_members(session_id, agent_id, created_at)
             SELECT ?1, id, ?2 FROM agents WHERE team_member = 1",
            params![id, created_at],
        )?;
        tx.commit()?;
        self.get_work_session(&id)?
            .ok_or_else(|| HubError::NotFound(id))
    }

    pub fn list_work_sessions(&self) -> Result<Vec<WorkSessionRecord>, HubError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, created_at FROM work_sessions ORDER BY created_at DESC")?;
        let sessions = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        sessions
            .into_iter()
            .map(|(id, name, created_at)| self.work_session_record(id, name, created_at))
            .collect()
    }

    pub fn add_work_session_member(
        &self,
        session_id: &str,
        agent_id: &str,
    ) -> Result<WorkSessionRecord, HubError> {
        if self.get_work_session(session_id)?.is_none() {
            return Err(HubError::NotFound(session_id.to_string()));
        }
        if !self.list_agents()?.iter().any(|agent| agent.id == agent_id) {
            return Err(HubError::NotFound(agent_id.to_string()));
        }
        self.conn.execute(
            "INSERT OR IGNORE INTO work_session_members(session_id, agent_id, created_at)
             VALUES (?1, ?2, ?3)",
            params![session_id, agent_id, Utc::now().to_rfc3339()],
        )?;
        self.get_work_session(session_id)?
            .ok_or_else(|| HubError::NotFound(session_id.to_string()))
    }

    fn get_work_session(&self, id: &str) -> Result<Option<WorkSessionRecord>, HubError> {
        self.conn
            .query_row(
                "SELECT id, name, created_at FROM work_sessions WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .map(|(id, name, created_at)| self.work_session_record(id, name, created_at))
            .transpose()
    }

    fn work_session_record(
        &self,
        id: String,
        name: String,
        created_at: String,
    ) -> Result<WorkSessionRecord, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_id FROM work_session_members WHERE session_id = ?1 ORDER BY agent_id",
        )?;
        let member_ids = stmt
            .query_map(params![id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WorkSessionRecord {
            id,
            name,
            created_at,
            member_ids,
        })
    }

    /// Wake every enrolled teammate except the sender and `system`.
    /// Slack/Orchestrate team sends must use this instead of waking a single harness.
    pub fn request_team_wakes(
        &self,
        from_agent: &str,
        reason: Option<&str>,
        message_id: Option<&str>,
        requires_human_gate: bool,
    ) -> Result<Vec<WakeRecord>, HubError> {
        let mut wakes = Vec::new();
        for member in self.list_team_members()? {
            if member.id == from_agent || member.id == "system" {
                continue;
            }
            wakes.push(self.request_wake(&member.id, reason, message_id, requires_human_gate)?);
        }
        Ok(wakes)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_memory(
        &self,
        tier: MemoryTier,
        scope: MemoryScope,
        agent_id: Option<&str>,
        workspace_path: Option<&str>,
        title: Option<&str>,
        body: &str,
        tags: &[String],
    ) -> Result<MemoryRecord, HubError> {
        self.write_memory_with_source(
            tier,
            scope,
            agent_id,
            workspace_path,
            title,
            body,
            tags,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_memory_with_source(
        &self,
        tier: MemoryTier,
        scope: MemoryScope,
        agent_id: Option<&str>,
        workspace_path: Option<&str>,
        title: Option<&str>,
        body: &str,
        tags: &[String],
        source_event_id: Option<&str>,
    ) -> Result<MemoryRecord, HubError> {
        if scope == MemoryScope::Workspace && workspace_path.is_none() {
            return Err(HubError::Invalid(
                "workspace scope requires --workspace".into(),
            ));
        }
        if body.trim().is_empty() {
            return Err(HubError::Invalid("memory body must not be empty".into()));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".into());

        self.conn.execute(
            r#"
            INSERT INTO memories(
                id, scope, workspace_path, tier, agent_id, title, body,
                tags_json, created_at, updated_at, stale, source_event_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11)
            "#,
            params![
                id,
                scope.as_str(),
                workspace_path,
                tier.as_str(),
                agent_id,
                title,
                body,
                tags_json,
                now,
                now,
                source_event_id,
            ],
        )?;

        self.get_memory(&id)?.ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_memory(&self, id: &str) -> Result<Option<MemoryRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, scope, workspace_path, tier, agent_id, title, body,
                   tags_json, created_at, updated_at, stale, source_event_id
            FROM memories WHERE id = ?1
            "#,
        )?;
        let row = stmt
            .query_row(params![id], |r| {
                Ok(MemoryRecord {
                    id: r.get(0)?,
                    scope: r.get(1)?,
                    workspace_path: r.get(2)?,
                    tier: r.get(3)?,
                    agent_id: r.get(4)?,
                    title: r.get(5)?,
                    body: r.get(6)?,
                    tags_json: r.get(7)?,
                    created_at: r.get(8)?,
                    updated_at: r.get(9)?,
                    stale: r.get::<_, i64>(10)? != 0,
                    source_event_id: r.get(11)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    pub fn update_memory(
        &self,
        id: &str,
        title: Option<&str>,
        body: &str,
        tags: Option<&[String]>,
    ) -> Result<MemoryRecord, HubError> {
        let now = Utc::now().to_rfc3339();

        if body.trim().is_empty() {
            return Err(HubError::Invalid("memory body must not be empty".into()));
        }

        if let Some(t) = tags {
            let tags_json = serde_json::to_string(t).unwrap_or_else(|_| "[]".into());
            let updated = self.conn.execute(
                "UPDATE memories SET title = ?1, body = ?2, tags_json = ?3, updated_at = ?4 WHERE id = ?5",
                params![title, body, tags_json, now, id],
            )?;
            if updated == 0 {
                return Err(HubError::NotFound(id.to_string()));
            }
        } else {
            let updated = self.conn.execute(
                "UPDATE memories SET title = ?1, body = ?2, updated_at = ?3 WHERE id = ?4",
                params![title, body, now, id],
            )?;
            if updated == 0 {
                return Err(HubError::NotFound(id.to_string()));
            }
        }

        self.get_memory(id)?
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }

    pub fn list_memories(
        &self,
        scope: Option<MemoryScope>,
        tier: Option<MemoryTier>,
        workspace_path: Option<&str>,
        include_stale: bool,
    ) -> Result<Vec<MemoryRecord>, HubError> {
        let mut sql = String::from(
            r#"
            SELECT id, scope, workspace_path, tier, agent_id, title, body,
                   tags_json, created_at, updated_at, stale, source_event_id
            FROM memories WHERE 1=1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if !include_stale {
            sql.push_str(" AND stale = 0");
        }
        if let Some(s) = scope {
            sql.push_str(" AND scope = ?");
            params_vec.push(Box::new(s.as_str().to_string()));
        }
        if let Some(t) = tier {
            sql.push_str(" AND tier = ?");
            params_vec.push(Box::new(t.as_str().to_string()));
        }
        if let Some(ws) = workspace_path {
            sql.push_str(" AND workspace_path = ?");
            params_vec.push(Box::new(ws.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 200");

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(MemoryRecord {
                id: r.get(0)?,
                scope: r.get(1)?,
                workspace_path: r.get(2)?,
                tier: r.get(3)?,
                agent_id: r.get(4)?,
                title: r.get(5)?,
                body: r.get(6)?,
                tags_json: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
                stale: r.get::<_, i64>(10)? != 0,
                source_event_id: r.get(11)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn search_memories(&self, query: &str) -> Result<Vec<MemoryRecord>, HubError> {
        let q = format!("%{}%", query.trim());
        if query.trim().is_empty() {
            return Err(HubError::Invalid("search query must not be empty".into()));
        }
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, scope, workspace_path, tier, agent_id, title, body,
                   tags_json, created_at, updated_at, stale, source_event_id
            FROM memories
            WHERE stale = 0 AND (body LIKE ?1 OR IFNULL(title, '') LIKE ?1 OR tags_json LIKE ?1)
            ORDER BY created_at DESC
            LIMIT 100
            "#,
        )?;
        let rows = stmt.query_map(params![q], |r| {
            Ok(MemoryRecord {
                id: r.get(0)?,
                scope: r.get(1)?,
                workspace_path: r.get(2)?,
                tier: r.get(3)?,
                agent_id: r.get(4)?,
                title: r.get(5)?,
                body: r.get(6)?,
                tags_json: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
                stale: r.get::<_, i64>(10)? != 0,
                source_event_id: r.get(11)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn mark_memory_stale(&self, id: &str, stale: bool) -> Result<(), HubError> {
        let n = self.conn.execute(
            "UPDATE memories SET stale = ?1, updated_at = ?2 WHERE id = ?3",
            params![if stale { 1 } else { 0 }, Utc::now().to_rfc3339(), id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        Ok(())
    }

    pub fn delete_memory(&self, id: &str) -> Result<(), HubError> {
        let n = self
            .conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        Ok(())
    }

    /// Promote a memory to another tier, preserving provenance via `source_event_id`.
    pub fn promote_memory(&self, id: &str, to_tier: MemoryTier) -> Result<MemoryRecord, HubError> {
        let src = self
            .get_memory(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        if src.stale {
            return Err(HubError::Invalid("cannot promote a stale memory".into()));
        }
        let from = MemoryTier::parse(&src.tier)?;
        if from == to_tier {
            return Ok(src);
        }
        // Only allow short_term → episodic → semantic (no demotion).
        let allowed = matches!(
            (from, to_tier),
            (MemoryTier::ShortTerm, MemoryTier::Episodic)
                | (MemoryTier::ShortTerm, MemoryTier::Semantic)
                | (MemoryTier::Episodic, MemoryTier::Semantic)
        );
        if !allowed {
            return Err(HubError::Invalid(format!(
                "cannot promote {} → {}",
                from.as_str(),
                to_tier.as_str()
            )));
        }

        let new_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let title = src
            .title
            .clone()
            .unwrap_or_else(|| format!("promoted from {}", from.as_str()));
        let body = format!(
            "{}\n\n---\n_Promoted from `{}` (`{}`) at {}_\n",
            src.body,
            from.as_str(),
            id,
            now
        );

        self.conn.execute(
            r#"
            INSERT INTO memories(
                id, scope, workspace_path, tier, agent_id, title, body,
                tags_json, created_at, updated_at, stale, source_event_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11)
            "#,
            params![
                new_id,
                src.scope,
                src.workspace_path,
                to_tier.as_str(),
                src.agent_id,
                title,
                body,
                src.tags_json,
                now,
                now,
                id,
            ],
        )?;
        // Mark source stale so short-term lists stay lean; provenance remains queryable.
        self.mark_memory_stale(id, true)?;
        self.get_memory(&new_id)?
            .ok_or_else(|| HubError::NotFound(new_id))
    }

    /// Compact short-term memories: keep the newest `keep_newest`, promote the rest to episodic.
    pub fn compact_short_term(&self, keep_newest: usize) -> Result<CompactReport, HubError> {
        let mut short = self.list_memories(None, Some(MemoryTier::ShortTerm), None, false)?;
        // list is DESC by created_at; keep head, promote tail
        let mut promoted = 0usize;
        let mut skipped = 0usize;
        if short.len() <= keep_newest {
            return Ok(CompactReport {
                examined: short.len(),
                promoted: 0,
                kept: short.len(),
                skipped: 0,
            });
        }
        let to_promote: Vec<MemoryRecord> = short.split_off(keep_newest);
        let kept = short.len();
        for m in &to_promote {
            match self.promote_memory(&m.id, MemoryTier::Episodic) {
                Ok(_) => promoted += 1,
                Err(_) => skipped += 1,
            }
        }
        Ok(CompactReport {
            examined: kept + to_promote.len(),
            promoted,
            kept,
            skipped,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_message(
        &self,
        from_agent: &str,
        to_agent: &str,
        kind: MessageKind,
        body: &str,
        subject: Option<&str>,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<MessageRecord, HubError> {
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        self.upsert_agent(from_agent, from_agent)?;
        self.upsert_agent(to_agent, to_agent)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO messages(
                id, from_agent, to_agent, workspace_path, task_id,
                kind, status, subject, body, created_at, acked_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)
            "#,
            params![
                id,
                from_agent,
                to_agent,
                workspace_path,
                task_id,
                kind.as_str(),
                MessageStatus::Pending.as_str(),
                subject,
                body,
                now,
            ],
        )?;
        self.get_message(&id)?.ok_or_else(|| HubError::NotFound(id))
    }

    /// Fan out a team message while retaining one shared subject so clients
    /// can render the broadcast once instead of once per recipient.
    pub fn send_message_to_team(
        &self,
        from_agent: &str,
        kind: MessageKind,
        body: &str,
        subject: Option<&str>,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let subject = subject
            .map(str::to_string)
            .unwrap_or_else(|| format!("team:{}", Uuid::new_v4()));
        let recipients = self
            .list_team_members()?
            .into_iter()
            .filter(|agent| agent.id != from_agent && agent.id != "system")
            .map(|agent| agent.id)
            .collect::<Vec<_>>();
        if recipients.is_empty() {
            return Ok(vec![self.send_message(
                from_agent,
                "team",
                kind,
                body,
                Some(&subject),
                workspace_path,
                task_id,
            )?]);
        }
        recipients
            .into_iter()
            .map(|recipient| {
                self.send_message(
                    from_agent,
                    &recipient,
                    kind,
                    body,
                    Some(&subject),
                    workspace_path,
                    task_id,
                )
            })
            .collect()
    }

    pub fn is_team_member(&self, agent_id: &str) -> Result<bool, HubError> {
        Ok(self
            .conn
            .query_row(
                "SELECT team_member FROM agents WHERE id = ?1",
                params![agent_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .map(|value| value != 0)
            .unwrap_or(false))
    }

    pub fn is_session_member(&self, session_id: &str, agent_id: &str) -> Result<bool, HubError> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM work_session_members WHERE session_id = ?1 AND agent_id = ?2",
                params![session_id, agent_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// C11: enforce distinct task vs. wake semantics per recipient.
    ///
    /// "Currently present" is defined as: enrolled on the standing team
    /// (`agents.team_member`), and — when `session_id` is given — also a
    /// member of that session. There is no live-heartbeat signal in this
    /// schema yet, so presence is this durable enrollment state, not a
    /// point-in-time process check.
    ///
    /// - Task-tagged recipients who are not currently present are rejected:
    ///   no message is sent and no membership is mutated.
    /// - Wake-tagged recipients who are not yet a team member are enrolled
    ///   (and added to the session, if any) before delivery, then a durable
    ///   wake request is filed through the existing policy/budget/human-gate
    ///   path (`request_wake`) — a denial there does not undo the enrollment
    ///   or the message send, it only leaves the recipient unwoken.
    /// - Every recipient gets exactly one durable `tagged_send_outcomes` row,
    ///   whether accepted or rejected.
    #[allow(clippy::too_many_arguments)]
    pub fn send_tagged_message(
        &self,
        from_agent: &str,
        to: &[String],
        is_task: bool,
        is_wake: bool,
        body: &str,
        subject: Option<&str>,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<Vec<SendOutcome>, HubError> {
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        if !is_task && !is_wake {
            return Err(HubError::Invalid(
                "send_tagged_message requires at least one of task/wake".into(),
            ));
        }
        let subject = subject
            .map(str::to_string)
            .unwrap_or_else(|| format!("tagged:{}", Uuid::new_v4()));

        let mut recipients: Vec<String> = Vec::new();
        for id in to {
            if id != "system" && id != from_agent && !recipients.contains(id) {
                recipients.push(id.clone());
            }
        }
        if recipients.is_empty() {
            return Err(HubError::Invalid(
                "send_tagged_message requires at least one recipient".into(),
            ));
        }

        let mut outcomes = Vec::with_capacity(recipients.len());
        for recipient in recipients {
            let present = if let Some(session_id) = session_id {
                self.is_team_member(&recipient)?
                    && self.is_session_member(session_id, &recipient)?
            } else {
                self.is_team_member(&recipient)?
            };

            if is_task && !present {
                outcomes.push(self.record_send_outcome(
                    &subject,
                    from_agent,
                    &recipient,
                    is_task,
                    is_wake,
                    false,
                    false,
                    false,
                    Some("task target is not a current team/session member".into()),
                    None,
                )?);
                continue;
            }

            let mut enrolled = false;
            if is_wake && !self.is_team_member(&recipient)? {
                self.upsert_agent(&recipient, &recipient)?;
                self.set_team_member(&recipient, true)?;
                if let Some(session_id) = session_id {
                    self.add_work_session_member(session_id, &recipient)?;
                }
                enrolled = true;
            }

            let kind = if is_wake {
                MessageKind::Wake
            } else {
                MessageKind::Message
            };
            let message = self.send_message(
                from_agent,
                &recipient,
                kind,
                body,
                Some(&subject),
                workspace_path,
                task_id,
            )?;

            let mut wake_requested = false;
            let mut reason = None;
            if is_wake {
                let wake_reason = format!("tagged send: {subject}");
                match self.request_wake(&recipient, Some(&wake_reason), Some(&message.id), false) {
                    Ok(_) => wake_requested = true,
                    Err(error) => reason = Some(format!("wake request denied: {error}")),
                }
            }

            outcomes.push(self.record_send_outcome(
                &subject,
                from_agent,
                &recipient,
                is_task,
                is_wake,
                true,
                enrolled,
                wake_requested,
                reason,
                Some(message.id),
            )?);
        }

        Ok(outcomes)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_send_outcome(
        &self,
        subject: &str,
        from_agent: &str,
        to_agent: &str,
        is_task: bool,
        is_wake: bool,
        accepted: bool,
        enrolled: bool,
        wake_requested: bool,
        reason: Option<String>,
        message_id: Option<String>,
    ) -> Result<SendOutcome, HubError> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO tagged_send_outcomes(
                id, subject, from_agent, to_agent, is_task, is_wake,
                accepted, enrolled, wake_requested, reason, message_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                id,
                subject,
                from_agent,
                to_agent,
                is_task as i64,
                is_wake as i64,
                accepted as i64,
                enrolled as i64,
                wake_requested as i64,
                reason,
                message_id,
                created_at,
            ],
        )?;
        Ok(SendOutcome {
            id,
            subject: subject.to_string(),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.to_string(),
            is_task,
            is_wake,
            accepted,
            enrolled,
            wake_requested,
            reason,
            message_id,
            created_at,
        })
    }

    pub fn list_tagged_send_outcomes(&self, subject: &str) -> Result<Vec<SendOutcome>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, subject, from_agent, to_agent, is_task, is_wake,
                   accepted, enrolled, wake_requested, reason, message_id, created_at
            FROM tagged_send_outcomes
            WHERE subject = ?1
            ORDER BY created_at
            "#,
        )?;
        let rows = stmt.query_map(params![subject], |row| {
            Ok(SendOutcome {
                id: row.get(0)?,
                subject: row.get(1)?,
                from_agent: row.get(2)?,
                to_agent: row.get(3)?,
                is_task: row.get::<_, i64>(4)? != 0,
                is_wake: row.get::<_, i64>(5)? != 0,
                accepted: row.get::<_, i64>(6)? != 0,
                enrolled: row.get::<_, i64>(7)? != 0,
                wake_requested: row.get::<_, i64>(8)? != 0,
                reason: row.get(9)?,
                message_id: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_message(&self, id: &str) -> Result<Option<MessageRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, from_agent, to_agent, workspace_path, task_id, kind, status,
                   subject, body, created_at, acked_at
            FROM messages WHERE id = ?1
            "#,
        )?;
        let row = stmt
            .query_row(params![id], |r| {
                Ok(MessageRecord {
                    id: r.get(0)?,
                    from_agent: r.get(1)?,
                    to_agent: r.get(2)?,
                    workspace_path: r.get(3)?,
                    task_id: r.get(4)?,
                    kind: r.get(5)?,
                    status: r.get(6)?,
                    subject: r.get(7)?,
                    body: r.get(8)?,
                    created_at: r.get(9)?,
                    acked_at: r.get(10)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    pub fn list_messages(
        &self,
        to_agent: Option<&str>,
        status: Option<MessageStatus>,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let mut sql = String::from(
            r#"
            SELECT id, from_agent, to_agent, workspace_path, task_id, kind, status,
                   subject, body, created_at, acked_at
            FROM messages WHERE 1=1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(to) = to_agent {
            sql.push_str(" AND to_agent = ?");
            params_vec.push(Box::new(to.to_string()));
        }
        if let Some(st) = status {
            sql.push_str(" AND status = ?");
            params_vec.push(Box::new(st.as_str().to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 200");

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(MessageRecord {
                id: r.get(0)?,
                from_agent: r.get(1)?,
                to_agent: r.get(2)?,
                workspace_path: r.get(3)?,
                task_id: r.get(4)?,
                kind: r.get(5)?,
                status: r.get(6)?,
                subject: r.get(7)?,
                body: r.get(8)?,
                created_at: r.get(9)?,
                acked_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Lists one Slack-like channel without exposing similarly named channels.
    /// In addition to the canonical `channel:<name>` subject, a colon-delimited
    /// suffix is accepted for future thread/topic metadata.
    pub fn list_channel_messages(
        &self,
        channel: &str,
        limit: usize,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let channel = channel
            .trim()
            .strip_prefix("channel:")
            .unwrap_or(channel.trim());
        if channel.is_empty() {
            return Err(HubError::Invalid("channel must not be empty".into()));
        }
        let subject = format!("channel:{channel}");
        let subject_prefix = format!("{subject}:%");
        let limit = limit.clamp(1, 200) as i64;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, from_agent, to_agent, workspace_path, task_id, kind, status,
                   subject, body, created_at, acked_at
            FROM messages
            WHERE subject = ?1 OR subject LIKE ?2
            ORDER BY created_at DESC
            LIMIT ?3
            "#,
        )?;
        let rows = stmt.query_map(params![subject, subject_prefix, limit], |r| {
            Ok(MessageRecord {
                id: r.get(0)?,
                from_agent: r.get(1)?,
                to_agent: r.get(2)?,
                workspace_path: r.get(3)?,
                task_id: r.get(4)?,
                kind: r.get(5)?,
                status: r.get(6)?,
                subject: r.get(7)?,
                body: r.get(8)?,
                created_at: r.get(9)?,
                acked_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Resolves the unique shared memories referenced by one message body.
    /// Unknown or ambiguous short prefixes are omitted; callers can still use
    /// `parse_memory_references` to present an unresolved reference to users.
    pub fn list_message_memories(&self, message_id: &str) -> Result<Vec<MemoryRecord>, HubError> {
        let message = self
            .get_message(message_id)?
            .ok_or_else(|| HubError::NotFound(message_id.to_string()))?;
        let mut resolved = Vec::new();
        for reference in parse_memory_references(&message.body) {
            let exact = self.get_memory(&reference)?;
            if let Some(memory) = exact {
                resolved.push(memory);
                continue;
            }
            let mut stmt = self.conn.prepare(
                r#"
                SELECT id, scope, workspace_path, tier, agent_id, title, body,
                       tags_json, created_at, updated_at, stale, source_event_id
                FROM memories WHERE id LIKE ?1 ORDER BY id ASC LIMIT 2
                "#,
            )?;
            let matches = stmt
                .query_map(params![format!("{reference}%")], |r| {
                    Ok(MemoryRecord {
                        id: r.get(0)?,
                        scope: r.get(1)?,
                        workspace_path: r.get(2)?,
                        tier: r.get(3)?,
                        agent_id: r.get(4)?,
                        title: r.get(5)?,
                        body: r.get(6)?,
                        tags_json: r.get(7)?,
                        created_at: r.get(8)?,
                        updated_at: r.get(9)?,
                        stale: r.get::<_, i64>(10)? != 0,
                        source_event_id: r.get(11)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            if matches.len() == 1 {
                resolved.push(matches.into_iter().next().expect("one memory match"));
            }
        }
        Ok(resolved)
    }

    pub fn poll_messages(
        &self,
        to_agent: &str,
        mark_acked: bool,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let pending = self.list_messages(Some(to_agent), Some(MessageStatus::Pending))?;
        if mark_acked {
            let now = Utc::now().to_rfc3339();
            for m in &pending {
                self.conn.execute(
                    "UPDATE messages SET status = ?1, acked_at = ?2 WHERE id = ?3",
                    params![MessageStatus::Acked.as_str(), now, m.id],
                )?;
            }
        }
        if mark_acked {
            // re-fetch with acked status for returned records
            let mut out = Vec::with_capacity(pending.len());
            for m in pending {
                if let Some(updated) = self.get_message(&m.id)? {
                    out.push(updated);
                }
            }
            Ok(out)
        } else {
            Ok(pending)
        }
    }

    pub fn get_wake_policy(&self) -> Result<WakePolicy, HubError> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'wake_policy'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        match raw {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| HubError::Invalid(format!("wake_policy JSON corrupt: {e}"))),
            None => Ok(WakePolicy::default()),
        }
    }

    pub fn set_wake_policy(&self, policy: &WakePolicy) -> Result<(), HubError> {
        let json = serde_json::to_string(policy)
            .map_err(|e| HubError::Invalid(format!("wake_policy serialize: {e}")))?;
        self.conn.execute(
            r#"
            INSERT INTO meta(key, value) VALUES ('wake_policy', ?1)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![json],
        )?;
        Ok(())
    }

    /// Set (or reset) an agent's spend budget (C6). Resets `spent_units` to 0
    /// and clears any prior pause.
    pub fn set_agent_budget(
        &self,
        agent_id: &str,
        limit_units: f64,
    ) -> Result<BudgetStatus, HubError> {
        if limit_units <= 0.0 {
            return Err(HubError::Invalid("limit_units must be > 0".into()));
        }
        self.upsert_agent(agent_id, agent_id)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO agent_budgets(agent_id, limit_units, spent_units, paused, updated_at)
            VALUES (?1, ?2, 0, 0, ?3)
            ON CONFLICT(agent_id) DO UPDATE SET
                limit_units = excluded.limit_units,
                spent_units = 0,
                paused = 0,
                updated_at = excluded.updated_at
            "#,
            params![agent_id, limit_units, now],
        )?;
        Ok(self.get_budget(agent_id)?.expect("just inserted"))
    }

    pub fn get_budget(&self, agent_id: &str) -> Result<Option<BudgetStatus>, HubError> {
        self.conn
            .query_row(
                "SELECT agent_id, limit_units, spent_units, paused, updated_at \
                 FROM agent_budgets WHERE agent_id = ?1",
                params![agent_id],
                |r| {
                    Ok(BudgetStatus {
                        agent_id: r.get(0)?,
                        limit_units: r.get(1)?,
                        spent_units: r.get(2)?,
                        paused: r.get::<_, i64>(3)? != 0,
                        updated_at: r.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(HubError::from)
    }

    pub fn list_agent_metrics(&self) -> Result<Vec<AgentMetrics>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_id, lines_written, tokens_used, tokens_cached, provider_calls, output_chars, updated_at
             FROM agent_metrics ORDER BY agent_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AgentMetrics {
                agent_id: r.get(0)?,
                lines_written: r.get(1)?,
                tokens_used: r.get(2)?,
                tokens_cached: r.get(3)?,
                provider_calls: r.get(4)?,
                output_chars: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(HubError::from)
    }

    pub fn record_agent_metrics(
        &self,
        agent_id: &str,
        lines_written: i64,
        tokens_used: i64,
        tokens_cached: i64,
        output_chars: i64,
    ) -> Result<AgentMetrics, HubError> {
        if [lines_written, tokens_used, tokens_cached, output_chars]
            .iter()
            .any(|value| *value < 0)
        {
            return Err(HubError::Invalid(
                "metric increments must be non-negative".into(),
            ));
        }
        self.upsert_agent(agent_id, agent_id)?;
        self.conn.execute(
            "INSERT INTO agent_metrics(agent_id, lines_written, tokens_used, tokens_cached, provider_calls, output_chars, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6)
             ON CONFLICT(agent_id) DO UPDATE SET
               lines_written = lines_written + excluded.lines_written,
               tokens_used = tokens_used + excluded.tokens_used,
               tokens_cached = tokens_cached + excluded.tokens_cached,
               provider_calls = provider_calls + 1,
               output_chars = output_chars + excluded.output_chars,
               updated_at = excluded.updated_at",
            params![agent_id, lines_written, tokens_used, tokens_cached, output_chars, Utc::now().to_rfc3339()],
        )?;
        self.list_agent_metrics()?
            .into_iter()
            .find(|metric| metric.agent_id == agent_id)
            .ok_or_else(|| HubError::NotFound(agent_id.into()))
    }

    /// Record `amount` units of spend against `agent_id`. Returns the updated
    /// status; `paused` flips to true once `spent_units >= limit_units`, but
    /// this call alone does **not** write a handoff — call `pause_for_budget`
    /// when the caller is ready to hand off and stop (C6).
    pub fn record_budget_usage(
        &self,
        agent_id: &str,
        amount: f64,
    ) -> Result<BudgetStatus, HubError> {
        let budget = self
            .get_budget(agent_id)?
            .ok_or_else(|| HubError::NotFound(format!("no budget set for {agent_id}")))?;
        let spent = budget.spent_units + amount;
        let paused = budget.paused || spent >= budget.limit_units;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE agent_budgets SET spent_units = ?1, paused = ?2, updated_at = ?3 WHERE agent_id = ?4",
            params![spent, if paused { 1 } else { 0 }, now, agent_id],
        )?;
        Ok(self.get_budget(agent_id)?.expect("just updated"))
    }

    /// Atomically reserve budget before starting a provider call. Unlike
    /// `record_budget_usage`, this rejects a call that would exceed the limit.
    pub fn try_consume_budget(
        &self,
        agent_id: &str,
        amount: f64,
    ) -> Result<BudgetStatus, HubError> {
        if !amount.is_finite() || amount <= 0.0 {
            return Err(HubError::Invalid(
                "budget amount must be finite and > 0".into(),
            ));
        }
        let budget = self
            .get_budget(agent_id)?
            .ok_or_else(|| HubError::NotFound(format!("no budget set for {agent_id}")))?;
        if budget.paused {
            return Err(HubError::Invalid(format!("{agent_id} budget is paused")));
        }
        let next_spent = budget.spent_units + amount;
        if next_spent > budget.limit_units {
            let now = Utc::now().to_rfc3339();
            self.conn.execute(
                "UPDATE agent_budgets SET paused = 1, updated_at = ?1 WHERE agent_id = ?2",
                params![now, agent_id],
            )?;
            return Err(HubError::Invalid(format!(
                "budget exceeded for {agent_id}: {}/{} units",
                next_spent, budget.limit_units
            )));
        }
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE agent_budgets SET spent_units = ?1, paused = ?2, updated_at = ?3 WHERE agent_id = ?4",
            params![next_spent, if next_spent >= budget.limit_units { 1 } else { 0 }, now, agent_id],
        )?;
        self.get_budget(agent_id)?
            .ok_or_else(|| HubError::NotFound(agent_id.into()))
    }

    /// Clear a budget pause so the agent can receive wakes again (C6). A
    /// human/owner action, not something an agent should call on itself.
    pub fn resume_agent(&self, agent_id: &str) -> Result<BudgetStatus, HubError> {
        let n = self.conn.execute(
            "UPDATE agent_budgets SET paused = 0, updated_at = ?1 WHERE agent_id = ?2",
            params![Utc::now().to_rfc3339(), agent_id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(agent_id.into()));
        }
        Ok(self.get_budget(agent_id)?.expect("just updated"))
    }

    /// C6 exhaustion flow: mark `agent_id` paused (no further wakes accepted
    /// until `resume_agent`), write a durable Markdown handoff summary under
    /// `markdown/handoffs/`, and send a `Handoff` message to `delegate_to`
    /// (defaults to `"human"`) so the work is picked up rather than lost.
    #[allow(clippy::too_many_arguments)]
    pub fn pause_for_budget(
        &self,
        agent_id: &str,
        task_id: Option<&str>,
        objective: &str,
        completed: &str,
        missing: &str,
        delegate_to: Option<&str>,
    ) -> Result<BudgetPauseOutcome, HubError> {
        if self.get_budget(agent_id)?.is_none() {
            return Err(HubError::NotFound(format!("no budget set for {agent_id}")));
        }
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE agent_budgets SET paused = 1, updated_at = ?1 WHERE agent_id = ?2",
            params![now, agent_id],
        )?;
        let status = self.get_budget(agent_id)?.expect("just updated");

        let delegate = delegate_to.unwrap_or("human");
        let summary = format!(
            "# Budget-exhaustion handoff: {agent_id}\n\n\
             Generated: {now}\n\n\
             - agent: `{agent_id}`\n\
             - task: `{}`\n\
             - spent: {} / {} units\n\n\
             ## Objective\n\n{objective}\n\n\
             ## Completed\n\n{completed}\n\n\
             ## Missing / next steps\n\n{missing}\n\n\
             ## Delegated to\n\n`{delegate}`\n",
            task_id.unwrap_or("-"),
            status.spent_units,
            status.limit_units,
        );

        let handoffs_dir = self.data_dir.join("markdown").join("handoffs");
        fs::create_dir_all(&handoffs_dir)?;
        let stamp = now.replace([':', '.'], "-");
        let summary_path = handoffs_dir.join(format!("{stamp}-{agent_id}.md"));
        fs::write(&summary_path, &summary)?;

        let message = self.send_message(
            agent_id,
            delegate,
            MessageKind::Handoff,
            &summary,
            Some("budget exhausted: handoff"),
            None,
            task_id,
        )?;

        Ok(BudgetPauseOutcome {
            status,
            summary_path,
            handoff_message_id: message.id,
        })
    }

    /// Persist a cancellation/shutdown handoff so interrupted work is not lost.
    pub fn record_shutdown(
        &self,
        agent_id: &str,
        task_id: Option<&str>,
        objective: &str,
        reason: &str,
        delegate_to: Option<&str>,
    ) -> Result<ShutdownOutcome, HubError> {
        let delegate = delegate_to.unwrap_or("human");
        let now = Utc::now().to_rfc3339();
        let summary = format!(
            "# Shutdown handoff: {agent_id}\n\nGenerated: {now}\n\n- task: `{}`\n- delegated to: `{delegate}`\n- reason: {reason}\n\n## Objective\n\n{objective}\n",
            task_id.unwrap_or("-")
        );
        let handoffs_dir = self.data_dir.join("markdown").join("handoffs");
        fs::create_dir_all(&handoffs_dir)?;
        let stamp = now.replace([':', '.'], "-");
        let summary_path = handoffs_dir.join(format!("{stamp}-{agent_id}-shutdown.md"));
        fs::write(&summary_path, &summary)?;
        let message = self.send_message(
            agent_id,
            delegate,
            MessageKind::Handoff,
            &summary,
            Some("shutdown: handoff required"),
            None,
            task_id,
        )?;
        Ok(ShutdownOutcome {
            summary_path,
            handoff_message_id: message.id,
        })
    }

    pub fn request_wake(
        &self,
        target_agent: &str,
        reason: Option<&str>,
        message_id: Option<&str>,
        requires_human_gate: bool,
    ) -> Result<WakeRecord, HubError> {
        self.upsert_agent(target_agent, target_agent)?;

        if let Some(budget) = self.get_budget(target_agent)? {
            if budget.paused {
                return Err(HubError::Invalid(format!(
                    "{target_agent} is budget-paused ({}/{} units spent); \
                     resume_agent() required before new wakes are allowed",
                    budget.spent_units, budget.limit_units
                )));
            }
        }

        let policy = self.get_wake_policy()?;
        let mut requires_human_gate = requires_human_gate;
        if policy.default_requires_human_gate {
            requires_human_gate = true;
        }
        if !requires_human_gate && !policy.allow_auto_wake {
            return Err(HubError::Invalid(
                "wake policy forbids auto-wake without human gate".into(),
            ));
        }

        // A pending wake is an edge-triggered signal. Repeating the same
        // request must not create duplicate durable rows or side-channel files.
        let existing = self
            .conn
            .query_row(
                r#"
                SELECT id, target_agent, message_id, reason, status,
                       requires_human_gate, created_at
                FROM wake_requests
                WHERE target_agent = ?1
                  AND status = 'pending'
                  AND message_id IS ?2
                  AND reason IS ?3
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                params![target_agent, message_id, reason],
                |r| {
                    Ok(WakeRecord {
                        id: r.get(0)?,
                        target_agent: r.get(1)?,
                        message_id: r.get(2)?,
                        reason: r.get(3)?,
                        status: r.get(4)?,
                        requires_human_gate: r.get::<_, i64>(5)? != 0,
                        created_at: r.get(6)?,
                    })
                },
            )
            .optional()?;
        if let Some(wake) = existing {
            return Ok(wake);
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO wake_requests(
                id, target_agent, message_id, reason, status,
                requires_human_gate, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                id,
                target_agent,
                message_id,
                reason,
                WakeStatus::Pending.as_str(),
                if requires_human_gate { 1 } else { 0 },
                now,
            ],
        )?;

        // Ephemeral wake side-channel: drop a file agents/file-watchers can observe.
        let wake_path = self.data_dir.join("wake").join(format!("{id}.json"));
        let payload = serde_json::json!({
            "id": id,
            "target_agent": target_agent,
            "message_id": message_id,
            "reason": reason,
            "requires_human_gate": requires_human_gate,
            "created_at": now,
            "status": "pending"
        });
        fs::write(wake_path, serde_json::to_string_pretty(&payload).unwrap())?;

        Ok(WakeRecord {
            id,
            target_agent: target_agent.into(),
            message_id: message_id.map(|s| s.into()),
            reason: reason.map(|s| s.into()),
            status: WakeStatus::Pending.as_str().into(),
            requires_human_gate,
            created_at: now,
        })
    }

    pub fn set_wake_status(&self, id: &str, status: WakeStatus) -> Result<(), HubError> {
        let n = self.conn.execute(
            "UPDATE wake_requests SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        // Keep side-channel file in sync when present.
        let path = self.data_dir.join("wake").join(format!("{id}.json"));
        if path.exists() {
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) {
                    v["status"] = serde_json::json!(status.as_str());
                    let _ = fs::write(&path, serde_json::to_string_pretty(&v).unwrap_or_default());
                }
            }
            if status != WakeStatus::Pending {
                let _ = fs::remove_file(&path);
            }
        }
        Ok(())
    }

    pub fn list_wakes(
        &self,
        target_agent: Option<&str>,
        pending_only: bool,
    ) -> Result<Vec<WakeRecord>, HubError> {
        let mut sql = String::from(
            r#"
            SELECT id, target_agent, message_id, reason, status, requires_human_gate, created_at
            FROM wake_requests WHERE 1=1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(t) = target_agent {
            sql.push_str(" AND target_agent = ?");
            params_vec.push(Box::new(t.to_string()));
        }
        if pending_only {
            sql.push_str(" AND status = 'pending'");
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 100");

        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(WakeRecord {
                id: r.get(0)?,
                target_agent: r.get(1)?,
                message_id: r.get(2)?,
                reason: r.get(3)?,
                status: r.get(4)?,
                requires_human_gate: r.get::<_, i64>(5)? != 0,
                created_at: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Append to a private journal file (never written into shared SQLite tables).
    pub fn append_private_journal(&self, agent_id: &str, entry: &str) -> Result<PathBuf, HubError> {
        if entry.trim().is_empty() {
            return Err(HubError::Invalid("journal entry must not be empty".into()));
        }
        let dir = self.data_dir.join("journals").join(agent_id);
        fs::create_dir_all(&dir)?;
        let path = dir.join("journal.md");
        let stamp = Utc::now().to_rfc3339();
        let block = format!("\n## {stamp}\n\n{entry}\n");
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        f.write_all(block.as_bytes())?;
        Ok(path)
    }

    /// Permanently delete memories already marked stale (M5 retention).
    pub fn purge_stale_memories(&self) -> Result<usize, HubError> {
        let n = self
            .conn
            .execute("DELETE FROM memories WHERE stale = 1", [])?;
        Ok(n)
    }

    /// Mark short-term memories older than `max_age_hours` as stale (soft retention).
    pub fn mark_short_term_stale_older_than(&self, max_age_hours: i64) -> Result<usize, HubError> {
        if max_age_hours < 0 {
            return Err(HubError::Invalid("max_age_hours must be >= 0".into()));
        }
        let cutoff = (Utc::now() - chrono::Duration::hours(max_age_hours)).to_rfc3339();
        let n = self.conn.execute(
            r#"
            UPDATE memories
            SET stale = 1, updated_at = ?1
            WHERE tier = 'short_term' AND stale = 0 AND created_at < ?2
            "#,
            params![Utc::now().to_rfc3339(), cutoff],
        )?;
        Ok(n)
    }

    pub fn set_message_status(
        &self,
        id: &str,
        status: MessageStatus,
    ) -> Result<MessageRecord, HubError> {
        let acked = if matches!(status, MessageStatus::Acked | MessageStatus::Done) {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };
        let n = self.conn.execute(
            "UPDATE messages SET status = ?1, acked_at = COALESCE(?2, acked_at) WHERE id = ?3",
            params![status.as_str(), acked, id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        self.get_message(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    pub fn update_message_body(&self, id: &str, body: &str) -> Result<MessageRecord, HubError> {
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        let n = self.conn.execute(
            "UPDATE messages SET body = ?1 WHERE id = ?2",
            params![body, id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        self.get_message(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    pub fn delete_message(&self, id: &str) -> Result<(), HubError> {
        let n = self.conn.execute(
            "UPDATE messages SET status = ?1 WHERE id = ?2",
            params![MessageStatus::Cancelled.as_str(), id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        Ok(())
    }

    /// Finds every row sharing `message_id`'s broadcast group: an exact
    /// `subject` match when it carries a `:<uuid>` suffix (CA-107 team/channel
    /// fan-out, one row per recipient), otherwise the legacy grouping by
    /// `(from_agent, body, subject, created-at-to-the-second)` that the
    /// desktop chat also uses to collapse duplicate renders.
    fn broadcast_group_ids(&self, message_id: &str) -> Result<Vec<String>, HubError> {
        let anchor = self
            .get_message(message_id)?
            .ok_or_else(|| HubError::NotFound(message_id.into()))?;

        let has_uuid_suffix = anchor
            .subject
            .as_deref()
            .is_some_and(|subject| subject.matches(':').count() >= 2);

        if has_uuid_suffix {
            let subject = anchor.subject.as_deref().expect("checked above");
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM messages WHERE subject = ?1")?;
            let ids = stmt
                .query_map(params![subject], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(ids);
        }

        let created_second = anchor.created_at.get(..19).unwrap_or(&anchor.created_at);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id FROM messages
            WHERE from_agent = ?1 AND body = ?2
              AND subject IS ?3
              AND substr(created_at, 1, 19) = ?4
            "#,
        )?;
        let ids = stmt
            .query_map(
                params![
                    anchor.from_agent,
                    anchor.body,
                    anchor.subject,
                    created_second
                ],
                |r| r.get::<_, String>(0),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    /// Edit every copy of a team/channel broadcast (CA-106). `message_id` may
    /// be any one row from the group; all sibling copies are updated too.
    pub fn update_broadcast(
        &self,
        message_id: &str,
        body: &str,
    ) -> Result<Vec<MessageRecord>, HubError> {
        if body.trim().is_empty() {
            return Err(HubError::Invalid("message body must not be empty".into()));
        }
        let ids = self.broadcast_group_ids(message_id)?;
        ids.iter()
            .map(|id| self.update_message_body(id, body))
            .collect()
    }

    /// Delete (cancel) every copy of a team/channel broadcast (CA-106).
    /// Returns the number of rows affected.
    pub fn delete_broadcast(&self, message_id: &str) -> Result<usize, HubError> {
        let ids = self.broadcast_group_ids(message_id)?;
        for id in &ids {
            self.delete_message(id)?;
        }
        Ok(ids.len())
    }

    /// Export high-priority memories + handoffs as git-friendly Markdown under `markdown/`.
    pub fn export_markdown(&self, out_dir: Option<&Path>) -> Result<PathBuf, HubError> {
        let out = out_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.data_dir.join("markdown"));
        fs::create_dir_all(&out)?;

        let episodic = self.list_memories(None, Some(MemoryTier::Episodic), None, false)?;
        let semantic = self.list_memories(None, Some(MemoryTier::Semantic), None, false)?;
        let handoffs = self.list_messages(None, None)?;
        let handoffs: Vec<_> = handoffs
            .into_iter()
            .filter(|m| m.kind == MessageKind::Handoff.as_str())
            .collect();

        let mut body = String::from("# Coding-Assistants Shared Memory Export\n\n");
        body.push_str(&format!("Generated: {}\n\n", Utc::now().to_rfc3339()));

        body.push_str("## Episodic\n\n");
        for m in &episodic {
            body.push_str(&format!(
                "### {} ({})\n\n- id: `{}`\n- scope: {}\n- agent: {}\n\n{}\n\n",
                m.title.as_deref().unwrap_or("(untitled)"),
                m.created_at,
                m.id,
                m.scope,
                m.agent_id.as_deref().unwrap_or("-"),
                m.body
            ));
        }

        body.push_str("## Semantic\n\n");
        for m in &semantic {
            body.push_str(&format!(
                "### {} ({})\n\n- id: `{}`\n- scope: {}\n- agent: {}\n\n{}\n\n",
                m.title.as_deref().unwrap_or("(untitled)"),
                m.created_at,
                m.id,
                m.scope,
                m.agent_id.as_deref().unwrap_or("-"),
                m.body
            ));
        }

        body.push_str("## Handoffs\n\n");
        if handoffs.is_empty() {
            body.push_str("_No handoff messages._\n\n");
        }
        for m in &handoffs {
            body.push_str(&format!(
                "### {} → {} ({})\n\n- id: `{}`\n- status: {}\n- task: {}\n\n{}\n\n",
                m.from_agent,
                m.to_agent,
                m.created_at,
                m.id,
                m.status,
                m.task_id.as_deref().unwrap_or("-"),
                m.body
            ));
        }

        let path = out.join("shared_memory.md");
        fs::write(&path, body)?;
        Ok(path)
    }

    /// Group consecutive steps that share a `parallel_group` into stages.
    pub fn workflow_stages(steps: &[WorkflowStep]) -> Vec<Vec<usize>> {
        let mut stages: Vec<Vec<usize>> = Vec::new();
        let mut i = 0usize;
        while i < steps.len() {
            if let Some(ref g) = steps[i].parallel_group {
                if g.trim().is_empty() {
                    stages.push(vec![i]);
                    i += 1;
                    continue;
                }
                let mut group = vec![i];
                let mut j = i + 1;
                while j < steps.len()
                    && steps[j]
                        .parallel_group
                        .as_ref()
                        .map(|x| x == g)
                        .unwrap_or(false)
                {
                    group.push(j);
                    j += 1;
                }
                stages.push(group);
                i = j;
            } else {
                stages.push(vec![i]);
                i += 1;
            }
        }
        stages
    }

    fn map_task_row(r: &rusqlite::Row<'_>) -> Result<TaskRecord, rusqlite::Error> {
        let steps_json: String = r.get(5)?;
        let steps: Vec<WorkflowStep> = serde_json::from_str(&steps_json).unwrap_or_default();
        let attempts_json: String = r.get(9).unwrap_or_else(|_| "{}".into());
        let open_json: String = r.get(10).unwrap_or_else(|_| "[]".into());
        let pending_json: String = r.get(11).unwrap_or_else(|_| "[]".into());
        let max_parallel: i64 = r.get(12).unwrap_or(4);
        Ok(TaskRecord {
            id: r.get(0)?,
            title: r.get(1)?,
            workspace_path: r.get(2)?,
            status: r.get(3)?,
            step_index: r.get(4)?,
            steps,
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
            last_message_id: r.get(8)?,
            attempts: serde_json::from_str(&attempts_json).unwrap_or_default(),
            open_agents: serde_json::from_str(&open_json).unwrap_or_default(),
            pending_agents: serde_json::from_str(&pending_json).unwrap_or_default(),
            max_parallel: max_parallel.max(1) as u32,
            require_human_approval: r.get::<_, i64>(13).unwrap_or(1) > 0,
        })
    }

    pub fn create_task(
        &self,
        title: &str,
        workspace_path: Option<&str>,
        steps: &[WorkflowStep],
    ) -> Result<TaskRecord, HubError> {
        self.create_task_with_parallel(title, workspace_path, steps, 4, true)
    }

    pub fn create_task_with_parallel(
        &self,
        title: &str,
        workspace_path: Option<&str>,
        steps: &[WorkflowStep],
        max_parallel: u32,
        require_human_approval: bool,
    ) -> Result<TaskRecord, HubError> {
        if title.trim().is_empty() {
            return Err(HubError::Invalid("task title must not be empty".into()));
        }
        if steps.is_empty() {
            return Err(HubError::Invalid(
                "task needs at least one workflow step".into(),
            ));
        }
        let max_parallel = max_parallel.max(1);
        for (i, s) in steps.iter().enumerate() {
            if s.agent.trim().is_empty() {
                return Err(HubError::Invalid(format!("step {i}: agent required")));
            }
            if s.instruction.trim().is_empty() {
                return Err(HubError::Invalid(format!("step {i}: instruction required")));
            }
            self.upsert_agent(&s.agent, &s.agent)?;
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let steps_json = serde_json::to_string(steps)
            .map_err(|e| HubError::Invalid(format!("steps serialize: {e}")))?;
        self.conn.execute(
            r#"
            INSERT INTO tasks(
                id, title, workspace_path, status, step_index, steps_json,
                created_at, updated_at, last_message_id,
                attempts_json, open_agents_json, pending_agents_json, max_parallel,
                require_human_approval
            ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, NULL, '{}', '[]', '[]', ?8, ?9)
            "#,
            params![
                id,
                title,
                workspace_path,
                TaskStatus::Pending.as_str(),
                steps_json,
                now,
                now,
                max_parallel as i64,
                if require_human_approval { 1 } else { 0 },
            ],
        )?;
        self.get_task(&id)?.ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_task(&self, id: &str) -> Result<Option<TaskRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, title, workspace_path, status, step_index, steps_json,
                   created_at, updated_at, last_message_id,
                   attempts_json, open_agents_json, pending_agents_json, max_parallel,
                   require_human_approval
            FROM tasks WHERE id = ?1
            "#,
        )?;
        let row = stmt.query_row(params![id], Self::map_task_row).optional()?;
        Ok(row)
    }

    pub fn list_tasks(&self, status: Option<TaskStatus>) -> Result<Vec<TaskRecord>, HubError> {
        let mut sql = String::from(
            r#"
            SELECT id, title, workspace_path, status, step_index, steps_json,
                   created_at, updated_at, last_message_id,
                   attempts_json, open_agents_json, pending_agents_json, max_parallel,
                   require_human_approval
            FROM tasks WHERE 1=1
            "#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(st) = status {
            sql.push_str(" AND status = ?");
            params_vec.push(Box::new(st.as_str().to_string()));
        }
        sql.push_str(" ORDER BY updated_at DESC LIMIT 100");
        let mut stmt = self.conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), Self::map_task_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn dispatch_step(
        &self,
        task_id: &str,
        task: &TaskRecord,
        step: &WorkflowStep,
        from_agent: &str,
        note: Option<&str>,
        stage_label: &str,
    ) -> Result<String, HubError> {
        let body = if let Some(n) = note {
            format!("{}\n\n---\nPrior note: {}", step.instruction, n)
        } else {
            step.instruction.clone()
        };
        let subject = Some(format!("[{}] {}", stage_label, task.title));
        let msg = self.send_message(
            from_agent,
            &step.agent,
            MessageKind::Handoff,
            &body,
            subject.as_deref(),
            task.workspace_path.as_deref(),
            Some(task_id),
        )?;
        let _wake = self.request_wake(
            &step.agent,
            Some(&format!("task {task_id} {stage_label}")),
            Some(&msg.id),
            task.require_human_approval,
        )?;
        Ok(msg.id)
    }

    #[allow(clippy::too_many_arguments)]
    fn persist_task_runtime(
        &self,
        id: &str,
        status: &str,
        stage_index: i64,
        last_message_id: Option<&str>,
        attempts: &std::collections::HashMap<String, u32>,
        open_agents: &[String],
        pending_agents: &[String],
    ) -> Result<(), HubError> {
        let now = Utc::now().to_rfc3339();
        let attempts_json = serde_json::to_string(attempts).unwrap_or_else(|_| "{}".into());
        let open_json = serde_json::to_string(open_agents).unwrap_or_else(|_| "[]".into());
        let pending_json = serde_json::to_string(pending_agents).unwrap_or_else(|_| "[]".into());
        self.conn.execute(
            r#"
            UPDATE tasks
            SET status = ?1, step_index = ?2, updated_at = ?3, last_message_id = ?4,
                attempts_json = ?5, open_agents_json = ?6, pending_agents_json = ?7
            WHERE id = ?8
            "#,
            params![
                status,
                stage_index,
                now,
                last_message_id,
                attempts_json,
                open_json,
                pending_json,
                id,
            ],
        )?;
        Ok(())
    }

    /// Advance to the next **stage** (sequential step or parallel group).
    /// Fails if the current stage still has open parallel agents.
    pub fn advance_task(
        &self,
        id: &str,
        from_agent: Option<&str>,
        note: Option<&str>,
    ) -> Result<TaskRecord, HubError> {
        let task = self
            .get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        let status = TaskStatus::parse(&task.status)?;
        if status.is_terminal() {
            return Err(HubError::Invalid(format!(
                "task is already {}",
                task.status
            )));
        }
        if !task.open_agents.is_empty() {
            return Err(HubError::Invalid(format!(
                "parallel stage still open for agents: {}",
                task.open_agents.join(", ")
            )));
        }
        if !task.pending_agents.is_empty() {
            return Err(HubError::Invalid(format!(
                "parallel stage still has queued agents: {}",
                task.pending_agents.join(", ")
            )));
        }

        let stages = Self::workflow_stages(&task.steps);
        if stages.is_empty() {
            return Err(HubError::Invalid("task has no steps".into()));
        }

        let next_stage = if status == TaskStatus::Pending {
            0i64
        } else {
            let ni = task.step_index + 1;
            if ni >= stages.len() as i64 {
                self.persist_task_runtime(
                    id,
                    TaskStatus::Done.as_str(),
                    task.step_index,
                    task.last_message_id.as_deref(),
                    &task.attempts,
                    &[],
                    &[],
                )?;
                return self
                    .get_task(id)?
                    .ok_or_else(|| HubError::NotFound(id.into()));
            }
            ni
        };

        self.activate_stage(id, &task, next_stage, from_agent.unwrap_or("human"), note)
    }

    fn activate_stage(
        &self,
        id: &str,
        task: &TaskRecord,
        stage_index: i64,
        from_agent: &str,
        note: Option<&str>,
    ) -> Result<TaskRecord, HubError> {
        let stages = Self::workflow_stages(&task.steps);
        let idxs = &stages[stage_index as usize];
        let stage_label = format!("{}/{}", stage_index + 1, stages.len());
        let mut attempts = task.attempts.clone();
        *attempts.entry(stage_index.to_string()).or_insert(0) += 1;

        let mut last_msg: Option<String> = None;
        let mut open: Vec<String> = Vec::new();
        let mut pending: Vec<String> = Vec::new();

        if idxs.len() == 1 {
            let step = &task.steps[idxs[0]];
            let msg_id = self.dispatch_step(id, task, step, from_agent, note, &stage_label)?;
            last_msg = Some(msg_id);
        } else {
            let mut agents_to_run: Vec<usize> = idxs.clone();
            let cap = task.max_parallel as usize;
            let take = cap.min(agents_to_run.len());
            let wake_now: Vec<usize> = agents_to_run.drain(..take).collect();
            for si in &wake_now {
                let step = &task.steps[*si];
                let msg_id = self.dispatch_step(
                    id,
                    task,
                    step,
                    from_agent,
                    note,
                    &format!("{stage_label}/{}", step.agent),
                )?;
                last_msg = Some(msg_id);
                open.push(step.agent.clone());
            }
            for si in agents_to_run {
                pending.push(task.steps[si].agent.clone());
            }
        }

        self.persist_task_runtime(
            id,
            TaskStatus::Running.as_str(),
            stage_index,
            last_msg.as_deref(),
            &attempts,
            &open,
            &pending,
        )?;
        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    /// Mark one agent finished in the current parallel stage.
    pub fn complete_parallel_member(
        &self,
        id: &str,
        agent: &str,
        note: Option<&str>,
    ) -> Result<TaskRecord, HubError> {
        let task = self
            .get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        if task.status != TaskStatus::Running.as_str() {
            return Err(HubError::Invalid("task is not running".into()));
        }
        if task.open_agents.is_empty() && task.pending_agents.is_empty() {
            return Err(HubError::Invalid(
                "no open parallel stage (use advance_task for sequential steps)".into(),
            ));
        }
        if !task.open_agents.iter().any(|a| a == agent) {
            return Err(HubError::Invalid(format!(
                "agent '{agent}' is not in the open parallel set"
            )));
        }

        let mut open: Vec<String> = task
            .open_agents
            .iter()
            .filter(|a| *a != agent)
            .cloned()
            .collect();
        let mut pending = task.pending_agents.clone();
        let mut last_msg = task.last_message_id.clone();
        let stage_index = task.step_index;
        let max_parallel = task.max_parallel;
        let attempts = task.attempts.clone();

        while open.len() < max_parallel as usize && !pending.is_empty() {
            let next_agent = pending.remove(0);
            let stages = Self::workflow_stages(&task.steps);
            let idxs = &stages[stage_index as usize];
            let step = idxs
                .iter()
                .map(|i| &task.steps[*i])
                .find(|s| s.agent == next_agent)
                .ok_or_else(|| {
                    HubError::Invalid(format!("pending agent '{next_agent}' not in stage"))
                })?;
            let msg_id = self.dispatch_step(
                id,
                &task,
                step,
                agent,
                note,
                &format!("{}/{}", stage_index + 1, next_agent),
            )?;
            last_msg = Some(msg_id);
            open.push(next_agent);
        }

        self.persist_task_runtime(
            id,
            TaskStatus::Running.as_str(),
            stage_index,
            last_msg.as_deref(),
            &attempts,
            &open,
            &pending,
        )?;
        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    /// Re-dispatch the current stage (honours max_retries on the stage).
    pub fn retry_task(
        &self,
        id: &str,
        from_agent: Option<&str>,
        note: Option<&str>,
    ) -> Result<TaskRecord, HubError> {
        let task = self
            .get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        if task.status != TaskStatus::Running.as_str() {
            return Err(HubError::Invalid("can only retry a running task".into()));
        }
        let stages = Self::workflow_stages(&task.steps);
        let stage_index = task.step_index as usize;
        if stage_index >= stages.len() {
            return Err(HubError::Invalid("invalid stage index".into()));
        }
        let idxs = &stages[stage_index];
        let max_retries = idxs
            .iter()
            .map(|i| task.steps[*i].max_retries)
            .max()
            .unwrap_or(0);
        let attempts = *task.attempts.get(&stage_index.to_string()).unwrap_or(&1);
        // After first dispatch attempts=1. With max_retries=1, one more dispatch is allowed
        // (activate will bump to 2). Block when attempts already exceeds max_retries.
        if attempts > max_retries {
            self.persist_task_runtime(
                id,
                TaskStatus::Failed.as_str(),
                task.step_index,
                task.last_message_id.as_deref(),
                &task.attempts,
                &[],
                &[],
            )?;
            return Err(HubError::Invalid(format!(
                "max_retries ({max_retries}) exhausted for stage {stage_index} (attempts={attempts}); task marked failed"
            )));
        }

        self.persist_task_runtime(
            id,
            TaskStatus::Running.as_str(),
            task.step_index,
            task.last_message_id.as_deref(),
            &task.attempts,
            &[],
            &[],
        )?;
        let task = self
            .get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        self.activate_stage(
            id,
            &task,
            task.step_index,
            from_agent.unwrap_or("human"),
            note.or(Some("retry")),
        )
    }

    pub fn cancel_task(&self, id: &str) -> Result<TaskRecord, HubError> {
        let n = self.conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2, open_agents_json = '[]', pending_agents_json = '[]' WHERE id = ?3",
            params![
                TaskStatus::Cancelled.as_str(),
                Utc::now().to_rfc3339(),
                id
            ],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        self.get_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    /// Export as in `export_markdown`, then `git add` + `git commit` the
    /// result if `out_dir` (or the default `markdown/` dir) sits inside a Git
    /// work tree (M3). Never errors solely because Git is unavailable, the
    /// directory isn't a repo, or there is nothing new to commit — those are
    /// reported via `GitExportOutcome`, not `Err`, so callers who don't care
    /// about Git can ignore the field.
    pub fn export_markdown_git(
        &self,
        out_dir: Option<&Path>,
        message: Option<&str>,
    ) -> Result<GitExportOutcome, HubError> {
        let path = self.export_markdown(out_dir)?;
        let dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.data_dir.join("markdown"));

        let in_work_tree = Command::new("git")
            .args([
                "-C",
                &dir.to_string_lossy(),
                "rev-parse",
                "--is-inside-work-tree",
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !in_work_tree {
            return Ok(GitExportOutcome {
                path,
                committed: false,
                detail: "not inside a Git work tree; commit skipped".into(),
            });
        }

        let add = Command::new("git")
            .args(["-C", &dir.to_string_lossy(), "add", "--"])
            .arg(&path)
            .output()?;
        if !add.status.success() {
            return Ok(GitExportOutcome {
                path,
                committed: false,
                detail: format!("git add failed: {}", String::from_utf8_lossy(&add.stderr)),
            });
        }

        let msg = message
            .map(str::to_string)
            .unwrap_or_else(|| "chore(hub): update shared memory export".to_string());
        let commit = Command::new("git")
            .args(["-C", &dir.to_string_lossy(), "commit", "-m", &msg, "--"])
            .arg(&path)
            .output()?;
        if commit.status.success() {
            Ok(GitExportOutcome {
                path,
                committed: true,
                detail: "committed".into(),
            })
        } else {
            // Commonly "nothing to commit" when the export is unchanged.
            Ok(GitExportOutcome {
                path,
                committed: false,
                detail: String::from_utf8_lossy(&commit.stderr).trim().to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn memory_message_wake_roundtrip() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let mem = store
            .write_memory(
                MemoryTier::Episodic,
                MemoryScope::Global,
                Some("grok"),
                None,
                Some("first handoff"),
                "Grok left a note for Claude about the hub schema.",
                &["hub".into(), "schema".into()],
            )
            .unwrap();
        assert_eq!(mem.tier, "episodic");

        let found = store.search_memories("hub schema").unwrap();
        assert_eq!(found.len(), 1);

        let msg = store
            .send_message(
                "grok",
                "claude",
                MessageKind::Handoff,
                "Please review the hub schema.",
                Some("schema review"),
                None,
                Some("task-1"),
            )
            .unwrap();
        assert_eq!(msg.status, "pending");

        let team_messages = store
            .send_message_to_team(
                "grok",
                MessageKind::Message,
                "A shared team update.",
                None,
                None,
                None,
            )
            .unwrap();
        assert!(team_messages
            .iter()
            .all(|message| message.from_agent == "grok"));
        assert!(team_messages
            .iter()
            .all(|message| message.to_agent != "grok"));
        assert!(team_messages.iter().all(|message| message
            .subject
            .as_deref()
            .unwrap()
            .starts_with("team:")));

        let polled = store.poll_messages("claude", true).unwrap();
        assert_eq!(polled.len(), 2);
        assert!(polled
            .iter()
            .any(|message| message.id == msg.id && message.status == "acked"));

        let wake = store
            .request_wake("claude", Some("schema ready"), Some(&msg.id), true)
            .unwrap();
        let duplicate = store
            .request_wake("claude", Some("schema ready"), Some(&msg.id), true)
            .unwrap();
        assert!(wake.requires_human_gate);
        assert_eq!(wake.id, duplicate.id);
        assert_eq!(store.list_wakes(Some("claude"), true).unwrap().len(), 1);
        assert!(dir
            .path()
            .join("wake")
            .join(format!("{}.json", wake.id))
            .exists());

        let journal = store
            .append_private_journal("grok", "Private note: do not share.")
            .unwrap();
        assert!(journal.exists());
        // private journal must not appear in shared memory tables
        assert!(store.search_memories("do not share").unwrap().is_empty());

        let export = store.export_markdown(None).unwrap();
        let text = fs::read_to_string(export).unwrap();
        assert!(text.contains("first handoff"));
    }

    #[test]
    fn audit_events_are_reviewable_and_hash_chained() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let root = dir.path().join("watched");
        fs::create_dir_all(&root).unwrap();

        let first = store
            .record_audit_event(
                &root,
                Path::new("journals/chat.md"),
                "modified",
                r#"{"pid":123,"attribution":"test"}"#,
                Some("abc"),
            )
            .unwrap();
        let second = store
            .record_audit_event(
                &root,
                Path::new("journals/chat.md"),
                "modified",
                r#"{"pid":123,"attribution":"test"}"#,
                Some("def"),
            )
            .unwrap();
        assert_eq!(
            second.previous_hash.as_deref(),
            Some(first.event_hash.as_str())
        );
        assert_eq!(store.verify_audit_chain().unwrap(), 2);
        assert_eq!(store.list_audit_events(true).unwrap().len(), 2);
        store.set_audit_status(&first.id, "approved").unwrap();
        assert_eq!(store.list_audit_events(true).unwrap().len(), 1);
        assert!(store.set_audit_status("missing", "approved").is_err());
    }

    #[test]
    fn promote_and_compact_short_term() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        for i in 0..5 {
            store
                .write_memory(
                    MemoryTier::ShortTerm,
                    MemoryScope::Global,
                    Some("grok"),
                    None,
                    Some(&format!("note-{i}")),
                    &format!("short body {i}"),
                    &[],
                )
                .unwrap();
            // tiny delay so created_at ordering is stable across platforms
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let report = store.compact_short_term(2).unwrap();
        assert_eq!(report.promoted, 3);
        assert_eq!(report.kept, 2);

        let short = store
            .list_memories(None, Some(MemoryTier::ShortTerm), None, false)
            .unwrap();
        assert_eq!(short.len(), 2);

        let episodic = store
            .list_memories(None, Some(MemoryTier::Episodic), None, false)
            .unwrap();
        assert_eq!(episodic.len(), 3);
        assert!(episodic[0].body.contains("Promoted from"));

        let one = store
            .write_memory(
                MemoryTier::Episodic,
                MemoryScope::Global,
                Some("claude"),
                None,
                Some("decision"),
                "Use SQLite as source of truth.",
                &[],
            )
            .unwrap();
        let semantic = store.promote_memory(&one.id, MemoryTier::Semantic).unwrap();
        assert_eq!(semantic.tier, "semantic");
        store.delete_memory(&semantic.id).unwrap();
        assert!(store.get_memory(&semantic.id).unwrap().is_none());
    }

    #[test]
    fn wake_policy_and_retention() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        // Default policy forces human gate even when caller passes false.
        let wake = store
            .request_wake("claude", Some("need review"), None, false)
            .unwrap();
        assert!(wake.requires_human_gate);

        store
            .set_wake_policy(&WakePolicy {
                default_requires_human_gate: false,
                allow_auto_wake: false,
            })
            .unwrap();
        let err = store
            .request_wake("claude", Some("auto"), None, false)
            .unwrap_err();
        assert!(err.to_string().contains("forbids auto-wake"));

        store
            .set_wake_policy(&WakePolicy {
                default_requires_human_gate: false,
                allow_auto_wake: true,
            })
            .unwrap();
        let auto = store
            .request_wake("gemini", Some("auto ok"), None, false)
            .unwrap();
        assert!(!auto.requires_human_gate);
        store
            .set_wake_status(&auto.id, WakeStatus::Delivered)
            .unwrap();
        assert_eq!(store.list_wakes(Some("gemini"), true).unwrap().len(), 0);

        let m = store
            .write_memory(
                MemoryTier::ShortTerm,
                MemoryScope::Global,
                Some("grok"),
                None,
                Some("old"),
                "stale me",
                &[],
            )
            .unwrap();
        store.mark_memory_stale(&m.id, true).unwrap();
        assert_eq!(store.purge_stale_memories().unwrap(), 1);
        assert!(store.get_memory(&m.id).unwrap().is_none());
    }

    #[test]
    fn m6_cross_agent_handoff_acceptance_flow() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let handoff = store
            .send_message(
                "grok",
                "claude",
                MessageKind::Handoff,
                "The shared Hub slice is ready for review.",
                Some("m6-acceptance"),
                None,
                Some("m6-acceptance"),
            )
            .unwrap();
        let memory = store
            .write_memory_with_source(
                MemoryTier::Episodic,
                MemoryScope::Global,
                Some("grok"),
                Some("m6-acceptance"),
                Some("Hub handoff"),
                "Review the Hub implementation and verify wake delivery.",
                &["handoff".into(), "acceptance".into()],
                Some("m6-acceptance"),
            )
            .unwrap();
        assert_eq!(memory.source_event_id.as_deref(), Some("m6-acceptance"));

        let inbox = store.poll_messages("claude", true).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].id, handoff.id);
        assert_eq!(inbox[0].status, "acked");

        let wake = store
            .request_wake("claude", Some("handoff ready"), Some(&handoff.id), true)
            .unwrap();
        let duplicate = store
            .request_wake("claude", Some("handoff ready"), Some(&handoff.id), true)
            .unwrap();
        assert_eq!(wake.id, duplicate.id);
        store
            .set_wake_status(&wake.id, WakeStatus::Delivered)
            .unwrap();
        assert!(store.list_wakes(Some("claude"), true).unwrap().is_empty());

        let export = store.export_markdown(None).unwrap();
        let text = fs::read_to_string(export).unwrap();
        assert!(text.contains("Hub handoff"));
        assert!(text.contains("The shared Hub slice is ready for review."));

        // CA-103: Slack-style channel communication across multiple agent
        // roles must stay isolated per channel at the data layer, since the
        // desktop SlackChatPanel filters purely by `subject == "channel:<id>"`
        // over the full `list_messages` result — a leak here would be
        // invisible in the UI but would surface as one channel seeing
        // another channel's traffic.
        store
            .send_message(
                "grok",
                "claude",
                MessageKind::Message,
                "general channel: build is green",
                Some("channel:general"),
                None,
                None,
            )
            .unwrap();
        store
            .send_message(
                "gemini",
                "claude",
                MessageKind::Message,
                &format!("team-coordination channel: see memory:{}", memory.id),
                Some("channel:team-coordination"),
                None,
                None,
            )
            .unwrap();
        store
            .send_message(
                "grok",
                "human",
                MessageKind::Message,
                "DM: quick question about the Hub schema",
                None,
                None,
                None,
            )
            .unwrap();

        let all = store.list_messages(None, None).unwrap();
        let general: Vec<_> = all
            .iter()
            .filter(|m| m.subject.as_deref() == Some("channel:general"))
            .collect();
        let team_coord: Vec<_> = all
            .iter()
            .filter(|m| m.subject.as_deref() == Some("channel:team-coordination"))
            .collect();
        assert_eq!(general.len(), 1);
        assert_eq!(general[0].body, "general channel: build is green");
        assert_eq!(team_coord.len(), 1);
        assert!(team_coord[0].body.contains(&memory.id));
        assert!(general.iter().all(|m| m.body != team_coord[0].body));

        let dm: Vec<_> = all
            .iter()
            .filter(|m| m.from_agent == "grok" && m.to_agent == "human")
            .collect();
        assert_eq!(dm.len(), 1);
        assert!(dm[0].subject.is_none());

        // Memory-link retrieval: a channel message can reference a memory
        // id inline (as the desktop drawer's "attach memory" action does);
        // the linked memory must still be reachable through the normal
        // search path used by both the CLI and the Tauri hub commands.
        let linked = store
            .search_memories("Review the Hub implementation")
            .unwrap();
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].id, memory.id);
        assert!(team_coord[0].body.contains(&linked[0].id));
    }

    #[test]
    fn team_broadcast_uses_enrolled_roster_and_includes_human() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        store.upsert_agent("process:1", "Codex · PID 1").unwrap();
        store.upsert_agent("a2a-peer", "a2a-peer").unwrap();

        let team = store
            .send_message_to_team(
                "grok",
                MessageKind::Message,
                "M6 roster check",
                Some("channel:general"),
                None,
                None,
            )
            .unwrap();
        let recipients: Vec<&str> = team.iter().map(|m| m.to_agent.as_str()).collect();
        assert!(recipients.contains(&"human"), "{recipients:?}");
        assert!(recipients.contains(&"claude"), "{recipients:?}");
        assert!(recipients.contains(&"chat"), "{recipients:?}");
        assert!(recipients.contains(&"gemini"), "{recipients:?}");
        assert!(!recipients.contains(&"grok"), "{recipients:?}");
        assert!(!recipients.contains(&"process:1"), "{recipients:?}");
        assert!(!recipients.contains(&"a2a-peer"), "{recipients:?}");
        assert!(!recipients.contains(&"ollama"), "{recipients:?}");
        assert!(!recipients.contains(&"system"), "{recipients:?}");
        assert!(team
            .iter()
            .all(|m| m.subject.as_deref() == Some("channel:general")));

        store.set_team_member("ollama", true).unwrap();
        store.set_team_member("claude", false).unwrap();
        let updated = store
            .send_message_to_team(
                "grok",
                MessageKind::Message,
                "roster after enroll change",
                None,
                None,
                None,
            )
            .unwrap();
        let recipients: Vec<&str> = updated.iter().map(|m| m.to_agent.as_str()).collect();
        assert!(recipients.contains(&"ollama"), "{recipients:?}");
        assert!(!recipients.contains(&"claude"), "{recipients:?}");
        assert!(recipients.contains(&"human"), "{recipients:?}");

        store.set_team_member("claude", true).unwrap();
        store.set_team_member("ollama", false).unwrap();
        let wakes = store
            .request_team_wakes("human", Some("Slack #general"), Some("msg-team-1"), false)
            .unwrap();
        let woke: Vec<&str> = wakes.iter().map(|w| w.target_agent.as_str()).collect();
        assert!(woke.contains(&"claude"), "{woke:?}");
        assert!(woke.contains(&"chat"), "{woke:?}");
        assert!(woke.contains(&"gemini"), "{woke:?}");
        assert!(woke.contains(&"grok"), "{woke:?}");
        assert!(!woke.contains(&"human"), "{woke:?}");
        assert!(!woke.contains(&"ollama"), "{woke:?}");
        assert!(!woke.contains(&"process:1"), "{woke:?}");
        assert_eq!(woke.len(), 4, "{woke:?}");
    }

    #[test]
    fn ca106_edit_and_delete_a_team_broadcast_updates_every_copy() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        store.set_team_member("claude", true).unwrap();
        store.set_team_member("grok", true).unwrap();
        store.set_team_member("chat", true).unwrap();

        let subject = "channel:general:11111111-1111-1111-1111-111111111111";
        let posted = store
            .send_message_to_team(
                "human",
                MessageKind::Message,
                "hi",
                Some(subject),
                None,
                None,
            )
            .unwrap();
        assert!(posted.len() >= 3, "{posted:?}");

        // Editing any one copy of the broadcast must update every sibling
        // row sharing the subject, not just the row that happened to render.
        let edited = store
            .update_broadcast(&posted[0].id, "hi (edited)")
            .unwrap();
        assert_eq!(edited.len(), posted.len());
        assert!(edited.iter().all(|m| m.body == "hi (edited)"));
        for original in &posted {
            let refreshed = store.get_message(&original.id).unwrap().unwrap();
            assert_eq!(refreshed.body, "hi (edited)");
        }

        let deleted_count = store.delete_broadcast(&posted[1].id).unwrap();
        assert_eq!(deleted_count, posted.len());
        for original in &posted {
            let refreshed = store.get_message(&original.id).unwrap().unwrap();
            assert_eq!(refreshed.status, "cancelled");
        }
    }

    #[test]
    fn ca106_edit_and_delete_a_legacy_broadcast_groups_by_sender_body_and_second() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        // Legacy posts share the exact `channel:<name>` subject with no
        // per-broadcast uuid suffix; grouping falls back to
        // (from_agent, body, subject, created-at-to-the-second).
        let a = store
            .send_message(
                "grok",
                "claude",
                MessageKind::Message,
                "legacy note",
                Some("channel:general"),
                None,
                None,
            )
            .unwrap();
        let b = store
            .send_message(
                "grok",
                "chat",
                MessageKind::Message,
                "legacy note",
                Some("channel:general"),
                None,
                None,
            )
            .unwrap();
        // A distinct send (different body) must not be swept into the group.
        let unrelated = store
            .send_message(
                "grok",
                "gemini",
                MessageKind::Message,
                "unrelated note",
                Some("channel:general"),
                None,
                None,
            )
            .unwrap();

        let deleted_count = store.delete_broadcast(&a.id).unwrap();
        assert_eq!(deleted_count, 2);
        assert_eq!(
            store.get_message(&a.id).unwrap().unwrap().status,
            "cancelled"
        );
        assert_eq!(
            store.get_message(&b.id).unwrap().unwrap().status,
            "cancelled"
        );
        assert_eq!(
            store.get_message(&unrelated.id).unwrap().unwrap().status,
            "pending"
        );
    }

    #[test]
    fn m3_export_markdown_git_commits_inside_a_work_tree() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        // No repo yet: commit is skipped, not an error.
        let outcome = store.export_markdown_git(None, None).unwrap();
        assert!(!outcome.committed);
        assert!(outcome.path.exists());

        let md_dir = dir.path().join("markdown");
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&md_dir)
                .args(args)
                .output()
                .expect("git available for test");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        git(&["init", "-q"]);
        git(&["config", "user.email", "hub-test@example.com"]);
        git(&["config", "user.name", "Hub Test"]);

        store
            .write_memory(
                MemoryTier::Episodic,
                MemoryScope::Global,
                Some("claude"),
                None,
                Some("git export"),
                "Verify markdown export auto-commits inside a work tree.",
                &[],
            )
            .unwrap();

        let outcome = store
            .export_markdown_git(None, Some("chore(hub): test export"))
            .unwrap();
        assert!(
            outcome.committed,
            "expected a commit, got: {}",
            outcome.detail
        );

        let log = Command::new("git")
            .arg("-C")
            .arg(&md_dir)
            .args(["log", "-1", "--pretty=%s"])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&log.stdout).trim(),
            "chore(hub): test export"
        );

        // The export body always rewrites its "Generated:" timestamp, so a
        // second call still has a diff and commits again rather than being a
        // no-op; that's the git-tracked-history behavior M3 asks for.
        let second = store
            .export_markdown_git(None, Some("chore(hub): test export 2"))
            .unwrap();
        assert!(
            second.committed,
            "expected a second commit, got: {}",
            second.detail
        );
    }

    #[test]
    fn c5_sequential_task_advance_plan_code_review() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let steps = vec![
            WorkflowStep {
                agent: "grok".into(),
                role: Some("Planner".into()),
                instruction: "Plan the dual-mode pathing fix.".into(),
                max_retries: 0,
                parallel_group: None,
            },
            WorkflowStep {
                agent: "claude".into(),
                role: Some("Developer".into()),
                instruction: "Implement the plan.".into(),
                max_retries: 0,
                parallel_group: None,
            },
            WorkflowStep {
                agent: "gemini".into(),
                role: Some("Reviewer".into()),
                instruction: "Review the implementation.".into(),
                max_retries: 0,
                parallel_group: None,
            },
        ];
        let task = store
            .create_task("Slice pathing", Some("/tmp/pmf"), &steps)
            .unwrap();
        assert_eq!(task.status, "pending");
        assert_eq!(task.step_index, 0);

        let t1 = store.advance_task(&task.id, None, None).unwrap();
        assert_eq!(t1.status, "running");
        assert_eq!(t1.step_index, 0);
        assert!(t1.last_message_id.is_some());
        let inbox = store.poll_messages("grok", true).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].kind, "handoff");

        let t2 = store
            .advance_task(&task.id, Some("grok"), Some("plan ready"))
            .unwrap();
        assert_eq!(t2.step_index, 1);
        let for_claude = store.poll_messages("claude", true).unwrap();
        assert!(!for_claude.is_empty());
        assert!(for_claude[0].body.contains("Implement"));

        let t3 = store.advance_task(&task.id, Some("claude"), None).unwrap();
        assert_eq!(t3.step_index, 2);

        let done = store.advance_task(&task.id, Some("gemini"), None).unwrap();
        assert_eq!(done.status, "done");

        let listed = store.list_tasks(Some(TaskStatus::Done)).unwrap();
        assert_eq!(listed.len(), 1);
    }

    #[test]
    fn c5_bounded_parallel_and_retry() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let steps = vec![
            WorkflowStep {
                agent: "planner".into(),
                role: None,
                instruction: "Plan".into(),
                max_retries: 0,
                parallel_group: None,
            },
            WorkflowStep {
                agent: "dev_a".into(),
                role: None,
                instruction: "Code path A".into(),
                max_retries: 1,
                parallel_group: Some("impl".into()),
            },
            WorkflowStep {
                agent: "dev_b".into(),
                role: None,
                instruction: "Code path B".into(),
                max_retries: 1,
                parallel_group: Some("impl".into()),
            },
            WorkflowStep {
                agent: "dev_c".into(),
                role: None,
                instruction: "Code path C".into(),
                max_retries: 1,
                parallel_group: Some("impl".into()),
            },
            WorkflowStep {
                agent: "reviewer".into(),
                role: None,
                instruction: "Review all".into(),
                max_retries: 0,
                parallel_group: None,
            },
        ];
        // max_parallel=2 → wake two of three implementers first
        let task = store
            .create_task_with_parallel("parallel slice", None, &steps, 2, true)
            .unwrap();
        let stages = HubStore::workflow_stages(&task.steps);
        assert_eq!(stages.len(), 3); // plan | parallel impl | review

        let t1 = store.advance_task(&task.id, None, None).unwrap();
        assert_eq!(t1.step_index, 0); // sequential plan
        assert!(t1.open_agents.is_empty());

        let t2 = store.advance_task(&task.id, Some("planner"), None).unwrap();
        assert_eq!(t2.step_index, 1);
        assert_eq!(t2.open_agents.len(), 2);
        assert_eq!(t2.pending_agents.len(), 1);

        // Cannot advance while parallel open
        assert!(store.advance_task(&task.id, None, None).is_err());

        let a = t2.open_agents[0].clone();
        let b = t2.open_agents[1].clone();
        let mid = store.complete_parallel_member(&task.id, &a, None).unwrap();
        // one free slot → pending agent wakes
        assert_eq!(mid.open_agents.len(), 2);
        assert!(mid.pending_agents.is_empty());

        let mid2 = store.complete_parallel_member(&task.id, &b, None).unwrap();
        let mid3 = store
            .complete_parallel_member(&task.id, &mid2.open_agents[0], None)
            .unwrap();
        // after draining the third
        let drained = if mid3.open_agents.is_empty() {
            mid3
        } else {
            store
                .complete_parallel_member(&task.id, &mid3.open_agents[0], None)
                .unwrap()
        };
        assert!(drained.open_agents.is_empty());
        assert!(drained.pending_agents.is_empty());

        // Retry current parallel stage once (max_retries=1 on those steps)
        let retried = store
            .retry_task(&task.id, Some("human"), Some("impl flaked"))
            .unwrap();
        assert_eq!(retried.step_index, 1);
        assert_eq!(retried.open_agents.len(), 2);

        // Drain again
        let mut cur = retried;
        while !cur.open_agents.is_empty() || !cur.pending_agents.is_empty() {
            let agent = cur.open_agents[0].clone();
            cur = store
                .complete_parallel_member(&task.id, &agent, None)
                .unwrap();
        }

        let done = store.advance_task(&task.id, Some("dev_a"), None).unwrap(); // review stage
        assert_eq!(done.step_index, 2);
        let finished = store
            .advance_task(&task.id, Some("reviewer"), None)
            .unwrap();
        assert_eq!(finished.status, "done");
    }

    #[test]
    fn c4_task_policy_controls_wake_gate() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .set_wake_policy(&WakePolicy {
                default_requires_human_gate: false,
                allow_auto_wake: true,
            })
            .unwrap();
        let steps = vec![WorkflowStep {
            agent: "claude".into(),
            role: None,
            instruction: "Run the delegated step.".into(),
            max_retries: 0,
            parallel_group: None,
        }];
        let task = store
            .create_task_with_parallel("ungated task", None, &steps, 1, false)
            .unwrap();
        store.advance_task(&task.id, Some("human"), None).unwrap();
        let wakes = store.list_wakes(Some("claude"), true).unwrap();
        assert_eq!(wakes.len(), 1);
        assert!(!wakes[0].requires_human_gate);
        assert!(
            !store
                .get_task(&task.id)
                .unwrap()
                .unwrap()
                .require_human_approval
        );
    }

    #[test]
    fn c6_budget_exhaustion_pauses_writes_handoff_and_blocks_wakes() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let set = store.set_agent_budget("claude", 10.0).unwrap();
        assert_eq!(set.spent_units, 0.0);
        assert!(!set.paused);

        // Under the limit: no pause, wakes still allowed.
        let under = store.record_budget_usage("claude", 4.0).unwrap();
        assert!(!under.paused);
        store
            .request_wake("claude", Some("still fine"), None, true)
            .unwrap();

        // Crossing the limit flips paused, but record_budget_usage alone
        // does not yet write a handoff or block new wakes on its own -- the
        // caller must call pause_for_budget to do that explicitly.
        let over = store.record_budget_usage("claude", 10.0).unwrap();
        assert!(over.paused);
        assert_eq!(over.spent_units, 14.0);

        let outcome = store
            .pause_for_budget(
                "claude",
                Some("task-42"),
                "Implement C6 budget handoff.",
                "Schema + store methods + tests.",
                "CLI/Tauri wiring and roadmap docs.",
                Some("grok"),
            )
            .unwrap();
        assert!(outcome.status.paused);
        assert!(outcome.summary_path.exists());
        let summary = fs::read_to_string(&outcome.summary_path).unwrap();
        assert!(summary.contains("Implement C6 budget handoff."));
        assert!(summary.contains("Delegated to"));
        assert!(summary.contains("grok"));

        let handoff = store
            .get_message(&outcome.handoff_message_id)
            .unwrap()
            .unwrap();
        assert_eq!(handoff.from_agent, "claude");
        assert_eq!(handoff.to_agent, "grok");
        assert_eq!(handoff.kind, "handoff");

        // Paused agent cannot receive further wakes until resumed.
        let err = store
            .request_wake("claude", Some("try again"), None, true)
            .unwrap_err();
        assert!(err.to_string().contains("budget-paused"));

        let resumed = store.resume_agent("claude").unwrap();
        assert!(!resumed.paused);
        store
            .request_wake("claude", Some("resumed"), None, true)
            .unwrap();
    }

    #[test]
    fn c6_shutdown_records_reviewable_handoff() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let outcome = store
            .record_shutdown(
                "claude",
                Some("task-99"),
                "Finish the migration",
                "owner cancelled the active provider call",
                Some("grok"),
            )
            .unwrap();
        assert!(outcome.summary_path.exists());
        let summary = fs::read_to_string(&outcome.summary_path).unwrap();
        assert!(summary.contains("Finish the migration"));
        assert!(summary.contains("owner cancelled"));
        let message = store
            .get_message(&outcome.handoff_message_id)
            .unwrap()
            .unwrap();
        assert_eq!(message.to_agent, "grok");
        assert_eq!(message.kind, "handoff");
    }

    #[test]
    fn channel_queries_and_memory_reference_resolution_are_isolated() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let memory = store
            .write_memory(
                MemoryTier::Episodic,
                MemoryScope::Global,
                Some("chat"),
                None,
                Some("Channel query contract"),
                "Messages can reference durable shared memory.",
                &[],
            )
            .unwrap();
        let short_id = &memory.id[..8];
        let general = store
            .send_message(
                "chat",
                "grok",
                MessageKind::Message,
                &format!("Review this [Memory #{short_id}] twice [Memory #{short_id}]."),
                Some("channel:general"),
                None,
                None,
            )
            .unwrap();
        store
            .send_message(
                "chat",
                "grok",
                MessageKind::Message,
                "Coordination-only message.",
                Some("channel:team-coordination"),
                None,
                None,
            )
            .unwrap();
        store
            .send_message(
                "chat",
                "grok",
                MessageKind::Message,
                "A general thread detail.",
                Some("channel:general:thread-1"),
                None,
                None,
            )
            .unwrap();

        let channel = store.list_channel_messages("general", 10).unwrap();
        assert_eq!(channel.len(), 2);
        assert!(channel.iter().all(|message| message
            .subject
            .as_deref()
            .unwrap()
            .starts_with("channel:general")));
        assert!(!channel
            .iter()
            .any(|message| message.body == "Coordination-only message."));
        assert_eq!(
            store
                .list_channel_messages("channel:general", 1)
                .unwrap()
                .len(),
            1
        );
        assert!(store.list_channel_messages("", 10).is_err());

        assert_eq!(
            parse_memory_references(&general.body),
            vec![short_id.to_string()]
        );
        assert!(parse_memory_references("[Memory #not-an-id] [Memory #]").is_empty());
        let linked = store.list_message_memories(&general.id).unwrap();
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].id, memory.id);
    }

    #[test]
    fn work_sessions_start_with_the_team_and_accept_later_team_members() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("claude", false).unwrap();

        let session = store.create_work_session("Cloud sync design").unwrap();
        assert!(session.member_ids.contains(&"human".to_string()));
        assert!(session.member_ids.contains(&"grok".to_string()));
        assert!(!session.member_ids.contains(&"claude".to_string()));

        store.set_team_member("claude", true).unwrap();
        let updated = store
            .add_work_session_member(&session.id, "claude")
            .unwrap();
        assert!(updated.member_ids.contains(&"claude".to_string()));

        let unchanged = store
            .add_work_session_member(&session.id, "claude")
            .unwrap();
        assert_eq!(
            unchanged
                .member_ids
                .iter()
                .filter(|agent_id| agent_id.as_str() == "claude")
                .count(),
            1
        );
        assert_eq!(store.list_work_sessions().unwrap()[0].id, session.id);
    }

    #[test]
    fn c11_task_tag_rejects_absent_recipient_without_side_effects() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        // "outsider" has never been seen, so it starts out absent.
        let outcomes = store
            .send_tagged_message(
                "human",
                &["grok".to_string(), "outsider".to_string()],
                true,
                false,
                "ship the release",
                None,
                None,
                None,
                None,
            )
            .unwrap();

        let grok = outcomes.iter().find(|o| o.to_agent == "grok").unwrap();
        assert!(grok.accepted);
        assert!(!grok.enrolled);
        assert!(grok.message_id.is_some());

        let outsider = outcomes.iter().find(|o| o.to_agent == "outsider").unwrap();
        assert!(!outsider.accepted);
        assert!(!outsider.enrolled);
        assert!(outsider.message_id.is_none());
        assert!(outsider
            .reason
            .as_deref()
            .unwrap()
            .contains("not a current"));

        // No membership mutation and no message actually delivered to "outsider".
        assert!(!store.is_team_member("outsider").unwrap());
        assert!(store
            .list_messages(Some("outsider"), None)
            .unwrap()
            .is_empty());

        // Durable per-recipient audit trail survives independent of the caller.
        let replayed = store
            .list_tagged_send_outcomes(&outcomes[0].subject)
            .unwrap();
        assert_eq!(replayed.len(), 2);
    }

    #[test]
    fn c11_wake_tag_enrolls_and_requests_wake_for_a_new_identity() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(!store.is_team_member("newbie").unwrap());

        let outcomes = store
            .send_tagged_message(
                "human",
                &["newbie".to_string()],
                false,
                true,
                "join the session and pick up C12",
                None,
                None,
                None,
                None,
            )
            .unwrap();

        let outcome = &outcomes[0];
        assert!(outcome.accepted);
        assert!(outcome.enrolled);
        assert!(outcome.wake_requested);
        assert!(store.is_team_member("newbie").unwrap());
        let pending = store.list_wakes(Some("newbie"), true).unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn c11_task_and_wake_together_apply_both_rules_per_recipient() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let session = store.create_work_session("Cloud sync design").unwrap();

        let outcomes = store
            .send_tagged_message(
                "human",
                &["grok".to_string(), "fresh".to_string()],
                true,
                true,
                "session kickoff",
                None,
                None,
                None,
                Some(&session.id),
            )
            .unwrap();

        // "grok" is already a team+session member: task passes, wake is a no-op enroll.
        let grok = outcomes.iter().find(|o| o.to_agent == "grok").unwrap();
        assert!(grok.accepted);
        assert!(!grok.enrolled);

        // "fresh" is present in neither team nor session, so the task check
        // fails first — task always wins over wake for the same recipient.
        let fresh = outcomes.iter().find(|o| o.to_agent == "fresh").unwrap();
        assert!(!fresh.accepted);
        assert!(!fresh.enrolled);
        assert!(!store.is_team_member("fresh").unwrap());
    }

    #[test]
    fn c11_wake_request_denial_does_not_undo_enrollment_or_delivery() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .set_wake_policy(&WakePolicy {
                default_requires_human_gate: false,
                allow_auto_wake: false,
            })
            .unwrap();

        let outcomes = store
            .send_tagged_message(
                "human",
                &["gated".to_string()],
                false,
                true,
                "policy should deny this auto-wake",
                None,
                None,
                None,
                None,
            )
            .unwrap();

        let outcome = &outcomes[0];
        // Enrollment and message delivery still happen; only the wake itself
        // is denied by the standing auto-wake-forbidden policy.
        assert!(outcome.accepted);
        assert!(outcome.enrolled);
        assert!(store.is_team_member("gated").unwrap());
        assert!(!store.list_messages(Some("gated"), None).unwrap().is_empty());
        assert!(!outcome.wake_requested);
        assert!(outcome.reason.as_deref().unwrap().contains("denied"));
    }

    #[test]
    fn c11_send_tagged_message_requires_a_tag_and_a_recipient() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store
            .send_tagged_message(
                "human",
                &["grok".to_string()],
                false,
                false,
                "body",
                None,
                None,
                None,
                None
            )
            .is_err());
        assert!(store
            .send_tagged_message("human", &[], true, false, "body", None, None, None, None)
            .is_err());
    }
}
