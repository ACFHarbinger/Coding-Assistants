//! Encrypted snapshot upload/download (S5 / #95).
//!
//! Works over any [`DriveClient`]. Tests use FakeDrive. CLI/desktop use
//! [`super::fs_drive::FsDrive`]. Live `hub.db` is never replaced.

use super::client::DriveClient;
use super::crypto::decrypt_object;
use super::key::CloudSyncKey;
use super::layout::replica_prefix;
use super::ops::{self, ResumeDone};
use super::pack::seal_and_put;
use super::policy::{self, classify};
use super::trust;
use super::types::{
    sha256_hex, BlobId, Category, Manifest, ManifestEntry, ObjectKind, SyncConfig, SyncResult,
};
use super::SyncError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const LIVE_DB_WARNING: &str = "live hub.db was not replaced";

#[derive(Serialize, Deserialize)]
struct LastVerified {
    schema_version: String,
    base: String,
}

pub fn upload_home(
    home: &Path,
    key: &CloudSyncKey,
    drive: &mut impl DriveClient,
    config: &SyncConfig,
    schema: i64,
) -> Result<SyncResult, SyncError> {
    let device = trust::ensure_local_device(home)?;
    let limits = ops::ensure_limits(home)?;
    let mut resume = ops::begin_resume(home, "up")?;
    let prefix = replica_prefix();
    let files = collect_uploadable(home, config);
    let mut entries = Vec::new();
    let mut keep = BTreeSet::new();
    let mut put_count = 0usize;
    for relative in files {
        let bytes = snapshot_bytes(home, &relative)?;
        let kind = object_kind(&relative);
        let content_hash = sha256_hex(&bytes);
        let relative_path = relative.to_string_lossy().replace('\\', "/");
        if let Some(done) = ops::find_done(&resume, &content_hash, kind) {
            keep.insert(done.blob_id.clone());
            entries.push(ManifestEntry {
                blob_id: done.blob_id.clone(),
                relative_path,
                category: classify(&relative),
                content_hash,
                size: done.size,
                kind: done.kind,
            });
            continue;
        }
        if ops::would_exceed(&limits, &resume, bytes.len() as u64) {
            ops::save_resume(home, &resume)?;
            return Ok(SyncResult {
                uploaded: put_count,
                warnings: vec![ops::QUOTA_WARNING.into(), LIVE_DB_WARNING.into()],
                ..SyncResult::default()
            });
        }
        let blob_id = seal_and_put(drive, &prefix, key, config, &relative, kind, &bytes)?;
        keep.insert(blob_id.clone());
        ops::record_done(
            home,
            &mut resume,
            ResumeDone {
                content_hash: content_hash.clone(),
                blob_id: blob_id.clone(),
                relative_path: relative_path.clone(),
                size: bytes.len() as u64,
                kind,
            },
        )?;
        put_count += 1;
        entries.push(ManifestEntry {
            blob_id,
            relative_path,
            category: classify(&relative),
            content_hash,
            size: bytes.len() as u64,
            kind,
        });
    }
    let uploaded = entries.len();
    let manifest = Manifest {
        format_version: 1,
        device_id: device.id,
        schema_version: Some(schema.to_string()),
        entries,
    };
    let manifest_bytes = serde_json::to_vec(&manifest)
        .map_err(|_| SyncError::Invalid("failed to encode manifest".into()))?;
    let base = seal_and_put(
        drive,
        &prefix,
        key,
        config,
        Path::new("markdown/export.md"),
        ObjectKind::Manifest,
        &manifest_bytes,
    )?;
    keep.insert(base.clone());
    prune_stale(drive, &keep)?;
    write_last_verified(home, schema, &base)?;
    super::merge::remember_base(home)?;
    ops::clear_resume(home)?;
    Ok(SyncResult {
        uploaded: uploaded + 1,
        warnings: vec![LIVE_DB_WARNING.into()],
        ..SyncResult::default()
    })
}

pub fn download_home(
    home: &Path,
    key: &CloudSyncKey,
    drive: &impl DriveClient,
    config: &SyncConfig,
) -> Result<SyncResult, SyncError> {
    let listed = drive.list(&replica_prefix())?;
    let (base, manifest) = load_manifest(key, drive, &listed)?;
    if !trust::may_accept(home, &manifest.device_id) {
        return Err(SyncError::Invalid("device is not trusted".into()));
    }
    let staging = home.join("sync").join("staging").join("restore");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(io_err)?;
    }
    fs::create_dir_all(&staging).map_err(io_err)?;
    for entry in &manifest.entries {
        if !policy::may_upload(config.policy_for(entry.category)) {
            continue;
        }
        let sealed = drive.get(&entry.blob_id)?;
        let (kind, plain) = decrypt_object(key, &sealed)?;
        if kind != entry.kind || sha256_hex(&plain) != entry.content_hash {
            return Err(SyncError::Invalid(
                "snapshot entry failed verification".into(),
            ));
        }
        let dest = staging_join(&staging, &entry.relative_path)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        fs::write(&dest, plain).map_err(io_err)?;
    }
    let base_dir = home.join("sync").join("staging").join("base");
    let mut result = super::merge::reconcile(home, &base_dir, &staging)?;
    if super::rebase::try_rebase_homes(home, &staging)?.is_some() {
        result.warnings.push("audit fork rebased".into());
    }
    let schema = manifest
        .schema_version
        .as_deref()
        .unwrap_or("0")
        .parse()
        .map_err(|_| SyncError::Invalid("replica schema is not an integer".into()))?;
    write_last_verified(home, schema, &base)?;
    super::merge::remember_base(home)?;
    result.warnings.push(LIVE_DB_WARNING.into());
    result.warnings.dedup();
    Ok(result)
}

fn load_manifest(
    key: &CloudSyncKey,
    drive: &impl DriveClient,
    listed: &[super::client::RemoteObject],
) -> Result<(BlobId, Manifest), SyncError> {
    let mut found = None;
    for object in listed {
        let sealed = drive.get(&object.blob_id)?;
        let Ok((ObjectKind::Manifest, plain)) = decrypt_object(key, &sealed) else {
            continue;
        };
        if found.is_some() {
            return Err(SyncError::Invalid("replica has multiple manifests".into()));
        }
        let manifest = serde_json::from_slice(&plain)
            .map_err(|_| SyncError::Invalid("replica manifest is not json".into()))?;
        found = Some((object.blob_id.clone(), manifest));
    }
    found.ok_or_else(|| SyncError::Invalid("replica manifest is missing".into()))
}

fn prune_stale(drive: &mut impl DriveClient, keep: &BTreeSet<BlobId>) -> Result<(), SyncError> {
    for object in drive.list(&replica_prefix())? {
        if keep.contains(&object.blob_id) {
            continue;
        }
        match drive.delete_if_match(&object.blob_id, &object.etag) {
            Ok(()) | Err(SyncError::NotFound) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn collect_uploadable(home: &Path, config: &SyncConfig) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit(home, home, config, &mut files);
    files.sort();
    files
}

fn visit(home: &Path, dir: &Path, config: &SyncConfig, files: &mut Vec<PathBuf>) {
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
            visit(home, &path, config, files);
            continue;
        }
        let relative = path.strip_prefix(home).unwrap_or(&path);
        if skip_upload(relative, config) {
            continue;
        }
        files.push(relative.to_path_buf());
    }
}

fn skip_upload(relative: &Path, config: &SyncConfig) -> bool {
    if !policy::may_upload(config.policy_for(classify(relative))) {
        return true;
    }
    matches!(relative.to_str(), Some("hub.db-wal" | "hub.db-shm"))
}

fn object_kind(relative: &Path) -> ObjectKind {
    if classify(relative) == Category::HubDatabase {
        ObjectKind::Snapshot
    } else {
        ObjectKind::Blob
    }
}

fn snapshot_bytes(home: &Path, relative: &Path) -> Result<Vec<u8>, SyncError> {
    if relative == Path::new("hub.db") {
        let staging = home.join("sync").join("staging").join("source");
        fs::create_dir_all(&staging).map_err(io_err)?;
        let copy = staging.join("hub.db");
        fs::copy(home.join("hub.db"), &copy).map_err(io_err)?;
        return fs::read(copy).map_err(io_err);
    }
    fs::read(home.join(relative)).map_err(io_err)
}

fn staging_join(staging: &Path, relative: &str) -> Result<PathBuf, SyncError> {
    let rel = Path::new(relative);
    if rel.is_absolute() || rel.components().any(|c| c.as_os_str() == "..") {
        return Err(SyncError::Invalid("manifest path is not relative".into()));
    }
    Ok(staging.join(rel))
}

fn write_last_verified(home: &Path, schema: i64, base: &BlobId) -> Result<(), SyncError> {
    fs::create_dir_all(home.join("sync")).map_err(io_err)?;
    let body = LastVerified {
        schema_version: schema.to_string(),
        base: base.as_str().to_string(),
    };
    let json = serde_json::to_vec(&body)
        .map_err(|_| SyncError::Invalid("failed to encode last-verified".into()))?;
    fs::write(home.join("sync").join("last-verified.json"), json).map_err(io_err)
}

fn io_err(error: std::io::Error) -> SyncError {
    SyncError::Invalid(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::client::FakeDrive;
    use crate::sync::layout::replica_prefix;
    use tempfile::tempdir;

    fn leak_needles<'a>(key: &'a CloudSyncKey, journal: &'a [u8]) -> Vec<&'a [u8]> {
        vec![
            key.expose().as_slice(),
            journal,
            b"cloud-sync.key",
            b"journals/claude.md",
            b"ya29.",
        ]
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty()
            && haystack
                .windows(needle.len())
                .any(|window| window == needle)
    }

    fn seed_pair() -> (
        tempfile::TempDir,
        tempfile::TempDir,
        CloudSyncKey,
        FakeDrive,
    ) {
        let a = tempdir().unwrap();
        let b = tempdir().unwrap();
        let key = CloudSyncKey::create(a.path()).unwrap();
        CloudSyncKey::import(b.path(), key.expose()).unwrap();
        (a, b, key, FakeDrive::new())
    }

    fn trust_peer(src: &Path, dest: &Path) {
        let device = crate::sync::trust::ensure_local_device(src).unwrap();
        crate::sync::trust::register(dest, device.id).unwrap();
    }

    #[test]
    fn two_devices_restore_without_replacing_live_hub_db() {
        let (a, b, key, mut drive) = seed_pair();
        fs::create_dir_all(a.path().join("journals")).unwrap();
        fs::create_dir_all(a.path().join("markdown")).unwrap();
        let journal = b"# claude\nsecret-journal-body\n";
        fs::write(a.path().join("journals/claude.md"), journal).unwrap();
        fs::write(a.path().join("markdown/export.md"), b"notes").unwrap();
        fs::write(a.path().join("hub.db"), b"device-a-db").unwrap();
        fs::write(a.path().join("hub.db-wal"), b"wal-must-stay-local").unwrap();
        fs::write(b.path().join("hub.db"), b"device-b-live-db").unwrap();
        fs::create_dir_all(a.path().join("keys/journals")).unwrap();
        fs::write(a.path().join("keys/journals/claude.key"), b"fernet-key").unwrap();

        let config = SyncConfig::v1_defaults();
        upload_home(a.path(), &key, &mut drive, &config, 3).unwrap();
        trust_peer(a.path(), b.path());
        let junk = BlobId::from_encrypted_bytes(b"not-cas1");
        drive
            .put_if_unmodified(&replica_prefix(), &junk, b"not-cas1", None)
            .unwrap();
        download_home(b.path(), &key, &drive, &config).unwrap();

        assert_eq!(
            fs::read(b.path().join("journals/claude.md")).unwrap(),
            journal
        );
        assert_eq!(
            fs::read(b.path().join("markdown/export.md")).unwrap(),
            b"notes"
        );
        assert_eq!(
            fs::read(b.path().join("hub.db")).unwrap(),
            b"device-b-live-db"
        );
        assert_eq!(
            fs::read(b.path().join("sync/staging/restore/hub.db")).unwrap(),
            b"device-a-db"
        );
        assert!(!b.path().join("keys/journals/claude.key").exists());
        assert!(!b.path().join("hub.db-wal").exists());
        assert!(!b.path().join("sync/staging/restore/hub.db-wal").exists());

        let listed = drive.list(&replica_prefix()).unwrap();
        assert!(listed.iter().all(|row| row.blob_id.as_str().len() == 64));
        assert!(listed.iter().all(|row| !row.blob_id.as_str().contains('/')));
        for blob in drive.ciphertext_blobs() {
            for needle in leak_needles(&key, journal) {
                assert!(!contains(blob, needle));
            }
        }
    }

    #[test]
    fn tamper_and_missing_entry_fail_closed() {
        let (a, b, key, mut drive) = seed_pair();
        fs::create_dir_all(a.path().join("journals")).unwrap();
        fs::write(a.path().join("journals/claude.md"), b"keep-me").unwrap();
        fs::create_dir_all(b.path().join("journals")).unwrap();
        fs::write(b.path().join("journals/claude.md"), b"device-b").unwrap();
        let config = SyncConfig::v1_defaults();
        upload_home(a.path(), &key, &mut drive, &config, 3).unwrap();
        trust_peer(a.path(), b.path());

        let object = drive.list(&replica_prefix()).unwrap().pop().unwrap();
        let mut bytes = drive.get(&object.blob_id).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        drive
            .put_if_unmodified(
                &replica_prefix(),
                &object.blob_id,
                &bytes,
                Some(&object.etag),
            )
            .unwrap();
        assert!(download_home(b.path(), &key, &drive, &config).is_err());
        assert_eq!(
            fs::read(b.path().join("journals/claude.md")).unwrap(),
            b"device-b"
        );

        let (a2, b2, key2, mut drive2) = seed_pair();
        fs::create_dir_all(a2.path().join("journals")).unwrap();
        fs::write(a2.path().join("journals/claude.md"), b"keep-me").unwrap();
        fs::create_dir_all(b2.path().join("journals")).unwrap();
        fs::write(b2.path().join("journals/claude.md"), b"device-b").unwrap();
        upload_home(a2.path(), &key2, &mut drive2, &config, 3).unwrap();
        trust_peer(a2.path(), b2.path());
        let victim = drive2
            .list(&replica_prefix())
            .unwrap()
            .into_iter()
            .find(|row| {
                decrypt_object(&key2, &drive2.get(&row.blob_id).unwrap())
                    .map(|(kind, _)| kind != ObjectKind::Manifest)
                    .unwrap_or(false)
            })
            .unwrap();
        drive2
            .delete_if_match(&victim.blob_id, &victim.etag)
            .unwrap();
        assert!(download_home(b2.path(), &key2, &drive2, &config).is_err());
        assert_eq!(
            fs::read(b2.path().join("journals/claude.md")).unwrap(),
            b"device-b"
        );
    }
}
