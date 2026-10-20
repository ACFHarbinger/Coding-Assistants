//! Discord pairing code + allow-list persisted next to `hub.db` (U25 / #319).
//!
//! Mirrors the Telegram binding store, with snowflake (u64) ids and a
//! per-channel message cursor instead of Telegram's global update offset:
//! the bot polls each bound channel's history with `after=<snowflake>`, so
//! replay protection has to be tracked per channel.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::store::HubError;

const BINDING_FILE: &str = "discord_binding.json";
const PAIRING_MINUTES: i64 = 10;

/// Discord user allowed to command this Hub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundUser {
    pub user_id: u64,
    /// Channel the user paired in; replies and wake pushes go here.
    pub channel_id: u64,
    pub username: Option<String>,
    pub bound_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pairing {
    pub code: String,
    pub expires_at: String,
    /// Channel the runner polls while waiting for `/start <code>`.
    pub channel_id: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingFile {
    #[serde(default)]
    pub pairing: Option<Pairing>,
    #[serde(default)]
    pub bound: Vec<BoundUser>,
    #[serde(default)]
    pub notified_wake_ids: Vec<String>,
    /// Per-channel cursor: highest message snowflake already handled.
    /// Persisted so restarting the opt-in runner cannot replay a previously
    /// accepted remote command.
    #[serde(default)]
    pub channel_cursors: BTreeMap<u64, u64>,
}

/// Durable Discord allow-list for one Hub instance.
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
                .map_err(|error| HubError::Invalid(format!("discord binding: {error}")))?
        } else {
            BindingFile::default()
        };
        Ok(Self { path, data })
    }

    pub fn issue_pairing(&mut self, channel_id: u64) -> Result<String, HubError> {
        let code = Uuid::new_v4().simple().to_string()[..6].to_uppercase();
        self.data.pairing = Some(Pairing {
            code: code.clone(),
            expires_at: (Utc::now() + Duration::minutes(PAIRING_MINUTES)).to_rfc3339(),
            channel_id,
        });
        self.save()?;
        Ok(code)
    }

    pub fn is_bound(&self, user_id: u64) -> bool {
        self.data.bound.iter().any(|user| user.user_id == user_id)
    }

    pub fn bound_users(&self) -> &[BoundUser] {
        &self.data.bound
    }

    /// Channels the runner must poll: every bound channel plus the channel
    /// of an active pairing code (bootstrap before anyone is bound).
    pub fn poll_channels(&self) -> Vec<u64> {
        let mut channels: Vec<u64> = self.data.bound.iter().map(|user| user.channel_id).collect();
        if let Some(pairing) = &self.data.pairing {
            channels.push(pairing.channel_id);
        }
        channels.sort_unstable();
        channels.dedup();
        channels
    }

    pub fn cursor(&self, channel_id: u64) -> Option<u64> {
        self.data.channel_cursors.get(&channel_id).copied()
    }

    /// Advances the durable per-channel cursor after a batch was handled.
    pub fn advance_cursor(&mut self, channel_id: u64, message_id: u64) -> Result<(), HubError> {
        let cursor = self.data.channel_cursors.entry(channel_id).or_insert(0);
        if message_id > *cursor {
            *cursor = message_id;
            self.save()?;
        }
        Ok(())
    }

    pub fn redeem(
        &mut self,
        user_id: u64,
        channel_id: u64,
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
                channel_id,
                username,
                bound_at: Utc::now().to_rfc3339(),
            });
        }
        self.data.pairing = None;
        self.save()
    }

    pub fn unbind(&mut self, user_id: Option<u64>) -> Result<usize, HubError> {
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
        if self.was_wake_notified(wake_id) {
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

    pub fn was_wake_notified(&self, wake_id: &str) -> bool {
        self.data.notified_wake_ids.iter().any(|id| id == wake_id)
    }

    fn save(&self) -> Result<(), HubError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_string_pretty(&self.data)
            .map_err(|error| HubError::Invalid(format!("discord binding: {error}")))?;
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
        let code = store.issue_pairing(100).unwrap();
        store.redeem(7, 100, Some("pk".into()), &code).unwrap();
        assert!(store.is_bound(7));
        assert_eq!(store.bound_users()[0].channel_id, 100);
        assert!(store.issue_pairing(100).is_ok());
        assert!(store.redeem(8, 100, None, "NOPE").is_err());
        store.data.pairing = Some(Pairing {
            code: "DEAD00".into(),
            expires_at: (Utc::now() - Duration::minutes(1)).to_rfc3339(),
            channel_id: 100,
        });
        assert!(store.redeem(8, 100, None, "DEAD00").is_err());
        assert!(!store.is_bound(8));
    }

    #[test]
    fn cursors_are_per_channel_monotonic_and_durable() {
        let dir = tempdir().unwrap();
        let mut store = BindingStore::open(dir.path()).unwrap();
        store.advance_cursor(100, 42).unwrap();
        store.advance_cursor(100, 12).unwrap();
        store.advance_cursor(200, 7).unwrap();
        assert_eq!(store.cursor(100), Some(42));
        assert_eq!(store.cursor(200), Some(7));

        let reopened = BindingStore::open(dir.path()).unwrap();
        assert_eq!(reopened.cursor(100), Some(42));
        assert_eq!(reopened.cursor(200), Some(7));
    }

    #[test]
    fn poll_channels_unions_bound_and_pairing_channels() {
        let dir = tempdir().unwrap();
        let mut store = BindingStore::open(dir.path()).unwrap();
        assert!(store.poll_channels().is_empty());
        store.issue_pairing(300).unwrap();
        assert_eq!(store.poll_channels(), vec![300]);
        let code = store.issue_pairing(300).unwrap();
        store.redeem(7, 100, None, &code).unwrap();
        assert_eq!(store.poll_channels(), vec![100]);
    }
}
