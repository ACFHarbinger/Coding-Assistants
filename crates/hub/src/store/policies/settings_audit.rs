use super::super::*;

/// `root_path` used for every settings-audit row so the dedicated stream is
/// a typed filter over the same hash-chained `audit_events` table other Hub
/// consumers already read (`list_audit_events`, `hub_approve_audit`), not a
/// second table.
const SETTINGS_AUDIT_ROOT: &str = "settings";

impl HubStore {
    /// Record a settings change on the shared Hub audit chain, scoped under
    /// `settings`. `process_json` carries only `field`/`scope` (never a
    /// value), matching the roadmap's no-secret-in-audit rule. A settings
    /// change already took effect via its own IPC call rather than needing
    /// separate human review, so the row is written and immediately marked
    /// `approved` rather than left `pending` like observed filesystem
    /// changes.
    pub fn record_settings_audit_event(
        &self,
        field: &str,
        scope: &str,
        action: &str,
    ) -> Result<AuditEvent, HubError> {
        let process_json = serde_json::json!({ "field": field, "scope": scope }).to_string();
        let event = self.record_audit_event(
            Path::new(SETTINGS_AUDIT_ROOT),
            Path::new(field),
            action,
            &process_json,
            None,
        )?;
        self.set_audit_status(&event.id, "approved")?;
        Ok(AuditEvent {
            status: "approved".into(),
            ..event
        })
    }

    /// The dedicated redacted settings audit stream: every row previously
    /// written by [`Self::record_settings_audit_event`].
    pub fn list_settings_audit_events(&self) -> Result<Vec<AuditEvent>, HubError> {
        Ok(self
            .list_audit_events(false)?
            .into_iter()
            .filter(|event| event.root_path == SETTINGS_AUDIT_ROOT)
            .collect())
    }
}
