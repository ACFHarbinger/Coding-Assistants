use super::super::*;

impl HubStore {
    /// Records that `agent_id` has read `scope` as of `at` (defaults to
    /// now). Only ever moves a marker forward — an out-of-order or replayed
    /// older timestamp never regresses an existing, more recent one.
    pub fn mark_read(
        &self,
        agent_id: &str,
        scope: &str,
        at: Option<&str>,
    ) -> Result<ReadMarker, HubError> {
        if agent_id.trim().is_empty() || scope.trim().is_empty() {
            return Err(HubError::Invalid(
                "mark_read requires a non-empty agent_id and scope".into(),
            ));
        }
        let last_read_at = at
            .map(str::to_string)
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        self.conn.execute(
            r#"
            INSERT INTO read_markers(agent_id, scope, last_read_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(agent_id, scope) DO UPDATE SET
                last_read_at = excluded.last_read_at
            WHERE excluded.last_read_at > read_markers.last_read_at
            "#,
            params![agent_id, scope, last_read_at],
        )?;
        // The UPSERT's WHERE guard means a regressed write is a silent
        // no-op, not a rejection — re-read whatever the marker actually
        // holds now rather than assume `last_read_at` above took effect.
        self.conn
            .query_row(
                "SELECT agent_id, scope, last_read_at FROM read_markers WHERE agent_id = ?1 AND scope = ?2",
                params![agent_id, scope],
                |row| {
                    Ok(ReadMarker {
                        agent_id: row.get(0)?,
                        scope: row.get(1)?,
                        last_read_at: row.get(2)?,
                    })
                },
            )
            .map_err(HubError::from)
    }

    /// Every team member's read marker for `scope`, for the chat UI to
    /// render "read by" against each message's `created_at`.
    pub fn list_read_markers(&self, scope: &str) -> Result<Vec<ReadMarker>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_id, scope, last_read_at FROM read_markers WHERE scope = ?1 ORDER BY agent_id",
        )?;
        let rows = stmt.query_map(params![scope], |row| {
            Ok(ReadMarker {
                agent_id: row.get(0)?,
                scope: row.get(1)?,
                last_read_at: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn mark_read_creates_and_updates_a_marker() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let first = store
            .mark_read("grok", "channel:general", Some("2026-01-01T00:00:00Z"))
            .unwrap();
        assert_eq!(first.agent_id, "grok");
        assert_eq!(first.last_read_at, "2026-01-01T00:00:00Z");

        let second = store
            .mark_read("grok", "channel:general", Some("2026-01-02T00:00:00Z"))
            .unwrap();
        assert_eq!(second.last_read_at, "2026-01-02T00:00:00Z");
    }

    #[test]
    fn mark_read_never_regresses_an_existing_marker() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        store
            .mark_read("grok", "channel:general", Some("2026-01-05T00:00:00Z"))
            .unwrap();
        let after_stale_write = store
            .mark_read("grok", "channel:general", Some("2026-01-01T00:00:00Z"))
            .unwrap();
        assert_eq!(after_stale_write.last_read_at, "2026-01-05T00:00:00Z");
    }

    #[test]
    fn mark_read_rejects_empty_agent_or_scope() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store.mark_read("", "channel:general", None).is_err());
        assert!(store.mark_read("grok", "", None).is_err());
    }

    #[test]
    fn list_read_markers_is_scoped_and_covers_multiple_agents() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        store
            .mark_read("grok", "channel:general", Some("2026-01-01T00:00:00Z"))
            .unwrap();
        store
            .mark_read("claude", "channel:general", Some("2026-01-02T00:00:00Z"))
            .unwrap();
        store
            .mark_read("claude", "channel:other", Some("2026-01-03T00:00:00Z"))
            .unwrap();

        let general = store.list_read_markers("channel:general").unwrap();
        assert_eq!(general.len(), 2);
        assert!(general.iter().any(|m| m.agent_id == "grok"));
        assert!(general.iter().any(|m| m.agent_id == "claude"));

        let other = store.list_read_markers("channel:other").unwrap();
        assert_eq!(other.len(), 1);
        assert_eq!(other[0].agent_id, "claude");
    }
}
