//! Local multi-instance coherence and version-stamped reject-and-refresh (T7).

use std::fmt;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStamp {
    pub mtime: SystemTime,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub enum StaleWriteError {
    StaleSettings,
    Io(String),
}

impl fmt::Display for StaleWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleSettings => write!(
                f,
                "Settings conflict: settings.toml was modified by another instance or desktop."
            ),
            Self::Io(e) => write!(f, "IO error while reading settings stamp: {e}"),
        }
    }
}

impl std::error::Error for StaleWriteError {}

#[derive(Debug, Clone, Default)]
pub struct ConflictState {
    pub is_conflict_active: bool,
    pub conflict_message: String,
    pub expected_stamp: Option<FileStamp>,
}

impl ConflictState {
    pub fn new(home: &Path) -> Self {
        let mut state = Self::default();
        state.update_stamp(home);
        state
    }

    /// Read the current modification stamp of `settings.toml`.
    pub fn read_stamp(home: &Path) -> Option<FileStamp> {
        let path = home.join("settings.toml");
        if let Ok(meta) = fs::metadata(&path) {
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let size = meta.len();
            Some(FileStamp { mtime, size })
        } else {
            None
        }
    }

    /// Update the baseline stamp recorded by this instance.
    pub fn update_stamp(&mut self, home: &Path) {
        self.expected_stamp = Self::read_stamp(home);
        self.is_conflict_active = false;
        self.conflict_message.clear();
    }

    /// Check if the file on disk has changed since `self.expected_stamp` was recorded.
    pub fn is_stale(&self, home: &Path) -> bool {
        let current = Self::read_stamp(home);
        match (&self.expected_stamp, &current) {
            (Some(expected), Some(actual)) => expected != actual,
            (None, Some(_)) => true, // file was created externally
            (Some(_), None) => true, // file was removed externally
            (None, None) => false,
        }
    }

    /// Guard a write action: if stale, sets active conflict and returns an error.
    pub fn guard_write(&mut self, home: &Path) -> Result<(), StaleWriteError> {
        if self.is_stale(home) {
            let msg = "Conflict: settings.toml was modified by another instance. Press [r] to Refresh & retry, [Esc] to dismiss.".to_string();
            self.is_conflict_active = true;
            self.conflict_message = msg;
            Err(StaleWriteError::StaleSettings)
        } else {
            Ok(())
        }
    }

    /// Record a conflict explicitly (e.g. from an audit event or DB transaction mismatch).
    pub fn record_conflict(&mut self, message: String) {
        self.is_conflict_active = true;
        self.conflict_message = message;
    }

    /// Dismiss the active banner without refreshing.
    pub fn dismiss(&mut self) {
        self.is_conflict_active = false;
        self.conflict_message.clear();
    }
}
