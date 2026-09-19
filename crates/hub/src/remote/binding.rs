//! Pairing code + allow-list persisted next to `hub.db` (U25 / #319).

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::store::HubError;

const BINDING_FILE: &str = "telegram_binding.json";
const PAIRING_MINUTES: i64 = 10;

/// Telegram user allowed to command this Hub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundUser {
    pub user_id: i64,
    pub chat_id: i64,
    pub username: Option<String>,
    pub bound_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pairing {
    pub code: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingFile {
    #[serde(default)]
    pub pairing: Option<Pairing>,
    #[serde(default)]
    pub bound: Vec<BoundUser>,
    #[serde(default)]
    pub notified_wake_ids: Vec<String>,
}

/// Durable allow-list for one Hub instance.
pub struct BindingStore {
    path: PathBuf,
    data: BindingFile,
}

impl BindingStore {
    pub fn open(data_dir: &Path) -> Result<Self, HubError> {
        let path = data_dir.join(BINDING_FILE);
        let data = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            serde_json::from_str(&raw)
                .map_err(|error| HubError::Invalid(format!("telegram binding: {error}")))?
        } else {
            BindingFile::default()
        };
        Ok(Self { path, data })
    }

    pub fn issue_pairing(&mut self) -> Result<String, HubError> {
        let code = Uuid::new_v4().simple().to_string()[..6].to_uppercase();
        self.data.pairing = Some(Pairing {
            code: code.clone(),
            expires_at: (Utc::now() + Duration::minutes(PAIRING_MINUTES)).to_rfc3339(),
        });
        self.save()?;
        Ok(code)
    }

    pub fn is_bound(&self, user_id: i64) -> bool {
        self.data.bound.iter().any(|user| user.user_id == user_id)
    }

    pub fn bound_users(&self) -> &[BoundUser] {
        &self.data.bound
    }

    pub fn redeem(
        &mut self,
        user_id: i64,
        chat_id: i64,
        username: Option<String>,
        code: &str,
    ) -> Result<(), HubError> {
        let pairing = self
            .data
            .pairing
            .as_ref()
            .ok_or_else(|| HubError::Invalid("no pairing code is active".into()))?;
        if pairing.code != code.trim().to_ascii_uppercase() {
            return Err(HubError::Invalid("pairing code does not match".into()));
        }
        let expires = chrono::DateTime::parse_from_rfc3339(&pairing.expires_at)
            .map_err(|error| HubError::Invalid(format!("pairing expiry: {error}")))?;
        if expires < Utc::now() {
            self.data.pairing = None;
            self.save()?;
            return Err(HubError::Invalid("pairing code expired".into()));
        }
        if !self.is_bound(user_id) {
            self.data.bound.push(BoundUser {
                user_id,
                chat_id,
                username,
                bound_at: Utc::now().to_rfc3339(),
            });
        }
        self.data.pairing = None;
        self.save()
    }

    pub fn unbind(&mut self, user_id: Option<i64>) -> Result<usize, HubError> {
        let before = self.data.bound.len();
        if let Some(user_id) = user_id {
            self.data.bound.retain(|user| user.user_id != user_id);
        } else {
            self.data.bound.clear();
        }
        self.save()?;
        Ok(before - self.data.bound.len())
    }

    pub fn mark_wake_notified(&mut self, wake_id: &str) -> Result<bool, HubError> {
        if self.data.notified_wake_ids.iter().any(|id| id == wake_id) {
            return Ok(false);
        }
        self.data.notified_wake_ids.push(wake_id.to_string());
        if self.data.notified_wake_ids.len() > 64 {
            let extra = self.data.notified_wake_ids.len() - 64;
            self.data.notified_wake_ids.drain(..extra);
        }
        self.save()?;
        Ok(true)
    }

    fn save(&self) -> Result<(), HubError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_string_pretty(&self.data)
            .map_err(|error| HubError::Invalid(format!("telegram binding: {error}")))?;
        let mut file = fs::File::create(&self.path)?;
        file.write_all(encoded.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pairing_redeems_once_and_rejects_a_wrong_or_expired_code() {
        let dir = tempdir().unwrap();
        let mut store = BindingStore::open(dir.path()).unwrap();
        let code = store.issue_pairing().unwrap();
        store.redeem(7, 7, Some("pk".into()), &code).unwrap();
        assert!(store.is_bound(7));
        assert!(store.issue_pairing().is_ok());
        assert!(store.redeem(8, 8, None, "NOPE").is_err());
        store.data.pairing = Some(Pairing {
            code: "DEAD00".into(),
            expires_at: (Utc::now() - Duration::minutes(1)).to_rfc3339(),
        });
        assert!(store.redeem(8, 8, None, "DEAD00").is_err());
        assert!(!store.is_bound(8));
    }
}
