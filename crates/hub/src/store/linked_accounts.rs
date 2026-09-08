//! Single-user linked external accounts (#286 / H7).
//!
//! Stores user-linked external provider accounts (ChatGPT / Claude / Google / ...)
//! under a provisional "local" key until H2 namespaced identities land.
//!
//! # Security invariants
//! - The linked_account record stores only metadata: provider, external_label,
//!   connection_kind, linked_at, and an optional vault token_ref key name.
//! - The actual token or secret NEVER lands in hub.db or in any struct returned
//!   to callers or across IPC. Only [`LinkedAccountStatus`] crosses IPC.

use super::*;

/// Stored database record for an external linked account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedAccountRecord {
    pub owner: String,
    pub provider: String,
    pub external_label: Option<String>,
    pub connection_kind: String,
    pub linked_at: i64,
    pub token_ref: Option<String>,
}

/// Non-secret status struct crossing IPC for Settings UI (#284).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedAccountStatus {
    pub provider: String,
    pub external_label: Option<String>,
    pub connection_kind: String,
    pub is_linked: bool,
    pub linked_at: Option<i64>,
    pub source: String,
}

impl HubStore {
    pub fn list_linked_accounts(&self, owner: &str) -> Result<Vec<LinkedAccountRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT owner, provider, external_label, connection_kind, linked_at, token_ref
             FROM linked_account
             WHERE owner = ?1
             ORDER BY provider ASC",
        )?;
        let rows = stmt.query_map(params![owner], |row| {
            Ok(LinkedAccountRecord {
                owner: row.get(0)?,
                provider: row.get(1)?,
                external_label: row.get(2)?,
                connection_kind: row.get(3)?,
                linked_at: row.get(4)?,
                token_ref: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_linked_account(
        &self,
        owner: &str,
        provider: &str,
    ) -> Result<Option<LinkedAccountRecord>, HubError> {
        self.conn
            .query_row(
                "SELECT owner, provider, external_label, connection_kind, linked_at, token_ref
                 FROM linked_account
                 WHERE owner = ?1 AND provider = ?2",
                params![owner, provider],
                |row| {
                    Ok(LinkedAccountRecord {
                        owner: row.get(0)?,
                        provider: row.get(1)?,
                        external_label: row.get(2)?,
                        connection_kind: row.get(3)?,
                        linked_at: row.get(4)?,
                        token_ref: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(HubError::from)
    }

    pub fn link_account(
        &self,
        owner: &str,
        provider: &str,
        external_label: Option<&str>,
        connection_kind: &str,
        token_ref: Option<&str>,
    ) -> Result<(), HubError> {
        let owner = owner.trim();
        let provider = provider.trim();
        if owner.is_empty() || provider.is_empty() {
            return Err(HubError::Invalid(
                "owner and provider must not be empty".into(),
            ));
        }
        let now = Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO linked_account(owner, provider, external_label, connection_kind, linked_at, token_ref)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(owner, provider) DO UPDATE SET
                external_label = excluded.external_label,
                connection_kind = excluded.connection_kind,
                linked_at = excluded.linked_at,
                token_ref = excluded.token_ref",
            params![owner, provider, external_label, connection_kind, now, token_ref],
        )?;
        Ok(())
    }

    pub fn unlink_account(&self, owner: &str, provider: &str) -> Result<bool, HubError> {
        let changed = self.conn.execute(
            "DELETE FROM linked_account WHERE owner = ?1 AND provider = ?2",
            params![owner.trim(), provider.trim()],
        )?;
        Ok(changed > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> HubStore {
        let dir = tempfile::tempdir().unwrap();
        HubStore::open(dir.path()).unwrap()
    }

    #[test]
    fn can_link_and_list_accounts() {
        let store = temp_store();
        assert!(store.list_linked_accounts("local").unwrap().is_empty());

        store
            .link_account(
                "local",
                "anthropic",
                Some("claude-pro@example.com"),
                "vendor_cli_login",
                None,
            )
            .unwrap();

        let list = store.list_linked_accounts("local").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].provider, "anthropic");
        assert_eq!(
            list[0].external_label.as_deref(),
            Some("claude-pro@example.com")
        );
        assert_eq!(list[0].connection_kind, "vendor_cli_login");

        let fetched = store.get_linked_account("local", "anthropic").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().provider, "anthropic");
    }

    #[test]
    fn can_unlink_account() {
        let store = temp_store();
        store
            .link_account("local", "openai", None, "oauth_device", None)
            .unwrap();
        assert_eq!(store.list_linked_accounts("local").unwrap().len(), 1);

        let unlinked = store.unlink_account("local", "openai").unwrap();
        assert!(unlinked);
        assert!(store.list_linked_accounts("local").unwrap().is_empty());

        // Second unlink is safe idempotent false
        assert!(!store.unlink_account("local", "openai").unwrap());
    }
}
