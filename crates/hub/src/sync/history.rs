//! Append-only sync history and redacted diagnostics (S9 / #99).
//!
//! Never include tokens, keys, key filenames, refresh tokens, or journal
//! plaintext in history, diagnostics, or audit JSON.

use super::ops::{self, SyncLimits};
use super::trust::{self, TrustStatus};
use super::types::{BlobId, SyncResult};
use super::SyncError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const SCHEMA: u32 = 1;
pub const HISTORY_LIMIT: usize = 50;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub at: String,
    pub action: String,
    pub uploaded: usize,
    pub downloaded: usize,
    pub conflicts: usize,
    pub pruned: usize,
    pub warnings: Vec<String>,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustExport {
    pub folder: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostics {
    pub schema: u32,
    pub last_verified_base: Option<String>,
    pub trust: Vec<TrustExport>,
    pub limits: SyncLimits,
    pub resume_action: Option<String>,
    pub history: Vec<HistoryEntry>,
}

pub fn append_history(
    home: &Path,
    action: &str,
    result: &SyncResult,
    ok: bool,
) -> Result<HistoryEntry, SyncError> {
    let entry = HistoryEntry {
        at: Utc::now().to_rfc3339(),
        action: action.to_string(),
        uploaded: result.uploaded,
        downloaded: result.downloaded,
        conflicts: result.conflicts,
        pruned: result.pruned,
        warnings: result
            .warnings
            .iter()
            .filter(|row| !is_leaky(row))
            .cloned()
            .collect(),
        ok,
    };
    let path = history_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    let line = serde_json::to_string(&entry).map_err(json_err)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(io_err)?;
    writeln!(file, "{line}").map_err(io_err)?;
    Ok(entry)
}

pub fn list_history(home: &Path) -> Result<Vec<HistoryEntry>, SyncError> {
    let path = history_path(home);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(&path).map_err(io_err)?;
    let mut rows = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(io_err)?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
            rows.push(entry);
        }
    }
    if rows.len() > HISTORY_LIMIT {
        rows.drain(0..rows.len() - HISTORY_LIMIT);
    }
    Ok(rows)
}

pub fn export_diagnostics(home: &Path) -> Result<Diagnostics, SyncError> {
    let trust = trust::list_trust(home)?
        .devices
        .into_iter()
        .map(|row| TrustExport {
            folder: row.folder,
            status: match row.status {
                TrustStatus::Trusted => "trusted".into(),
                TrustStatus::Revoked => "revoked".into(),
            },
        })
        .collect();
    Ok(Diagnostics {
        schema: SCHEMA,
        last_verified_base: read_last_verified_base(home),
        trust,
        limits: ops::ensure_limits(home)?,
        resume_action: ops::pending_retry(home),
        history: list_history(home)?,
    })
}

fn read_last_verified_base(home: &Path) -> Option<String> {
    let bytes = fs::read(home.join("sync").join("last-verified.json")).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let base = value.get("base")?.as_str()?;
    BlobId::parse(base).ok().map(|id| id.as_str().to_string())
}

fn is_leaky(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("token")
        || lower.contains("key")
        || lower.contains("ya29")
        || lower.contains("bearer")
}

fn history_path(home: &Path) -> PathBuf {
    home.join("sync").join("history.jsonl")
}

fn io_err(error: std::io::Error) -> SyncError {
    SyncError::Invalid(error.to_string())
}

fn json_err(error: serde_json::Error) -> SyncError {
    SyncError::Invalid(error.to_string())
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
