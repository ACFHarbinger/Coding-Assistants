use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactReport {
    pub examined: usize,
    pub promoted: usize,
    pub kept: usize,
    pub skipped: usize,
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
            "#,
        )?;

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
                tags_json, created_at, updated_at, stale
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)
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
            ],
        )?;

        self.get_memory(&id)?
            .ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_memory(&self, id: &str) -> Result<Option<MemoryRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, scope, workspace_path, tier, agent_id, title, body,
                   tags_json, created_at, updated_at, stale
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
                   tags_json, created_at, updated_at, stale
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
                   tags_json, created_at, updated_at, stale
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
        self.get_message(&id)?
            .ok_or_else(|| HubError::NotFound(id))
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

    pub fn request_wake(
        &self,
        target_agent: &str,
        reason: Option<&str>,
        message_id: Option<&str>,
        requires_human_gate: bool,
    ) -> Result<WakeRecord, HubError> {
        self.upsert_agent(target_agent, target_agent)?;
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
    pub fn append_private_journal(
        &self,
        agent_id: &str,
        entry: &str,
    ) -> Result<PathBuf, HubError> {
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

    /// Export high-priority memories as git-friendly Markdown under `markdown/`.
    pub fn export_markdown(&self, out_dir: Option<&Path>) -> Result<PathBuf, HubError> {
        let out = out_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.data_dir.join("markdown"));
        fs::create_dir_all(&out)?;

        let episodic = self.list_memories(None, Some(MemoryTier::Episodic), None, false)?;
        let semantic = self.list_memories(None, Some(MemoryTier::Semantic), None, false)?;

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

        let path = out.join("shared_memory.md");
        fs::write(&path, body)?;
        Ok(path)
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
        assert!(wake.requires_human_gate);
        assert!(dir.path().join("wake").join(format!("{}.json", wake.id)).exists());

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
        let semantic = store
            .promote_memory(&one.id, MemoryTier::Semantic)
            .unwrap();
        assert_eq!(semantic.tier, "semantic");
        store.delete_memory(&semantic.id).unwrap();
        assert!(store.get_memory(&semantic.id).unwrap().is_none());
    }
}
