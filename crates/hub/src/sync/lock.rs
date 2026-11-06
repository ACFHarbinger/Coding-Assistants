//! Hub mutation lock for an owner-started sync run (S4 / #94).
//!
//! The lock file is local-only (`sync/lock`). It never stores tokens, keys,
//! or secret paths. Preview does not take this lock.

use super::SyncError;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const LOCKED_MESSAGE: &str = "hub is locked for cloud sync";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockFile {
    pub pid: u32,
    pub started_at: String,
    pub action: String,
}

pub fn lock_path(home: &Path) -> PathBuf {
    home.join("sync").join("lock")
}

pub fn is_held(home: &Path) -> bool {
    match read(home) {
        Ok(Some(lock)) => pid_alive(lock.pid),
        _ => false,
    }
}

pub fn read(home: &Path) -> Result<Option<LockFile>, SyncError> {
    let path = lock_path(home);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|_| SyncError::Invalid(LOCKED_MESSAGE.into()))?;
    let parsed: LockFile = serde_json::from_slice(&bytes)
        .map_err(|_| SyncError::Invalid("sync lock is not json".into()))?;
    if parsed.action.trim().is_empty() {
        return Err(SyncError::Invalid("sync lock is missing action".into()));
    }
    Ok(Some(parsed))
}

/// Acquire a lock that survives across IPC until [`release`].
pub fn acquire_persisted(home: &Path, action: &str) -> Result<LockFile, SyncError> {
    let action = action.trim();
    if action.is_empty() {
        return Err(SyncError::Invalid("sync lock action is required".into()));
    }
    if is_held(home) {
        return Err(SyncError::Invalid(LOCKED_MESSAGE.into()));
    }
    let _ = fs::remove_file(lock_path(home));
    fs::create_dir_all(home.join("sync")).map_err(|error| SyncError::Invalid(error.to_string()))?;
    let record = LockFile {
        pid: std::process::id(),
        started_at: chrono::Utc::now().to_rfc3339(),
        action: action.to_string(),
    };
    let path = lock_path(home);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| SyncError::Invalid(LOCKED_MESSAGE.into()))?;
    let body = serde_json::to_vec(&record)
        .map_err(|_| SyncError::Invalid("sync lock encode failed".into()))?;
    file.write_all(&body)
        .map_err(|error| SyncError::Invalid(error.to_string()))?;
    Ok(record)
}

pub fn release(home: &Path) -> Result<(), SyncError> {
    let path = lock_path(home);
    if path.is_file() {
        fs::remove_file(path).map_err(|error| SyncError::Invalid(error.to_string()))?;
    }
    Ok(())
}

/// Releases on drop so CLI runs cannot leave the Hub locked after return.
pub struct SyncLockGuard {
    home: PathBuf,
}

impl SyncLockGuard {
    pub fn acquire(home: &Path, action: &str) -> Result<Self, SyncError> {
        acquire_persisted(home, action)?;
        Ok(Self {
            home: home.to_path_buf(),
        })
    }
}

impl Drop for SyncLockGuard {
    fn drop(&mut self) {
        let _ = release(&self.home);
    }
}

fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn second_acquire_fails_while_pid_is_live() {
        let dir = tempdir().unwrap();
        let first = acquire_persisted(dir.path(), "up").unwrap();
        assert_eq!(first.action, "up");
        assert!(is_held(dir.path()));
        let err = acquire_persisted(dir.path(), "down").unwrap_err();
        assert_eq!(err, SyncError::Invalid(LOCKED_MESSAGE.into()));
        release(dir.path()).unwrap();
        assert!(!is_held(dir.path()));
    }

    #[test]
    fn dead_pid_is_stale_and_cancel_always_clears() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("sync")).unwrap();
        fs::write(
            lock_path(dir.path()),
            r#"{"pid":4294967295,"started_at":"t","action":"up"}"#,
        )
        .unwrap();
        assert!(!is_held(dir.path()));
        acquire_persisted(dir.path(), "sync").unwrap();
        release(dir.path()).unwrap();
        acquire_persisted(dir.path(), "up").unwrap();
        release(dir.path()).unwrap();
        assert!(!is_held(dir.path()));
    }

    #[test]
    fn lock_json_has_no_secret_fields() {
        let dir = tempdir().unwrap();
        let lock = acquire_persisted(dir.path(), "preview-not-used").unwrap();
        let json = serde_json::to_string(&lock).unwrap();
        assert!(!json.contains("token"));
        assert!(!json.contains("key"));
        assert!(!json.contains("refresh"));
        release(dir.path()).unwrap();
    }
}
