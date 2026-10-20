//! `scheduled_tasks` DDL, split out of `policies/audit.rs` (488/500 lines
//! — no room for inline DDL there), same pattern as `ensure_board_tables`.

use super::super::*;

impl HubStore {
    pub fn ensure_schedule_tables(&self) -> Result<(), HubError> {
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
                token_baseline_json TEXT NOT NULL DEFAULT '{}'
            );
            CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_due
                ON scheduled_tasks(status, next_fire_at);
            "#,
        )?;
        Ok(())
    }
}
