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
const LOCK_TIMEOUT_SECS: u64 = 5;

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
        let mut store = Self {
            path,
            data: BindingFile::default(),
        };
        store.reload()?;
        Ok(store)
    }

    /// Reload disk state into memory so an external `ca discord unbind` or `pair`
    /// takes effect immediately without restarting the daemon.
    ///
    /// Fails closed: if reading or parsing the file fails, existing authorizations
    /// in memory are cleared so stale permissions cannot be exploited.
    pub fn reload(&mut self) -> Result<(), HubError> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(content) => Some(content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                self.data.bound.clear();
                return Err(HubError::from(error));
            }
        };
        let disk_data = match raw {
            Some(raw) => serde_json::from_str(&raw).map_err(|error| {
                self.data.bound.clear();
                HubError::Invalid(format!("discord binding: {error}"))
            })?,
            None => BindingFile::default(),
        };
        self.data = disk_data;
        Ok(())
    }

    pub fn issue_pairing(&mut self, channel_id: u64) -> Result<String, HubError> {
        let code = Uuid::new_v4().simple().to_string()[..6].to_uppercase();
        let expires_at = (Utc::now() + Duration::minutes(PAIRING_MINUTES)).to_rfc3339();
        self.with_mutation(|store| {
            store.data.pairing = Some(Pairing {
                code: code.clone(),
                expires_at: expires_at.clone(),
                channel_id,
            });
            Ok(code.clone())
        })
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
        self.with_mutation(|store| {
            let cursor = store.data.channel_cursors.entry(channel_id).or_insert(0);
            if message_id > *cursor {
                *cursor = message_id;
            }
            Ok(())
        })
    }

    pub fn redeem(
        &mut self,
        user_id: u64,
        channel_id: u64,
        username: Option<String>,
        code: &str,
    ) -> Result<(), HubError> {
        self.with_mutation(|store| {
            let pairing = store
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
                store.data.pairing = None;
                let _ = store.save();
                return Err(HubError::Invalid("pairing code expired".into()));
            }
            if !store.is_bound(user_id) {
                store.data.bound.push(BoundUser {
                    user_id,
                    channel_id,
                    username,
                    bound_at: Utc::now().to_rfc3339(),
                });
            }
            store.data.pairing = None;
            Ok(())
        })
    }

    pub fn unbind(&mut self, user_id: Option<u64>) -> Result<usize, HubError> {
        self.with_mutation(|store| {
            let before = store.data.bound.len();
            if let Some(user_id) = user_id {
                store.data.bound.retain(|user| user.user_id != user_id);
            } else {
                store.data.bound.clear();
            }
            Ok(before - store.data.bound.len())
        })
    }

    pub fn mark_wake_notified(&mut self, wake_id: &str) -> Result<bool, HubError> {
        if self.was_wake_notified(wake_id) {
            return Ok(false);
        }
        self.with_mutation(|store| {
            if store.was_wake_notified(wake_id) {
                return Ok(false);
            }
            store.data.notified_wake_ids.push(wake_id.to_string());
            if store.data.notified_wake_ids.len() > 64 {
                let extra = store.data.notified_wake_ids.len() - 64;
                store.data.notified_wake_ids.drain(..extra);
            }
            Ok(true)
        })
    }

    pub fn was_wake_notified(&self, wake_id: &str) -> bool {
        self.data.notified_wake_ids.iter().any(|id| id == wake_id)
    }

    fn lock_path(&self) -> PathBuf {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(BINDING_FILE);
        parent.join(format!(".{file_name}.lock"))
    }

    fn with_mutation<T, F>(&mut self, f: F) -> Result<T, HubError>
    where
        F: FnOnce(&mut Self) -> Result<T, HubError>,
    {
        let lock_path = self.lock_path();
        let _guard = BindingLockGuard::acquire(&lock_path)?;
        self.reload()?;
        let res = f(self)?;
        self.save()?;
        Ok(res)
    }

    fn save(&self) -> Result<(), HubError> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let encoded = serde_json::to_string_pretty(&self.data)
            .map_err(|error| HubError::Invalid(format!("discord binding: {error}")))?;
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(BINDING_FILE);
        let tmp_path = parent.join(format!(".{file_name}.tmp-{}", Uuid::new_v4().simple()));
        let write_res = (|| -> Result<(), std::io::Error> {
            let mut file = fs::File::create(&tmp_path)?;
            file.write_all(encoded.as_bytes())?;
            file.sync_all()?;
            fs::rename(&tmp_path, &self.path)?;
            Ok(())
        })();
        if let Err(error) = write_res {
            let _ = fs::remove_file(&tmp_path);
            return Err(HubError::from(error));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LockRecord {
    pid: u32,
    nonce: String,
    created_at_ms: i64,
}

struct BindingLockGuard {
    lock_path: PathBuf,
    nonce: String,
}

impl BindingLockGuard {
    fn acquire(lock_path: &Path) -> Result<Self, HubError> {
        Self::acquire_with_timeout(lock_path, std::time::Duration::from_secs(LOCK_TIMEOUT_SECS))
    }

    fn acquire_with_timeout(
        lock_path: &Path,
        timeout: std::time::Duration,
    ) -> Result<Self, HubError> {
        let start = std::time::Instant::now();
        let pid = std::process::id();
        let nonce = Uuid::new_v4().to_string();

        if let Some(parent) = lock_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        loop {
            match Self::try_create_lock(lock_path, pid, &nonce) {
                Ok(()) => {
                    return Ok(Self {
                        lock_path: lock_path.to_path_buf(),
                        nonce,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if Self::is_stale(lock_path) {
                        let _ = fs::remove_file(lock_path);
                        continue;
                    }
                    if start.elapsed() >= timeout {
                        return Err(HubError::Invalid("discord binding lock timeout".into()));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => {
                    if start.elapsed() >= timeout {
                        return Err(HubError::from(error));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    }

    fn try_create_lock(lock_path: &Path, pid: u32, nonce: &str) -> Result<(), std::io::Error> {
        let record = LockRecord {
            pid,
            nonce: nonce.to_string(),
            created_at_ms: Utc::now().timestamp_millis(),
        };
        let encoded = serde_json::to_vec(&record).map_err(std::io::Error::other)?;
        let parent = lock_path.parent().unwrap_or_else(|| Path::new("."));
        let tmp_path = parent.join(format!(
            ".discord_binding.lock.tmp-{}",
            Uuid::new_v4().simple()
        ));
        let write_res = (|| -> Result<(), std::io::Error> {
            let mut file = fs::File::create(&tmp_path)?;
            file.write_all(&encoded)?;
            file.sync_all()
        })();
        if let Err(error) = write_res {
            let _ = fs::remove_file(&tmp_path);
            return Err(error);
        }
        let link_res = fs::hard_link(&tmp_path, lock_path);
        let _ = fs::remove_file(&tmp_path);
        match link_res {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(error),
            Err(_) => {
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(lock_path)?;
                if let Err(error) = file.write_all(&encoded).and_then(|()| file.sync_all()) {
                    let _ = fs::remove_file(lock_path);
                    return Err(error);
                }
                Ok(())
            }
        }
    }

    fn is_stale(lock_path: &Path) -> bool {
        let raw = match fs::read_to_string(lock_path) {
            Ok(content) => content,
            Err(_) => return false,
        };
        let Ok(record) = serde_json::from_str::<LockRecord>(&raw) else {
            return fs::metadata(lock_path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|elapsed| elapsed > std::time::Duration::from_secs(2));
        };
        if !pid_alive(record.pid) {
            return true;
        }
        let now_ms = Utc::now().timestamp_millis();
        now_ms.saturating_sub(record.created_at_ms) > (LOCK_TIMEOUT_SECS as i64) * 1_000
    }
}

impl Drop for BindingLockGuard {
    fn drop(&mut self) {
        if let Ok(raw) = fs::read_to_string(&self.lock_path) {
            if let Ok(record) = serde_json::from_str::<LockRecord>(&raw) {
                if record.nonce == self.nonce {
                    let _ = fs::remove_file(&self.lock_path);
                }
            }
        }
    }
}

fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

#[cfg(test)]
#[path = "discord_binding_tests.rs"]
mod tests;
