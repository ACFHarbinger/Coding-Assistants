use super::super::*;
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

    /// Open an existing Hub database without creating directories, applying
    /// migrations, or writing WAL sidecars beyond SQLite's read-only open.
    pub fn open_existing_read_only(data_dir: impl AsRef<Path>) -> Result<Self, HubError> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let db_path = data_dir.join("hub.db");
        if !db_path.is_file() {
            return Err(HubError::NotFound(format!(
                "no hub.db under {}",
                data_dir.display()
            )));
        }
        let conn = Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Ok(Self { conn, data_dir })
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
                policy_decision TEXT,
                message_id TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tagged_send_outcomes_subject
                ON tagged_send_outcomes(subject, created_at);

            CREATE TABLE IF NOT EXISTS message_recipient_sets (
                subject TEXT PRIMARY KEY NOT NULL,
                session_id TEXT,
                recipient_ids_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS harness_captures (
                id TEXT PRIMARY KEY NOT NULL,
                harness TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                session_id TEXT,
                content_hash TEXT NOT NULL,
                message_id TEXT,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(harness, agent_id, session_id, content_hash)
            );

            CREATE INDEX IF NOT EXISTS idx_harness_captures_session
                ON harness_captures(session_id, created_at);

            CREATE TABLE IF NOT EXISTS chat_channels (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                topic TEXT,
                builtin INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                deleted_at TEXT
            );

            CREATE TABLE IF NOT EXISTS read_markers (
                agent_id TEXT NOT NULL,
                scope TEXT NOT NULL,
                last_read_at TEXT NOT NULL,
                PRIMARY KEY (agent_id, scope)
            );

            CREATE INDEX IF NOT EXISTS idx_read_markers_scope
                ON read_markers(scope);

            CREATE TABLE IF NOT EXISTS harness_session_registrations (
                harness TEXT NOT NULL,
                workspace TEXT NOT NULL,
                disk_session_id TEXT NOT NULL,
                leader_socket TEXT,
                registered_at TEXT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'observed',
                state TEXT NOT NULL DEFAULT 'ready',
                managed_pid INTEGER,
                writer_owner TEXT,
                writer_acquired_at TEXT,
                PRIMARY KEY (harness, workspace)
            );
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
            "ALTER TABLE tagged_send_outcomes ADD COLUMN policy_decision TEXT",
            "ALTER TABLE harness_session_registrations ADD COLUMN mode TEXT NOT NULL DEFAULT 'observed'",
            "ALTER TABLE harness_session_registrations ADD COLUMN state TEXT NOT NULL DEFAULT 'ready'",
            "ALTER TABLE harness_session_registrations ADD COLUMN managed_pid INTEGER",
            "ALTER TABLE harness_session_registrations ADD COLUMN writer_owner TEXT",
            "ALTER TABLE harness_session_registrations ADD COLUMN writer_acquired_at TEXT",
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

        // Team membership is an explicit user action. A fresh Hub starts with
        // only its human owner; agents become session members through the
        // Orchestrate "Add to team" control or an explicit wake.
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
                "UPDATE agents SET team_member = CASE WHEN id = 'human' THEN 1 ELSE 0 END",
                [],
            )?;
            self.conn.execute(
                "INSERT OR REPLACE INTO meta(key, value) VALUES ('team_roster_seeded', '2')",
                [],
            )?;
        } else if roster_seeded.as_deref() == Some("1") {
            // Version 1 silently seeded every primary agent. Migrate only the
            // exact untouched legacy default, preserving any roster that a
            // user has actually changed.
            let legacy_default_count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM agents
                 WHERE team_member = 1 AND id IN ('human', 'claude', 'chat', 'gemini', 'grok')",
                [],
                |row| row.get(0),
            )?;
            let custom_member_count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM agents
                 WHERE team_member = 1 AND id NOT IN ('human', 'claude', 'chat', 'gemini', 'grok')",
                [],
                |row| row.get(0),
            )?;
            if legacy_default_count == 5 && custom_member_count == 0 {
                self.conn.execute(
                    "UPDATE agents SET team_member = CASE WHEN id = 'human' THEN 1 ELSE 0 END",
                    [],
                )?;
            }
            self.conn.execute(
                "INSERT OR REPLACE INTO meta(key, value) VALUES ('team_roster_seeded', '2')",
                [],
            )?;
        }

        self.seed_default_channels()?;
        Ok(())
    }
}
