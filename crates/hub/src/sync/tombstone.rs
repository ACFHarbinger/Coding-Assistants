//! Confirm-only tombstones and manual 30-day retention (S8 / #98).
//!
//! Deletes never auto-propagate. Expiry cleanup is owner-explicit only.
//! Live `hub.db` is never replaced or deleted.

use super::lock::SyncLockGuard;
use super::types::sha256_hex;
use super::SyncError;
use crate::HubStore;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_RETENTION_DAYS: i64 = 30;
const POLICY_CONFIRM_ONLY: &str = "confirm-only";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tombstone {
    pub slug: String,
    pub path: String,
    pub created_at: String,
    pub expires_at: String,
    pub content_hash: String,
    pub policy: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupCandidate {
    pub kind: String,
    pub slug: String,
    pub aged_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurgeReport {
    pub purged: Vec<String>,
}

pub fn confirm_delete(store: &HubStore, slug: &str) -> Result<Tombstone, SyncError> {
    validate_slug(slug)?;
    let _guard = SyncLockGuard::acquire(store.data_dir(), "tombstone")?;
    confirm_delete_locked(store, slug)
}

pub(crate) fn confirm_delete_locked(store: &HubStore, slug: &str) -> Result<Tombstone, SyncError> {
    validate_slug(slug)?;
    let home = store.data_dir();
    let dir = conflicts_root(home).join(slug);
    if !dir.is_dir() {
        return Err(SyncError::Invalid("conflict not found".into()));
    }
    if !is_delete_confirm(&dir, home, slug)? {
        return Err(SyncError::Invalid(
            "conflict is not a confirm-only delete".into(),
        ));
    }
    let relative = resolve_path(&dir, slug)?;
    if is_live_db(&relative) {
        return Err(SyncError::Invalid(
            "confirm-only delete must not remove live hub.db".into(),
        ));
    }
    let live = home.join(&relative);
    let content_hash = hash_file(&live)
        .or_else(|| hash_file(&dir.join("local")))
        .unwrap_or_default();
    let created = Utc::now();
    let tombstone = Tombstone {
        slug: slug.to_string(),
        path: relative.to_string_lossy().into_owned(),
        created_at: created.to_rfc3339(),
        expires_at: (created + Duration::days(DEFAULT_RETENTION_DAYS)).to_rfc3339(),
        content_hash: content_hash.clone(),
        policy: POLICY_CONFIRM_ONLY.into(),
    };
    let dest = tombstones_root(home).join(format!("{slug}.json"));
    atomic_write(
        &dest,
        &serde_json::to_vec(&tombstone).map_err(|e| SyncError::Invalid(e.to_string()))?,
    )?;
    if live.is_file() {
        fs::remove_file(&live).map_err(io_err)?;
    }
    let payload = serde_json::json!({
        "kind": "sync-tombstone",
        "slug": slug,
        "content_hash": content_hash,
        "local_hash": hash_file(&dir.join("local")),
        "remote_hash": hash_file(&dir.join("remote")),
        "policy": POLICY_CONFIRM_ONLY,
    });
    store
        .record_audit_for_sync(
            Path::new("sync"),
            &relative,
            "sync-tombstone",
            &payload.to_string(),
            None,
        )
        .map_err(|e| SyncError::Invalid(e.to_string()))?;
    Ok(tombstone)
}

pub fn list_tombstones(home: &Path) -> Result<Vec<Tombstone>, SyncError> {
    let root = tombstones_root(home);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for entry in fs::read_dir(&root).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(slug) = name.strip_suffix(".json") else {
            continue;
        };
        if validate_slug(slug).is_err() {
            continue;
        }
        if let Ok(item) = read_tombstone(&entry.path()) {
            items.push(item);
        }
    }
    items.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(items)
}

pub fn expired_cleanup_candidates(
    home: &Path,
    now: DateTime<Utc>,
) -> Result<Vec<CleanupCandidate>, SyncError> {
    let mut items = Vec::new();
    for stone in list_tombstones(home)? {
        if let Some(expires) = parse_rfc3339(&stone.expires_at) {
            if expires <= now {
                items.push(CleanupCandidate {
                    kind: "tombstone".into(),
                    slug: stone.slug,
                    aged_at: stone.expires_at,
                });
            }
        }
    }
    for conflict in list_conflict_ages(home)? {
        if conflict.aged + Duration::days(DEFAULT_RETENTION_DAYS) <= now {
            items.push(CleanupCandidate {
                kind: "conflict".into(),
                slug: conflict.slug,
                aged_at: conflict.aged.to_rfc3339(),
            });
        }
    }
    items.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.kind.cmp(&b.kind)));
    Ok(items)
}

/// Owner-explicit purge. `confirm_all_expired` lists first, then deletes
/// only expired items. Never purges a non-expired slug. Never auto-called.
pub fn purge_expired(
    store: &HubStore,
    slugs: &[String],
    confirm_all_expired: bool,
) -> Result<PurgeReport, SyncError> {
    if !confirm_all_expired && slugs.is_empty() {
        return Err(SyncError::Invalid(
            "purge requires explicit slugs or confirm_all_expired".into(),
        ));
    }
    let home = store.data_dir();
    let now = Utc::now();
    let candidates = expired_cleanup_candidates(home, now)?;
    let targets = if confirm_all_expired && slugs.is_empty() {
        candidates.clone()
    } else {
        resolve_owner_slugs(slugs, &candidates)?
    };
    if targets.is_empty() {
        return Ok(PurgeReport { purged: Vec::new() });
    }
    let _guard = SyncLockGuard::acquire(home, "purge-expired")?;
    let mut purged = Vec::new();
    for item in &targets {
        match item.kind.as_str() {
            "tombstone" => {
                let path = tombstones_root(home).join(format!("{}.json", item.slug));
                if path.is_file() {
                    fs::remove_file(&path).map_err(io_err)?;
                }
            }
            "conflict" => {
                let path = conflicts_root(home).join(&item.slug);
                if path.is_dir() {
                    fs::remove_dir_all(&path).map_err(io_err)?;
                }
            }
            _ => return Err(SyncError::Invalid("unknown cleanup kind".into())),
        }
        purged.push(format!("{}:{}", item.kind, item.slug));
    }
    let payload = serde_json::json!({
        "kind": "sync-purge-expired",
        "slugs": purged,
    });
    store
        .record_audit_for_sync(
            Path::new("sync"),
            Path::new("tombstones"),
            "sync-purge-expired",
            &payload.to_string(),
            None,
        )
        .map_err(|e| SyncError::Invalid(e.to_string()))?;
    Ok(PurgeReport { purged })
}

fn resolve_owner_slugs(
    slugs: &[String],
    candidates: &[CleanupCandidate],
) -> Result<Vec<CleanupCandidate>, SyncError> {
    let mut selected = Vec::new();
    for slug in slugs {
        validate_slug(slug)?;
        let matches: Vec<_> = candidates
            .iter()
            .filter(|c| c.slug == *slug)
            .cloned()
            .collect();
        if matches.is_empty() {
            return Err(SyncError::Invalid(format!(
                "refusing to purge non-expired or unknown slug {slug}"
            )));
        }
        selected.extend(matches);
    }
    Ok(selected)
}

fn is_delete_confirm(dir: &Path, home: &Path, slug: &str) -> Result<bool, SyncError> {
    let reason = fs::read_to_string(dir.join("reason.txt"))
        .unwrap_or_default()
        .trim()
        .to_string();
    if reason == "delete-confirm" {
        return Ok(true);
    }
    let remote_missing = !dir.join("remote").is_file();
    let live_present = resolve_path(dir, slug)
        .map(|p| home.join(p).is_file())
        .unwrap_or(false);
    let local_present = dir.join("local").is_file() || live_present;
    Ok(remote_missing && local_present)
}

fn list_conflict_ages(home: &Path) -> Result<Vec<ConflictAge>, SyncError> {
    let root = conflicts_root(home);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for entry in fs::read_dir(&root).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().into_owned();
        if validate_slug(&slug).is_err() {
            continue;
        }
        if let Some(aged) = conflict_aged_at(&entry.path()) {
            items.push(ConflictAge { slug, aged });
        }
    }
    Ok(items)
}

fn conflict_aged_at(dir: &Path) -> Option<DateTime<Utc>> {
    if let Ok(raw) = fs::read_to_string(dir.join("created_at.txt")) {
        if let Some(ts) = parse_rfc3339(&raw) {
            return Some(ts);
        }
    }
    if let Ok(bytes) = fs::read(dir.join("decision.json")) {
        if let Ok(decision) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(ts) = decision
                .get("decided_at")
                .and_then(|v| v.as_str())
                .and_then(parse_rfc3339)
            {
                return Some(ts);
            }
        }
    }
    dir.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from)
}

fn read_tombstone(path: &Path) -> Result<Tombstone, SyncError> {
    let bytes = fs::read(path).map_err(io_err)?;
    serde_json::from_slice(&bytes).map_err(|e| SyncError::Invalid(e.to_string()))
}

fn resolve_path(dir: &Path, slug: &str) -> Result<PathBuf, SyncError> {
    let raw = fs::read_to_string(dir.join("path.txt"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| slug.replace("__", "/"));
    let path = PathBuf::from(&raw);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|c| c.as_os_str() == "..")
    {
        return Err(SyncError::Invalid("invalid conflict path".into()));
    }
    Ok(path)
}

fn validate_slug(slug: &str) -> Result<(), SyncError> {
    if slug.is_empty()
        || slug == "."
        || slug.contains("..")
        || slug.contains('/')
        || slug.contains('\\')
    {
        return Err(SyncError::Invalid("invalid conflict slug".into()));
    }
    Ok(())
}

fn is_live_db(relative: &Path) -> bool {
    matches!(
        relative.to_str(),
        Some("hub.db" | "hub.db-wal" | "hub.db-shm")
    )
}

fn hash_file(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|b| sha256_hex(&b))
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.trim())
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn tombstones_root(home: &Path) -> PathBuf {
    home.join("sync").join("tombstones")
}

fn conflicts_root(home: &Path) -> PathBuf {
    home.join("sync").join("conflicts")
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

struct ConflictAge {
    slug: String,
    aged: DateTime<Utc>,
}

#[cfg(test)]
#[path = "tombstone_tests.rs"]
mod tests;
