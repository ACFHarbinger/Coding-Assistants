use super::super::*;

impl HubStore {
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
}
