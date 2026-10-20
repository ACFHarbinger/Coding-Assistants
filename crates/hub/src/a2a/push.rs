//! Per-task A2A push-notification (webhook) config storage (issue #364).
//!
//! Persists to `a2a_push_configs.json` next to `hub.db`, following the
//! `telegram_binding.json` precedent. Writes are atomic (temp file +
//! rename) and serialized process-wide through [`PUSH_STORE_LOCK`] so the
//! std-only A2A listener threads and the notifier thread cannot interleave
//! a read-modify-write.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::store::HubError;

/// Config file name inside the Hub data dir.
pub const PUSH_CONFIG_FILE: &str = "a2a_push_configs.json";
/// Per-task cap: A2A v1.0 allows several configs per task; bound them so a
/// caller cannot turn the notifier into an amplification loop.
pub const MAX_CONFIGS_PER_TASK: usize = 10;
/// Global cap across all tasks.
pub const MAX_CONFIGS_TOTAL: usize = 256;
/// Bound on callback URL length.
pub const MAX_URL_LEN: usize = 2048;
/// Allowed characters for caller-supplied config ids (must also keep the
/// derived vault key valid: `[A-Za-z0-9_.:-]`).
pub const MAX_ID_LEN: usize = 64;

fn default_enabled() -> bool {
    true
}

/// One registered webhook. The wire view (id/url/token/authentication) is
/// built from this in `rpc.rs`; the webhook credential itself is never
/// stored here — only `has_credentials` records that the P12 vault holds
/// it under [`vault_key_for`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushConfigEntry {
    pub task_id: String,
    pub config_id: String,
    pub url: String,
    /// Opaque caller-owned echo token (A2A `PushNotificationConfig.token`).
    /// Sent back as the `X-A2A-Notification-Token` header; never logged.
    #[serde(default)]
    pub token: Option<String>,
    /// Scheme names from `authentication.schemes` (e.g. `["Bearer"]`).
    #[serde(default)]
    pub auth_schemes: Vec<String>,
    /// True when `authentication.credentials` was handed to the P12 vault.
    #[serde(default)]
    pub has_credentials: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub created_at: String,
    /// Last task state the notifier attempted to deliver. Set to the
    /// current state at registration (baseline), updated after every
    /// delivery attempt — best-effort, so a crashed notifier does not
    /// re-fire ancient transitions on restart.
    #[serde(default)]
    pub last_notified_state: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PushConfigFile {
    #[serde(default)]
    configs: Vec<PushConfigEntry>,
}

/// Durable registry for webhook configs of one Hub instance.
pub struct PushConfigStore {
    path: PathBuf,
    data: PushConfigFile,
}

/// Serializes read-modify-write cycles across listener/notifier threads.
static PUSH_STORE_LOCK: Mutex<()> = Mutex::new(());

/// P12 vault key holding one config's webhook credential.
pub fn vault_key_for(task_id: &str, config_id: &str) -> String {
    format!("a2a.push.{task_id}.{config_id}")
}

/// Open a store under the process-wide lock, mutate, and persist. This is
/// the only write path; readers (notifier) use [`PushConfigStore::open`].
pub fn update_store<T>(
    data_dir: &Path,
    mutate: impl FnOnce(&mut PushConfigStore) -> Result<T, HubError>,
) -> Result<T, HubError> {
    let _guard = PUSH_STORE_LOCK.lock().unwrap();
    let mut store = PushConfigStore::open(data_dir)?;
    let out = mutate(&mut store)?;
    store.save()?;
    Ok(out)
}

impl PushConfigStore {
    pub fn open(data_dir: &Path) -> Result<Self, HubError> {
        let path = data_dir.join(PUSH_CONFIG_FILE);
        let data = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            serde_json::from_str(&raw)
                .map_err(|error| HubError::Invalid(format!("a2a push configs: {error}")))?
        } else {
            PushConfigFile::default()
        };
        Ok(Self { path, data })
    }

    fn save(&self) -> Result<(), HubError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_string_pretty(&self.data)
            .map_err(|error| HubError::Invalid(format!("a2a push configs: {error}")))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, encoded)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn configs(&self) -> &[PushConfigEntry] {
        &self.data.configs
    }

    pub fn count_for_task(&self, task_id: &str) -> usize {
        self.data
            .configs
            .iter()
            .filter(|entry| entry.task_id == task_id)
            .count()
    }

    pub fn find(&self, task_id: &str, config_id: &str) -> Option<&PushConfigEntry> {
        self.data
            .configs
            .iter()
            .find(|entry| entry.task_id == task_id && entry.config_id == config_id)
    }

    pub fn find_mut(&mut self, task_id: &str, config_id: &str) -> Option<&mut PushConfigEntry> {
        self.data
            .configs
            .iter_mut()
            .find(|entry| entry.task_id == task_id && entry.config_id == config_id)
    }

    pub fn list_for_task(&self, task_id: &str) -> Vec<PushConfigEntry> {
        self.data
            .configs
            .iter()
            .filter(|entry| entry.task_id == task_id)
            .cloned()
            .collect()
    }

    /// Insert or replace the `(task_id, config_id)` slot.
    pub fn upsert(&mut self, entry: PushConfigEntry) {
        if let Some(existing) = self.find_mut(&entry.task_id, &entry.config_id) {
            *existing = entry;
        } else {
            self.data.configs.push(entry);
        }
    }

    pub fn remove(&mut self, task_id: &str, config_id: &str) -> Option<PushConfigEntry> {
        let index = self
            .data
            .configs
            .iter()
            .position(|entry| entry.task_id == task_id && entry.config_id == config_id)?;
        Some(self.data.configs.remove(index))
    }
}

/// Config/task ids must be safe inside a P12 vault key
/// (`crate::secret::validate_key` alphabet) and bounded.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// SSRF-sane callback URL check (see the module threat model): http(s)
/// only, explicit host, no userinfo, bounded length. Loopback and
/// internal addresses are allowed — the A2A caller is already
/// owner-authenticated and the deployment posture is loopback-default.
pub fn validate_callback_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("callback url must not be empty".into());
    }
    if trimmed.len() > MAX_URL_LEN {
        return Err(format!("callback url exceeds {MAX_URL_LEN} chars"));
    }
    let uri: ureq::http::Uri = trimmed
        .parse()
        .map_err(|_| "callback url is not a valid URI".to_string())?;
    match uri.scheme_str() {
        Some("http") | Some("https") => {}
        _ => return Err("callback url must use http or https".into()),
    }
    let authority = uri
        .authority()
        .ok_or_else(|| "callback url needs a host".to_string())?;
    if authority.as_str().contains('@') {
        return Err("callback url must not embed userinfo credentials".into());
    }
    if uri.host().unwrap_or("").is_empty() {
        return Err("callback url needs a host".into());
    }
    Ok(())
}
