//! Mutation gate for an owner-started cloud-sync run (S4 / #94).

use super::*;
use crate::sync::{self, LOCKED_MESSAGE};
use rusqlite::OptionalExtension;

impl HubStore {
    pub fn hub_schema_version(&self) -> Result<i64, HubError> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        value
            .and_then(|raw| raw.parse().ok())
            .ok_or_else(|| HubError::Invalid("hub schema_version missing".into()))
    }

    pub fn ensure_mutable(&self) -> Result<(), HubError> {
        if sync::is_held(self.data_dir()) {
            Err(HubError::Invalid(LOCKED_MESSAGE.into()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn lock_blocks_send_and_leaves_list_readable() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        crate::sync::acquire_persisted(store.data_dir(), "up").unwrap();
        let err = store
            .send_message(
                "claude",
                "cursor",
                MessageKind::Message,
                "hi",
                None,
                None,
                None,
            )
            .unwrap_err();
        assert!(err.to_string().contains(LOCKED_MESSAGE));
        assert!(store.list_agents().is_ok());
        crate::sync::release(store.data_dir()).unwrap();
        store
            .send_message(
                "claude",
                "cursor",
                MessageKind::Message,
                "hi",
                None,
                None,
                None,
            )
            .unwrap();
    }
}
