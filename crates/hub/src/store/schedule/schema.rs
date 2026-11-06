//! `scheduled_tasks` DDL, split out of `policies/audit.rs` (488/500 lines
//! — no room for inline DDL there), same pattern as `ensure_board_tables`.

use super::super::*;

impl HubStore {
    pub fn ensure_schedule_tables(&self) -> Result<(), HubError> {
        // App + `ca schedule tick` can race on the same hub.db; wait rather
        // than fail the second claim with SQLITE_BUSY.
        self.conn
            .busy_timeout(std::time::Duration::from_millis(5_000))?;
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS scheduled_tasks (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                instruction TEXT NOT NULL,
                target_json TEXT NOT NULL,
                workspace TEXT NOT NULL,
                fire_at TEXT NOT NULL,
                repeat_interval_secs INTEGER,
                response_timeout_secs INTEGER NOT NULL DEFAULT 3600,
                gate_criteria_json TEXT NOT NULL DEFAULT '[]',
                gate_combinator TEXT NOT NULL DEFAULT 'all',
                budget_json TEXT NOT NULL DEFAULT '{}',
                max_attempts INTEGER NOT NULL,
                deadline TEXT,
                status TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_fired_at TEXT,
                next_fire_at TEXT,
                attempts INTEGER NOT NULL DEFAULT 0,
                token_baseline_json TEXT NOT NULL DEFAULT '{}',
                handoff_metadata_json TEXT,
                context_filter TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_due
                ON scheduled_tasks(status, next_fire_at);
            CREATE TABLE IF NOT EXISTS scheduled_dispatch_outbox (
                id TEXT PRIMARY KEY NOT NULL,
                schedule_id TEXT NOT NULL,
                attempt INTEGER NOT NULL,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL,
                delivered_at TEXT,
                UNIQUE(schedule_id, attempt),
                FOREIGN KEY(schedule_id) REFERENCES scheduled_tasks(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_scheduled_outbox_pending
                ON scheduled_dispatch_outbox(schedule_id, attempt);
            CREATE TABLE IF NOT EXISTS scheduled_dispatch_recipients (
                schedule_id TEXT NOT NULL,
                attempt INTEGER NOT NULL,
                to_agent TEXT NOT NULL,
                message_id TEXT,
                claimed_at TEXT NOT NULL,
                PRIMARY KEY (schedule_id, attempt, to_agent)
            );
            "#,
        )?;
        // #352 resume link. Duplicate-column errors on already-migrated DBs
        // are ignored the same way audit.rs soft-migrates.
        let _ = self.conn.execute(
            "ALTER TABLE scheduled_tasks ADD COLUMN pending_gate_json TEXT",
            [],
        );
        // P17 / #361. NULL means the pre-filter handoff body and no metadata audit.
        let _ = self.conn.execute(
            "ALTER TABLE scheduled_tasks ADD COLUMN handoff_metadata_json TEXT",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE scheduled_tasks ADD COLUMN context_filter TEXT",
            [],
        );
        Ok(())
    }
}
