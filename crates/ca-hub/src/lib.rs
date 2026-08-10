//! Coding-Assistants shared memory, messaging, and wake primitives.
//!
//! Implements the M1 (SQLite schema) and part of the C1/C2 spine from
//! `docs/moon/roadmaps/memory.md` and `docs/moon/roadmaps/communication.md`:
//! durable cross-agent messages, tiered memory (short-term/episodic/semantic),
//! and per-agent private journals kept in a separate table that is never
//! surfaced through the shared-memory read/search paths.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum HubError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HubError>;

/// Memory tier, per the owner's exact vocabulary (admin report §5, 2026-08-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Complete raw logs/transcripts from very recent sessions.
    ShortTerm,
    /// Lessons/context from a significant event (e.g. a long-standing bug finally fixed).
    Episodic,
    /// General, slow-changing knowledge of a codebase's architecture/deps/features.
    Semantic,
}

impl Tier {
    fn as_str(self) -> &'static str {
        match self {
            Tier::ShortTerm => "short_term",
            Tier::Episodic => "episodic",
            Tier::Semantic => "semantic",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "episodic" => Tier::Episodic,
            "semantic" => Tier::Semantic,
            _ => Tier::ShortTerm,
        }
    }
}

/// Memory scope: the shared global store, or one scoped to a single workspace.
/// Both must coexist per owner Q&A (Grok Q13 / Chat Q13, 2026-08-10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    Global,
    Workspace(String),
}

impl Scope {
    fn as_str(&self) -> &str {
        match self {
            Scope::Global => "global",
            Scope::Workspace(w) => w,
        }
    }

    fn parse(s: &str) -> Self {
        if s == "global" {
            Scope::Global
        } else {
            Scope::Workspace(s.to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from_agent: String,
    /// `None` = broadcast to all agents.
    pub to_agent: Option<String>,
    pub workspace: Option<String>,
    pub thread_id: Option<String>,
    pub body: String,
    pub created_at: String,
    pub read_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub scope: String,
    pub tier: Tier,
    /// `None` = not agent-specific (shared observation).
    pub agent: Option<String>,
    pub content: String,
    pub created_at: String,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub agent: String,
    pub content: String,
    pub created_at: String,
    pub encrypted: bool,
}

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;

CREATE TABLE IF NOT EXISTS messages (
    id          TEXT PRIMARY KEY,
    from_agent  TEXT NOT NULL,
    to_agent    TEXT,
    workspace   TEXT,
    thread_id   TEXT,
    body        TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    read_at     TEXT
);
CREATE INDEX IF NOT EXISTS idx_messages_to ON messages(to_agent);
CREATE INDEX IF NOT EXISTS idx_messages_workspace ON messages(workspace);

CREATE TABLE IF NOT EXISTS memory_entries (
    id          TEXT PRIMARY KEY,
    scope       TEXT NOT NULL,
    tier        TEXT NOT NULL,
    agent       TEXT,
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    stale       INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_memory_scope ON memory_entries(scope);
CREATE INDEX IF NOT EXISTS idx_memory_tier ON memory_entries(tier);

-- Private per-agent journals (M4 / RJ1-RJ3): a separate table, never joined
-- into the shared-memory read/search paths below. `encrypted` records intent
-- only (RJ2 requires explicit owner permission before this crate performs
-- real encryption) -- this table's contents are stored in plaintext for now.
CREATE TABLE IF NOT EXISTS journal_entries (
    id          TEXT PRIMARY KEY,
    agent       TEXT NOT NULL,
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    encrypted   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_journal_agent ON journal_entries(agent);
";

pub struct Hub {
    conn: Connection,
}

impl Hub {
    /// Open (creating if needed) the shared hub database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Open a private, in-memory hub. Useful for tests.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Default on-disk location: `~/.coding-assistants/hub.sqlite3`, matching
    /// the existing `~/.coding-assistants/mcp.json` convention in
    /// `src-tauri/src/agents.rs`.
    pub fn default_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Path::new(&home).join(".coding-assistants").join("hub.sqlite3")
    }

    // ---- Messages (C1) ----------------------------------------------------

    pub fn write_message(
        &self,
        from_agent: &str,
        to_agent: Option<&str>,
        workspace: Option<&str>,
        thread_id: Option<&str>,
        body: &str,
    ) -> Result<Message> {
        let msg = Message {
            id: Uuid::new_v4().to_string(),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.map(str::to_string),
            workspace: workspace.map(str::to_string),
            thread_id: thread_id.map(str::to_string),
            body: body.to_string(),
            created_at: Utc::now().to_rfc3339(),
            read_at: None,
        };
        self.conn.execute(
            "INSERT INTO messages (id, from_agent, to_agent, workspace, thread_id, body, created_at, read_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![msg.id, msg.from_agent, msg.to_agent, msg.workspace, msg.thread_id, msg.body, msg.created_at],
        )?;
        Ok(msg)
    }

    /// Poll for messages, optionally filtered by recipient (`None` recipient
    /// filter also matches broadcasts) and/or workspace. This is the durable
    /// half of the mailbox; the ephemeral wake-signal (C3) is a separate
    /// mechanism layered on top, not implemented in this crate.
    pub fn read_messages(
        &self,
        to_agent: Option<&str>,
        workspace: Option<&str>,
        unread_only: bool,
    ) -> Result<Vec<Message>> {
        let mut sql = String::from(
            "SELECT id, from_agent, to_agent, workspace, thread_id, body, created_at, read_at FROM messages WHERE 1=1",
        );
        let mut clauses: Vec<String> = Vec::new();
        if to_agent.is_some() {
            clauses.push("(to_agent = ?1 OR to_agent IS NULL)".to_string());
        }
        if workspace.is_some() {
            clauses.push(format!("workspace = ?{}", if to_agent.is_some() { 2 } else { 1 }));
        }
        if unread_only {
            clauses.push("read_at IS NULL".to_string());
        }
        for c in &clauses {
            sql.push_str(" AND ");
            sql.push_str(c);
        }
        sql.push_str(" ORDER BY created_at ASC");

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = match (to_agent, workspace) {
            (Some(a), Some(w)) => stmt.query_map(params![a, w], Self::row_to_message)?,
            (Some(a), None) => stmt.query_map(params![a], Self::row_to_message)?,
            (None, Some(w)) => stmt.query_map(params![w], Self::row_to_message)?,
            (None, None) => stmt.query_map([], Self::row_to_message)?,
        };
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn mark_read(&self, message_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE messages SET read_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), message_id],
        )?;
        Ok(())
    }

    fn row_to_message(row: &rusqlite::Row) -> rusqlite::Result<Message> {
        Ok(Message {
            id: row.get(0)?,
            from_agent: row.get(1)?,
            to_agent: row.get(2)?,
            workspace: row.get(3)?,
            thread_id: row.get(4)?,
            body: row.get(5)?,
            created_at: row.get(6)?,
            read_at: row.get(7)?,
        })
    }

    // ---- Memory (M1-M3) ----------------------------------------------------

    pub fn write_memory(
        &self,
        scope: &Scope,
        tier: Tier,
        agent: Option<&str>,
        content: &str,
    ) -> Result<MemoryEntry> {
        let entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            scope: scope.as_str().to_string(),
            tier,
            agent: agent.map(str::to_string),
            content: content.to_string(),
            created_at: Utc::now().to_rfc3339(),
            stale: false,
        };
        self.conn.execute(
            "INSERT INTO memory_entries (id, scope, tier, agent, content, created_at, stale)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![entry.id, entry.scope, entry.tier.as_str(), entry.agent, entry.content, entry.created_at],
        )?;
        Ok(entry)
    }

    /// Simple substring search over a scope's memory (both global and the
    /// given workspace are searched, since scopes must coexist per the
    /// owner's decision). Full-text/vector search is future work.
    pub fn search_memory(&self, scope: &Scope, query: &str) -> Result<Vec<MemoryEntry>> {
        let like = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, scope, tier, agent, content, created_at, stale FROM memory_entries
             WHERE (scope = ?1 OR scope = 'global') AND stale = 0 AND content LIKE ?2
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![scope.as_str(), like], |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                scope: row.get(1)?,
                tier: Tier::parse(&row.get::<_, String>(2)?),
                agent: row.get(3)?,
                content: row.get(4)?,
                created_at: row.get(5)?,
                stale: row.get::<_, i64>(6)? != 0,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn mark_memory_stale(&self, id: &str) -> Result<()> {
        self.conn.execute("UPDATE memory_entries SET stale = 1 WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ---- Private journals (M4 / RJ1-RJ3) -----------------------------------
    //
    // Deliberately separate from messages/memory_entries above: no query in
    // this section ever joins against the shared tables, and no shared-table
    // query above ever reads journal_entries. Real encryption (RJ2) is not
    // implemented yet -- `encrypted` is recorded but not yet enforced.

    pub fn journal_write(&self, agent: &str, content: &str) -> Result<JournalEntry> {
        let entry = JournalEntry {
            id: Uuid::new_v4().to_string(),
            agent: agent.to_string(),
            content: content.to_string(),
            created_at: Utc::now().to_rfc3339(),
            encrypted: false,
        };
        self.conn.execute(
            "INSERT INTO journal_entries (id, agent, content, created_at, encrypted) VALUES (?1, ?2, ?3, ?4, 0)",
            params![entry.id, entry.agent, entry.content, entry.created_at],
        )?;
        Ok(entry)
    }

    /// Returns only the calling agent's own journal entries -- never another
    /// agent's, and never mixed into shared memory search results.
    pub fn journal_read(&self, agent: &str) -> Result<Vec<JournalEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent, content, created_at, encrypted FROM journal_entries WHERE agent = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![agent], |row| {
            Ok(JournalEntry {
                id: row.get(0)?,
                agent: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                encrypted: row.get::<_, i64>(4)? != 0,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn message_by_id(&self, id: &str) -> Result<Option<Message>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, from_agent, to_agent, workspace, thread_id, body, created_at, read_at FROM messages WHERE id = ?1",
                params![id],
                Self::row_to_message,
            )
            .optional()?)
    }
}

pub fn parse_scope(s: &str) -> Scope {
    Scope::parse(s)
}

pub fn parse_tier(s: &str) -> Tier {
    Tier::parse(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_message_roundtrip() {
        let hub = Hub::open_in_memory().unwrap();
        hub.write_message("claude", Some("grok"), Some("coding-assistants"), None, "hello").unwrap();
        let msgs = hub.read_messages(Some("grok"), None, false).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].body, "hello");
        assert!(msgs[0].read_at.is_none());
    }

    #[test]
    fn broadcast_message_visible_to_any_recipient() {
        let hub = Hub::open_in_memory().unwrap();
        hub.write_message("owner", None, None, None, "broadcast").unwrap();
        let msgs = hub.read_messages(Some("anyone"), None, false).unwrap();
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn memory_search_respects_scope_and_global() {
        let hub = Hub::open_in_memory().unwrap();
        hub.write_memory(&Scope::Global, Tier::Semantic, None, "global fact about architecture").unwrap();
        hub.write_memory(&Scope::Workspace("repo-a".into()), Tier::Episodic, Some("claude"), "repo-a lesson learned").unwrap();
        hub.write_memory(&Scope::Workspace("repo-b".into()), Tier::Episodic, Some("grok"), "repo-b lesson learned").unwrap();

        let repo_a_results = hub.search_memory(&Scope::Workspace("repo-a".into()), "lesson").unwrap();
        assert_eq!(repo_a_results.len(), 1);
        assert_eq!(repo_a_results[0].scope, "repo-a");

        let global_results = hub.search_memory(&Scope::Workspace("repo-a".into()), "architecture").unwrap();
        assert_eq!(global_results.len(), 1, "global-scope memory must be visible from any workspace scope");
    }

    #[test]
    fn journals_are_isolated_per_agent_and_never_shared() {
        let hub = Hub::open_in_memory().unwrap();
        hub.journal_write("claude", "private thought").unwrap();
        hub.journal_write("grok", "grok's private thought").unwrap();

        let claude_journal = hub.journal_read("claude").unwrap();
        assert_eq!(claude_journal.len(), 1);
        assert_eq!(claude_journal[0].content, "private thought");

        // Never leaks into shared memory search.
        let shared = hub.search_memory(&Scope::Global, "private").unwrap();
        assert!(shared.is_empty(), "journal entries must never appear in shared memory search");
    }

    #[test]
    fn stale_memory_excluded_from_search() {
        let hub = Hub::open_in_memory().unwrap();
        let entry = hub.write_memory(&Scope::Global, Tier::ShortTerm, None, "temporary note").unwrap();
        hub.mark_memory_stale(&entry.id).unwrap();
        let results = hub.search_memory(&Scope::Global, "temporary").unwrap();
        assert!(results.is_empty());
    }
}
