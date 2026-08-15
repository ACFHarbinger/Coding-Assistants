use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 1;

// Shared record types (enums/structs/helpers) live in [types]; submodules
// reach them through this re-export, and the import statements above stay
// here because the impl submodules glob-import this module
// (use super::super::*).
mod types;
pub use types::*;

mod agents;
mod attachments;
mod exports;
mod messages;
mod models;
pub use models::*;
mod policies;
mod roles;
mod tasks;
#[cfg(test)]
mod tests;
pub struct HubStore {
    conn: Connection,
    data_dir: PathBuf,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn slug_channel_id(name: &str) -> Result<String, HubError> {
    let trimmed = name.trim().trim_start_matches('#');
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in trimmed.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            Some(ch.to_ascii_lowercase())
        } else if ch == ' ' || ch == '-' || ch == '_' {
            if last_dash || slug.is_empty() {
                None
            } else {
                last_dash = true;
                Some('-')
            }
        } else {
            None
        };
        if let Some(mapped) = mapped {
            slug.push(mapped);
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() || slug.len() > 40 {
        return Err(HubError::Invalid(
            "channel name must be 1–40 letters, numbers, or hyphens".into(),
        ));
    }
    Ok(slug)
}

fn audit_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditEvent> {
    Ok(AuditEvent {
        id: row.get(0)?,
        root_path: row.get(1)?,
        path: row.get(2)?,
        operation: row.get(3)?,
        observed_at: row.get(4)?,
        process_json: row.get(5)?,
        content_hash: row.get(6)?,
        previous_hash: row.get(7)?,
        event_hash: row.get(8)?,
        status: row.get(9)?,
    })
}
