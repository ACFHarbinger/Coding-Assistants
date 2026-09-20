//! Shared coordination log (C15 / #330). Bus entries and task-board rows
//! that agents parse to coordinate live in the `bus_entries` table, while
//! Markdown (`AGENT_BUS.md`, per-agent `journals/*.md`) remains human-readable
//! narrative output. Private per-agent journal files are legacy and untouched.

use super::*;

impl HubStore {
    pub fn ensure_bus_entries_table(&self) -> Result<(), HubError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS bus_entries (
                id TEXT PRIMARY KEY NOT NULL,
                agent TEXT NOT NULL,
                topic TEXT NOT NULL DEFAULT 'log',
                body TEXT NOT NULL,
                created_at TEXT NOT NULL,
                issue_ref TEXT,
                task_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_bus_entries_created ON bus_entries(created_at);
            CREATE INDEX IF NOT EXISTS idx_bus_entries_agent_created ON bus_entries(agent, created_at);
            CREATE INDEX IF NOT EXISTS idx_bus_entries_topic_created ON bus_entries(topic, created_at);
            "#,
        )?;
        Ok(())
    }

    /// Append one shared bus entry or task-board row. Empty agent/body is
    /// rejected; a blank topic falls back to `"log"`.
    pub fn append_bus_entry(
        &self,
        agent: &str,
        topic: Option<&str>,
        body: &str,
        issue_ref: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<BusEntryRecord, HubError> {
        self.ensure_mutable()?;
        let agent = agent.trim();
        if agent.is_empty() {
            return Err(HubError::Invalid("bus entry agent is required".into()));
        }
        if body.trim().is_empty() {
            return Err(HubError::Invalid("bus entry body must not be empty".into()));
        }
        let topic = topic
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .unwrap_or("log");
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let issue_ref = issue_ref.map(str::trim).filter(|s| !s.is_empty());
        let task_id = task_id.map(str::trim).filter(|s| !s.is_empty());
        self.conn.execute(
            "INSERT INTO bus_entries(id, agent, topic, body, created_at, issue_ref, task_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, agent, topic, body, created_at, issue_ref, task_id],
        )?;
        Ok(BusEntryRecord {
            id,
            agent: agent.to_string(),
            topic: topic.to_string(),
            body: body.to_string(),
            created_at,
            issue_ref: issue_ref.map(str::to_string),
            task_id: task_id.map(str::to_string),
        })
    }

    /// Query shared bus entries, newest first. All filter fields combine with
    /// AND; `since` is an RFC3339 lower bound on `created_at`.
    pub fn list_bus_entries(
        &self,
        filter: &BusEntryFilter,
    ) -> Result<Vec<BusEntryRecord>, HubError> {
        let mut sql = String::from(
            "SELECT id, agent, topic, body, created_at, issue_ref, task_id FROM bus_entries",
        );
        let mut clauses: Vec<String> = Vec::new();
        let mut values: Vec<String> = Vec::new();
        for (column, value) in [
            ("agent", filter.agent.as_deref()),
            ("topic", filter.topic.as_deref()),
            ("issue_ref", filter.issue_ref.as_deref()),
            ("task_id", filter.task_id.as_deref()),
        ] {
            if let Some(value) = value.map(str::trim).filter(|s| !s.is_empty()) {
                clauses.push(format!("{column} = ?{}", values.len() + 1));
                values.push(value.to_string());
            }
        }
        if let Some(since) = filter
            .since
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            clauses.push(format!("created_at >= ?{}", values.len() + 1));
            values.push(since.to_string());
        }
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY created_at DESC, rowid DESC LIMIT ?");
        values.push(filter.limit.unwrap_or(50).max(1).to_string());
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(values), |row| {
                Ok(BusEntryRecord {
                    id: row.get(0)?,
                    agent: row.get(1)?,
                    topic: row.get(2)?,
                    body: row.get(3)?,
                    created_at: row.get(4)?,
                    issue_ref: row.get(5)?,
                    task_id: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
