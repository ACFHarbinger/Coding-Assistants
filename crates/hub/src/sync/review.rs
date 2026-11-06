//! Owner conflict list/apply (S7 / #97).
//!
//! Choices: local | remote | keep-both | manual. Live `hub.db` is never
//! replaced. Audit JSON is hashes + choice only.

use super::lock::SyncLockGuard;
use super::types::sha256_hex;
use super::SyncError;
use crate::HubStore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictChoice {
    Local,
    Remote,
    KeepBoth,
    Manual,
}

impl ConflictChoice {
    pub fn parse(value: &str) -> Result<Self, SyncError> {
        match value.trim() {
            "local" => Ok(Self::Local),
            "remote" => Ok(Self::Remote),
            "keep-both" => Ok(Self::KeepBoth),
            "manual" => Ok(Self::Manual),
            _ => Err(SyncError::Invalid("unknown conflict choice".into())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
            Self::KeepBoth => "keep-both",
            Self::Manual => "manual",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictDecision {
    pub choice: ConflictChoice,
    pub decided_at: String,
    pub local_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub base_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictItem {
    pub slug: String,
    pub path: String,
    pub reason: String,
    pub decision: Option<ConflictDecision>,
    pub local_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub base_hash: Option<String>,
}

pub fn list_conflicts(home: &Path) -> Result<Vec<ConflictItem>, SyncError> {
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
        items.push(read_item(&entry.path(), &slug)?);
    }
    items.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(items)
}

pub fn apply_choice(
    store: &HubStore,
    slug: &str,
    choice: ConflictChoice,
) -> Result<ConflictDecision, SyncError> {
    validate_slug(slug)?;
    let home = store.data_dir();
    let dir = conflicts_root(home).join(slug);
    if !dir.is_dir() {
        return Err(SyncError::Invalid("conflict not found".into()));
    }
    let relative = resolve_path(&dir, slug)?;
    let hashes = side_hashes(&dir);
    let _guard = SyncLockGuard::acquire(home, "resolve")?;
    if choice == ConflictChoice::Remote && !dir.join("remote").is_file() {
        super::tombstone::confirm_delete_locked(store, slug)?;
    } else {
        apply_live(home, &dir, &relative, choice)?;
    }
    let decision = ConflictDecision {
        choice,
        decided_at: chrono::Utc::now().to_rfc3339(),
        local_hash: hashes.0.clone(),
        remote_hash: hashes.1.clone(),
        base_hash: hashes.2.clone(),
    };
    atomic_write(
        &dir.join("decision.json"),
        &serde_json::to_vec(&decision).map_err(|e| SyncError::Invalid(e.to_string()))?,
    )?;
    let payload = serde_json::json!({
        "kind": "sync-conflict-decision",
        "choice": choice.as_str(),
        "slug": slug,
        "local_hash": hashes.0,
        "remote_hash": hashes.1,
        "base_hash": hashes.2,
    });
    store
        .record_audit_for_sync(
            Path::new("sync"),
            &relative,
            "sync-conflict-decision",
            &payload.to_string(),
            None,
        )
        .map_err(|e| SyncError::Invalid(e.to_string()))?;
    Ok(decision)
}

fn apply_live(
    home: &Path,
    dir: &Path,
    relative: &Path,
    choice: ConflictChoice,
) -> Result<(), SyncError> {
    let db = is_live_db(relative);
    match choice {
        ConflictChoice::Manual => Ok(()),
        ConflictChoice::Local => restore_if_missing(home, dir, relative, "local"),
        ConflictChoice::Remote => {
            if db {
                return Err(SyncError::Invalid(
                    "applying remote must not replace live hub.db".into(),
                ));
            }
            let src = dir.join("remote");
            if !src.is_file() {
                return Err(SyncError::Invalid("conflict remote copy is missing".into()));
            }
            atomic_copy(&src, &home.join(relative))
        }
        ConflictChoice::KeepBoth => {
            if db {
                return Ok(());
            }
            restore_if_missing(home, dir, relative, "local")?;
            let remote = dir.join("remote");
            if remote.is_file() {
                atomic_copy(&remote, &remote_sibling(&home.join(relative)))?;
            }
            Ok(())
        }
    }
}

fn restore_if_missing(
    home: &Path,
    dir: &Path,
    relative: &Path,
    side: &str,
) -> Result<(), SyncError> {
    let live = home.join(relative);
    if live.is_file() {
        return Ok(());
    }
    let src = dir.join(side);
    if src.is_file() {
        atomic_copy(&src, &live)?;
    }
    Ok(())
}

fn read_item(dir: &Path, slug: &str) -> Result<ConflictItem, SyncError> {
    let hashes = side_hashes(dir);
    Ok(ConflictItem {
        slug: slug.to_string(),
        path: resolve_path(dir, slug)?.to_string_lossy().into_owned(),
        reason: fs::read_to_string(dir.join("reason.txt"))
            .unwrap_or_default()
            .trim()
            .to_string(),
        decision: fs::read(dir.join("decision.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok()),
        local_hash: hashes.0,
        remote_hash: hashes.1,
        base_hash: hashes.2,
    })
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

fn remote_sibling(live: &Path) -> PathBuf {
    match (live.file_stem(), live.extension()) {
        (Some(stem), Some(ext)) => live.with_file_name(format!(
            "{}.remote.{}",
            stem.to_string_lossy(),
            ext.to_string_lossy()
        )),
        (Some(stem), None) => live.with_file_name(format!("{}.remote", stem.to_string_lossy())),
        _ => live.with_extension("remote"),
    }
}

fn side_hashes(dir: &Path) -> (Option<String>, Option<String>, Option<String>) {
    (
        hash_file(&dir.join("local")),
        hash_file(&dir.join("remote")),
        hash_file(&dir.join("base")),
    )
}

fn hash_file(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|b| sha256_hex(&b))
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

fn atomic_copy(from: &Path, to: &Path) -> Result<(), SyncError> {
    atomic_write(to, &fs::read(from).map_err(io_err)?)
}

fn io_err(error: std::io::Error) -> SyncError {
    SyncError::Invalid(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn stage(home: &Path, slug: &str, path: Option<&str>, local: &[u8], remote: &[u8]) {
        let dir = conflicts_root(home).join(slug);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("reason.txt"), "same-path-edit").unwrap();
        fs::write(dir.join("local"), local).unwrap();
        fs::write(dir.join("remote"), remote).unwrap();
        if let Some(path) = path {
            fs::write(dir.join("path.txt"), path).unwrap();
        }
    }

    fn write_live(home: &Path, rel: &str, bytes: &[u8]) {
        let path = home.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn no_file_missing_or_empty_conflicts_dir() {
        let dir = tempdir().unwrap();
        assert!(list_conflicts(dir.path()).unwrap().is_empty());
        fs::create_dir_all(conflicts_root(dir.path())).unwrap();
        assert!(list_conflicts(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn legacy_dir_lists_and_applies_without_path_or_decision() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        stage(dir.path(), "markdown__note.md", None, b"L", b"R");
        write_live(dir.path(), "markdown/note.md", b"L");
        let listed = list_conflicts(dir.path()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].path, "markdown/note.md");
        assert!(listed[0].decision.is_none());
        let decision = apply_choice(&store, "markdown__note.md", ConflictChoice::Remote).unwrap();
        assert_eq!(decision.choice, ConflictChoice::Remote);
        assert_eq!(fs::read(dir.path().join("markdown/note.md")).unwrap(), b"R");
        assert_eq!(
            fs::read(dir.path().join("sync/conflicts/markdown__note.md/local")).unwrap(),
            b"L"
        );
        assert_eq!(
            fs::read(dir.path().join("sync/conflicts/markdown__note.md/remote")).unwrap(),
            b"R"
        );
        let json = store.list_audit_events(false).unwrap();
        let row = json
            .iter()
            .find(|e| e.operation == "sync-conflict-decision")
            .unwrap();
        assert!(row.process_json.contains("\"choice\":\"remote\""));
        assert!(!row.process_json.contains("token"));
        assert!(!row.process_json.contains("cloud-sync.key"));
        assert!(!row.process_json.contains("\"L\""));
    }

    #[test]
    fn malformed_slug_and_choice_fail_closed() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        stage(
            dir.path(),
            "markdown__note.md",
            Some("markdown/note.md"),
            b"L",
            b"R",
        );
        write_live(dir.path(), "markdown/note.md", b"L");
        let err = apply_choice(&store, "..", ConflictChoice::Local).unwrap_err();
        assert!(err.to_string().contains("invalid conflict slug"));
        assert!(ConflictChoice::parse("overwrite").is_err());
        assert_eq!(fs::read(dir.path().join("markdown/note.md")).unwrap(), b"L");
    }

    #[test]
    fn interrupted_write_leaves_live_and_conflict_copies() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        stage(
            dir.path(),
            "markdown__note.md",
            Some("markdown/note.md"),
            b"L",
            b"R",
        );
        write_live(dir.path(), "markdown/note.md", b"L");
        let parent = dir.path().join("markdown");
        let mut perms = fs::metadata(&parent).unwrap().permissions();
        let original = perms.clone();
        perms.set_readonly(true);
        fs::set_permissions(&parent, perms).unwrap();
        let err = apply_choice(&store, "markdown__note.md", ConflictChoice::Remote).unwrap_err();
        let _ = fs::set_permissions(&parent, original);
        assert!(!err.to_string().is_empty());
        assert_eq!(fs::read(dir.path().join("markdown/note.md")).unwrap(), b"L");
        let cdir = conflicts_root(dir.path()).join("markdown__note.md");
        assert_eq!(fs::read(cdir.join("local")).unwrap(), b"L");
        assert_eq!(fs::read(cdir.join("remote")).unwrap(), b"R");
        assert!(!cdir.join("decision.json").exists());
    }

    #[test]
    fn remote_hub_db_fails_closed_keep_both_and_local_leave_live() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let live = fs::read(dir.path().join("hub.db")).unwrap();
        stage(
            dir.path(),
            "hub.db",
            Some("hub.db"),
            b"local-snap",
            b"remote-snap",
        );
        let err = apply_choice(&store, "hub.db", ConflictChoice::Remote).unwrap_err();
        assert!(err.to_string().contains("hub.db"));
        assert_eq!(fs::read(dir.path().join("hub.db")).unwrap(), live);
        apply_choice(&store, "hub.db", ConflictChoice::KeepBoth).unwrap();
        assert_eq!(fs::read(dir.path().join("hub.db")).unwrap(), live);
        assert!(!dir.path().join("hub.remote.db").exists());
        apply_choice(&store, "hub.db", ConflictChoice::Local).unwrap();
        assert_eq!(fs::read(dir.path().join("hub.db")).unwrap(), live);
        assert_eq!(
            fs::read(dir.path().join("sync/conflicts/hub.db/remote")).unwrap(),
            b"remote-snap"
        );
    }

    #[test]
    fn keep_both_writes_sibling_and_local_restores_missing() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        stage(
            dir.path(),
            "markdown__note.md",
            Some("markdown/note.md"),
            b"L",
            b"R",
        );
        write_live(dir.path(), "markdown/note.md", b"L");
        apply_choice(&store, "markdown__note.md", ConflictChoice::KeepBoth).unwrap();
        assert_eq!(fs::read(dir.path().join("markdown/note.md")).unwrap(), b"L");
        assert_eq!(
            fs::read(dir.path().join("markdown/note.remote.md")).unwrap(),
            b"R"
        );
        fs::remove_file(dir.path().join("markdown/note.md")).unwrap();
        apply_choice(&store, "markdown__note.md", ConflictChoice::Local).unwrap();
        assert_eq!(fs::read(dir.path().join("markdown/note.md")).unwrap(), b"L");
        apply_choice(&store, "markdown__note.md", ConflictChoice::Manual).unwrap();
        assert_eq!(fs::read(dir.path().join("markdown/note.md")).unwrap(), b"L");
    }
}
