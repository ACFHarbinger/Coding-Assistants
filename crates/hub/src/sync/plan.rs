//! Shared sync plan for CLI and desktop (S4–S5 / #94–#95).
//!
//! Category counts only — no secret filenames, tokens, or absolute paths.

use super::fs_drive::FsDrive;
use super::google_auth::resolve_refresh_token;
use super::key::CloudSyncKey;
use super::lock;
use super::policy::classify;
use super::snapshot::{self, LIVE_DB_WARNING};
use super::types::{BlobId, Category, SyncConfig};
use super::{SyncError, SyncResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncPlan {
    pub action: String,
    pub account_connected: bool,
    pub provider: String,
    pub local_schema: String,
    pub replica_schema: Option<String>,
    pub schema_warning: Option<String>,
    pub last_verified_base: Option<String>,
    pub category_counts: BTreeMap<String, usize>,
    pub lock_held: bool,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncSession {
    pub plan: SyncPlan,
    pub result: SyncResult,
}

#[derive(Deserialize)]
struct LastVerified {
    schema_version: Option<String>,
    base: Option<String>,
}

pub fn build_plan(home: &Path, local_schema: i64, action: &str) -> Result<SyncPlan, SyncError> {
    let connected = resolve_refresh_token().is_some();
    let last = read_last_verified(home);
    let replica_schema = last.as_ref().and_then(|row| row.schema_version.clone());
    let last_verified_base = last.as_ref().and_then(|row| {
        row.base
            .as_deref()
            .and_then(|value| BlobId::parse(value).ok().map(|id| id.as_str().to_string()))
    });
    let schema_warning = replica_schema.as_ref().and_then(|remote| {
        let local = local_schema.to_string();
        if remote != &local {
            Some(format!(
                "hub schema mismatch: local {local}, replica {remote}"
            ))
        } else {
            None
        }
    });
    Ok(SyncPlan {
        action: action.to_string(),
        account_connected: connected,
        provider: if connected {
            "google-drive".into()
        } else {
            "none".into()
        },
        local_schema: local_schema.to_string(),
        replica_schema,
        schema_warning,
        last_verified_base,
        category_counts: count_categories(home),
        lock_held: lock::is_held(home),
        errors: Vec::new(),
    })
}

/// Owner-started run: take the lock, then transfer an encrypted snapshot.
pub fn run_locked(home: &Path, local_schema: i64, action: &str) -> Result<SyncSession, SyncError> {
    if action == "preview" {
        return Err(SyncError::Invalid(
            "preview does not take the hub lock".into(),
        ));
    }
    let _guard = lock::SyncLockGuard::acquire(home, action)?;
    finish_run(home, local_schema, action)
}

/// Desktop start: lock stays held until [`lock::release`].
pub fn start_persisted(
    home: &Path,
    local_schema: i64,
    action: &str,
) -> Result<SyncSession, SyncError> {
    if action == "preview" {
        return Err(SyncError::Invalid(
            "preview does not take the hub lock".into(),
        ));
    }
    lock::acquire_persisted(home, action)?;
    finish_run(home, local_schema, action)
}

fn finish_run(home: &Path, local_schema: i64, action: &str) -> Result<SyncSession, SyncError> {
    let mut plan = build_plan(home, local_schema, action)?;
    plan.lock_held = lock::is_held(home);
    let mut result = SyncResult::default();
    if let Some(warning) = plan.schema_warning.clone() {
        result.warnings.push(warning);
    }
    let key = CloudSyncKey::load(home)?;
    let config = SyncConfig::v1_defaults();
    let mut drive = FsDrive::open_local(home)?;
    match action {
        "up" => merge_result(
            &mut result,
            snapshot::upload_home(home, &key, &mut drive, &config, local_schema)?,
        ),
        "down" => merge_result(
            &mut result,
            snapshot::download_home(home, &key, &drive, &config)?,
        ),
        "sync" => {
            merge_result(
                &mut result,
                snapshot::upload_home(home, &key, &mut drive, &config, local_schema)?,
            );
            merge_result(
                &mut result,
                snapshot::download_home(home, &key, &drive, &config)?,
            );
        }
        _ => {
            return Err(SyncError::Invalid(format!("unknown sync action: {action}")));
        }
    }
    if !result.warnings.iter().any(|row| row == LIVE_DB_WARNING) {
        result.warnings.push(LIVE_DB_WARNING.into());
    }
    Ok(SyncSession { plan, result })
}

fn merge_result(into: &mut SyncResult, from: SyncResult) {
    into.uploaded += from.uploaded;
    into.downloaded += from.downloaded;
    into.pruned += from.pruned;
    into.conflicts += from.conflicts;
    for warning in from.warnings {
        if !into.warnings.contains(&warning) {
            into.warnings.push(warning);
        }
    }
}

fn read_last_verified(home: &Path) -> Option<LastVerified> {
    let path = home.join("sync").join("last-verified.json");
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn count_categories(home: &Path) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    visit(home, home, &mut counts);
    counts
}

fn visit(home: &Path, dir: &Path, counts: &mut BTreeMap<String, usize>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            visit(home, &path, counts);
            continue;
        }
        let relative = path.strip_prefix(home).unwrap_or(&path);
        let category = classify(relative);
        let key = category_key(category);
        *counts.entry(key).or_insert(0) += 1;
    }
}

fn category_key(category: Category) -> String {
    serde_json::to_value(category)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "shared_durable".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn plan_omits_secret_names_and_warns_on_schema_mismatch() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("keys/journals")).unwrap();
        fs::create_dir_all(dir.path().join("journals")).unwrap();
        fs::write(dir.path().join("keys/cloud-sync.key"), [7u8; 32]).unwrap();
        fs::write(dir.path().join("keys/journals/claude.key"), b"fernet").unwrap();
        fs::write(dir.path().join("journals/claude.md"), b"secret-journal").unwrap();
        fs::create_dir_all(dir.path().join("sync")).unwrap();
        fs::write(
            dir.path().join("sync/last-verified.json"),
            r#"{"schema_version":"2","base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        )
        .unwrap();
        let plan = build_plan(dir.path(), 3, "preview").unwrap();
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("hub schema mismatch"));
        assert!(!json.contains("cloud-sync.key"));
        assert!(!json.contains("claude.key"));
        assert!(!json.contains("claude.md"));
        assert!(!json.contains("fernet"));
        assert!(!json.contains("token"));
        assert_eq!(
            plan.last_verified_base.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert!(!plan.lock_held);
    }

    #[test]
    fn preview_does_not_lock_and_run_releases() {
        let dir = tempdir().unwrap();
        CloudSyncKey::create(dir.path()).unwrap();
        fs::write(dir.path().join("hub.db"), b"sqlite").unwrap();
        fs::create_dir_all(dir.path().join("journals")).unwrap();
        fs::write(dir.path().join("journals/claude.md"), b"secret-journal").unwrap();
        let plan = build_plan(dir.path(), 3, "preview").unwrap();
        assert!(!plan.lock_held);
        let session = run_locked(dir.path(), 3, "up").unwrap();
        assert!(!lock::is_held(dir.path()));
        assert!(session.result.uploaded >= 1);
        assert!(session
            .result
            .warnings
            .iter()
            .any(|row| row.contains(LIVE_DB_WARNING)));
        assert_eq!(fs::read(dir.path().join("hub.db")).unwrap(), b"sqlite");
        let json = serde_json::to_string(&session).unwrap();
        assert!(!json.contains("Bearer"));
        assert!(!json.contains("secret-journal"));
        let replica = dir.path().join("sync/remote/replica");
        for name in fs::read_dir(&replica).unwrap().flatten() {
            let file = name.file_name();
            let text = file.to_string_lossy();
            if text.ends_with(".etag") {
                continue;
            }
            assert_eq!(text.len(), 64);
            assert!(!text.contains('/'));
        }
    }
}
