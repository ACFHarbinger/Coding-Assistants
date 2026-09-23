//! Local-only device identity and trust list (S9 / #99).
//!
//! Who may sync going forward. Revoke does **not** rotate
//! `keys/cloud-sync.key` (manual owner action / v1 non-goal).
//! Unknown and revoked devices fail closed.

use super::types::{DeviceId, DeviceIdentity};
use super::SyncError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustStatus {
    Trusted,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustEntry {
    pub id: DeviceId,
    pub folder: String,
    pub status: TrustStatus,
    pub registered_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustList {
    pub schema: u32,
    pub devices: Vec<TrustEntry>,
}

impl TrustList {
    fn empty() -> Self {
        Self {
            schema: SCHEMA,
            devices: Vec::new(),
        }
    }
}

pub fn ensure_local_device(home: &Path) -> Result<DeviceIdentity, SyncError> {
    if let Some(existing) = load_local_device(home)? {
        register(home, existing.id.clone())?;
        return Ok(existing);
    }
    let identity = DeviceIdentity::generate();
    atomic_write(
        &device_path(home),
        &serde_json::to_vec(&identity).map_err(json_err)?,
    )?;
    register(home, identity.id.clone())?;
    Ok(identity)
}

pub fn load_local_device(home: &Path) -> Result<Option<DeviceIdentity>, SyncError> {
    let path = device_path(home);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(io_err)?;
    let identity = serde_json::from_slice(&bytes).map_err(json_err)?;
    Ok(Some(identity))
}

pub fn list_trust(home: &Path) -> Result<TrustList, SyncError> {
    load_trust(home)
}

pub fn register(home: &Path, id: DeviceId) -> Result<TrustEntry, SyncError> {
    let mut list = load_trust(home)?;
    let now = Utc::now().to_rfc3339();
    if let Some(entry) = list.devices.iter_mut().find(|row| row.id == id) {
        entry.status = TrustStatus::Trusted;
        entry.revoked_at = None;
        let saved = entry.clone();
        write_trust(home, &list)?;
        return Ok(saved);
    }
    let entry = TrustEntry {
        folder: id.remote_folder_name(),
        id,
        status: TrustStatus::Trusted,
        registered_at: now,
        revoked_at: None,
    };
    list.devices.push(entry.clone());
    write_trust(home, &list)?;
    Ok(entry)
}

/// Mark a device revoked. Does not read or write `keys/cloud-sync.key`.
pub fn revoke(home: &Path, id: DeviceId) -> Result<TrustEntry, SyncError> {
    let mut list = load_trust(home)?;
    let entry = list
        .devices
        .iter_mut()
        .find(|row| row.id == id)
        .ok_or_else(|| SyncError::Invalid("device is not registered".into()))?;
    entry.status = TrustStatus::Revoked;
    entry.revoked_at = Some(Utc::now().to_rfc3339());
    let saved = entry.clone();
    write_trust(home, &list)?;
    Ok(saved)
}

/// Owner re-trust of a previously revoked (or newly named) device.
pub fn retrust(home: &Path, id: DeviceId) -> Result<TrustEntry, SyncError> {
    register(home, id)
}

/// Unknown or revoked devices are rejected.
pub fn may_accept(home: &Path, id: &DeviceId) -> bool {
    load_trust(home)
        .ok()
        .and_then(|list| {
            list.devices
                .iter()
                .find(|row| row.id == *id)
                .map(|row| row.status == TrustStatus::Trusted)
        })
        .unwrap_or(false)
}

fn load_trust(home: &Path) -> Result<TrustList, SyncError> {
    let path = trust_path(home);
    if !path.is_file() {
        return Ok(TrustList::empty());
    }
    let bytes = fs::read(&path).map_err(io_err)?;
    serde_json::from_slice(&bytes).map_err(json_err)
}

fn write_trust(home: &Path, list: &TrustList) -> Result<(), SyncError> {
    atomic_write(
        &trust_path(home),
        &serde_json::to_vec(list).map_err(json_err)?,
    )
}

fn device_path(home: &Path) -> PathBuf {
    home.join("sync").join("device.json")
}

fn trust_path(home: &Path) -> PathBuf {
    home.join("sync").join("trust.json")
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), SyncError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    let tmp = tmp_path(path);
    fs::write(&tmp, bytes).map_err(io_err)?;
    fs::rename(&tmp, path).map_err(io_err)
}

fn io_err(error: std::io::Error) -> SyncError {
    SyncError::Invalid(error.to_string())
}

fn json_err(error: serde_json::Error) -> SyncError {
    SyncError::Invalid(error.to_string())
}

#[cfg(test)]
#[path = "trust_tests.rs"]
mod tests;
