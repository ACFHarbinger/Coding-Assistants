use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
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
            other => Err(HubError::Invalid(format!("unknown memory tier: {other}"))),
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
            other => Err(HubError::Invalid(format!("unknown scope: {other}"))),
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
pub struct AgentRecord {
    pub id: String,
    pub display_name: String,
    pub created_at: String,
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

pub struct HubStore {
    conn: Connection,
    data_dir: PathBuf,
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
                created_at TEXT NOT NULL
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
                max_parallel INTEGER NOT NULL DEFAULT 4
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_status
                ON tasks(status, updated_at);
            "#,
        )?;

        // Soft-migrate columns for DBs created before C5 retries/parallel.
        for ddl in [
            "ALTER TABLE tasks ADD COLUMN attempts_json TEXT NOT NULL DEFAULT '{}'",
            "ALTER TABLE tasks ADD COLUMN open_agents_json TEXT NOT NULL DEFAULT '[]'",
            "ALTER TABLE tasks ADD COLUMN pending_agents_json TEXT NOT NULL DEFAULT '[]'",
            "ALTER TABLE tasks ADD COLUMN max_parallel INTEGER NOT NULL DEFAULT 4",
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

        Ok(())
    }

    pub fn upsert_agent(&self, id: &str, display_name: &str) -> Result<(), HubError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO agents(id, display_name, created_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(id) DO UPDATE SET display_name = excluded.display_name
            "#,
            params![id, display_name, now],
        )?;
        Ok(())
    }

    pub fn list_agents(&self) -> Result<Vec<AgentRecord>, HubError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, display_name, created_at FROM agents ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(AgentRecord {
                id: r.get(0)?,
                display_name: r.get(1)?,
                created_at: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

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

    pub fn request_wake(
        &self,
        target_agent: &str,
        reason: Option<&str>,
        message_id: Option<&str>,
        requires_human_gate: bool,
    ) -> Result<WakeRecord, HubError> {
        self.upsert_agent(target_agent, target_agent)?;

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
        Ok(n as usize)
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
        Ok(n as usize)
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
        })
    }

    pub fn create_task(
        &self,
        title: &str,
        workspace_path: Option<&str>,
        steps: &[WorkflowStep],
    ) -> Result<TaskRecord, HubError> {
        self.create_task_with_parallel(title, workspace_path, steps, 4)
    }

    pub fn create_task_with_parallel(
        &self,
        title: &str,
        workspace_path: Option<&str>,
        steps: &[WorkflowStep],
        max_parallel: u32,
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
                attempts_json, open_agents_json, pending_agents_json, max_parallel
            ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, NULL, '{}', '[]', '[]', ?8)
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
            ],
        )?;
        self.get_task(&id)?.ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_task(&self, id: &str) -> Result<Option<TaskRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, title, workspace_path, status, step_index, steps_json,
                   created_at, updated_at, last_message_id,
                   attempts_json, open_agents_json, pending_agents_json, max_parallel
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
                   attempts_json, open_agents_json, pending_agents_json, max_parallel
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
            true,
        )?;
        Ok(msg.id)
    }

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

        let polled = store.poll_messages("claude", true).unwrap();
        assert_eq!(polled.len(), 1);
        assert_eq!(polled[0].status, "acked");

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
            .create_task_with_parallel("parallel slice", None, &steps, 2)
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
}
