//! Resume state, quotas, and owner-started retry (S9 / #99).
//!
//! Offline retry is owner-started only. No background, interval, watch,
//! or shutdown sync. v1 puts stay sequential; `max_concurrent` is stored.

use super::types::{BlobId, ObjectKind};
use super::SyncError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_MAX_OBJECTS: u64 = 10_000;
pub const DEFAULT_MAX_BYTES: u64 = 536_870_912;
pub const DEFAULT_MAX_CONCURRENT: u32 = 2;
pub const QUOTA_WARNING: &str = "quota reached; owner retry to continue";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncLimits {
    #[serde(default = "default_max_objects")]
    pub max_objects: u64,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: u64,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
}

impl Default for SyncLimits {
    fn default() -> Self {
        Self {
            max_objects: DEFAULT_MAX_OBJECTS,
            max_bytes: DEFAULT_MAX_BYTES,
            max_concurrent: DEFAULT_MAX_CONCURRENT,
        }
    }
}

fn default_max_objects() -> u64 {
    DEFAULT_MAX_OBJECTS
}
fn default_max_bytes() -> u64 {
    DEFAULT_MAX_BYTES
}
fn default_max_concurrent() -> u32 {
    DEFAULT_MAX_CONCURRENT
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeDone {
    pub content_hash: String,
    pub blob_id: BlobId,
    pub relative_path: String,
    pub size: u64,
    pub kind: ObjectKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeState {
    pub action: String,
    pub done: Vec<ResumeDone>,
}

pub fn ensure_limits(home: &Path) -> Result<SyncLimits, SyncError> {
    let path = limits_path(home);
    if path.is_file() {
        let bytes = fs::read(&path).map_err(io_err)?;
        return serde_json::from_slice(&bytes).map_err(json_err);
    }
    let limits = SyncLimits::default();
    write_limits(home, &limits)?;
    Ok(limits)
}

pub fn write_limits(home: &Path, limits: &SyncLimits) -> Result<(), SyncError> {
    atomic_write(
        &limits_path(home),
        &serde_json::to_vec_pretty(limits).map_err(json_err)?,
    )
}

pub fn load_resume(home: &Path) -> Result<Option<ResumeState>, SyncError> {
    let path = resume_path(home);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(io_err)?;
    Ok(Some(serde_json::from_slice(&bytes).map_err(json_err)?))
}

pub fn begin_resume(home: &Path, action: &str) -> Result<ResumeState, SyncError> {
    if let Some(existing) = load_resume(home)? {
        return Ok(existing);
    }
    let state = ResumeState {
        action: action.to_string(),
        done: Vec::new(),
    };
    save_resume(home, &state)?;
    Ok(state)
}

pub fn save_resume(home: &Path, state: &ResumeState) -> Result<(), SyncError> {
    atomic_write(
        &resume_path(home),
        &serde_json::to_vec_pretty(state).map_err(json_err)?,
    )
}

pub fn clear_resume(home: &Path) -> Result<(), SyncError> {
    let path = resume_path(home);
    if path.exists() {
        fs::remove_file(&path).map_err(io_err)?;
    }
    Ok(())
}

/// Owner-started retry target. `None` when there is no resume file.
pub fn pending_retry(home: &Path) -> Option<String> {
    load_resume(home)
        .ok()
        .flatten()
        .map(|state| state.action)
        .filter(|action| !action.is_empty())
}

pub fn find_done<'a>(resume: &'a ResumeState, content_hash: &str) -> Option<&'a ResumeDone> {
    resume
        .done
        .iter()
        .find(|row| row.content_hash == content_hash)
}

pub fn would_exceed(limits: &SyncLimits, resume: &ResumeState, extra_bytes: u64) -> bool {
    let objects = resume.done.len() as u64;
    let bytes: u64 = resume.done.iter().map(|row| row.size).sum();
    objects + 1 > limits.max_objects || bytes.saturating_add(extra_bytes) > limits.max_bytes
}

pub fn record_done(
    home: &Path,
    resume: &mut ResumeState,
    entry: ResumeDone,
) -> Result<(), SyncError> {
    if find_done(resume, &entry.content_hash).is_none() {
        resume.done.push(entry);
    }
    save_resume(home, resume)
}

fn limits_path(home: &Path) -> PathBuf {
    home.join("sync").join("limits.json")
}

fn resume_path(home: &Path) -> PathBuf {
    home.join("sync").join("resume.json")
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
#[path = "ops_tests.rs"]
mod tests;
