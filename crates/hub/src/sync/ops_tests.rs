use super::*;
use crate::sync::client::{DriveClient, FakeDrive};
use crate::sync::crypto::decrypt_object;
use crate::sync::key::CloudSyncKey;
use crate::sync::layout::replica_prefix;
use crate::sync::plan::run_locked;
use crate::sync::snapshot::upload_home;
use crate::sync::types::{sha256_hex, ObjectKind, SyncConfig};
use std::fs;
use tempfile::tempdir;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn seed_two_files(home: &std::path::Path) {
    fs::create_dir_all(home.join("journals")).unwrap();
    fs::create_dir_all(home.join("markdown")).unwrap();
    fs::write(home.join("journals/claude.md"), b"journal-one").unwrap();
    fs::write(home.join("markdown/export.md"), b"notes-two").unwrap();
}

fn manifest_entries(
    key: &CloudSyncKey,
    drive: &FakeDrive,
) -> Vec<crate::sync::types::ManifestEntry> {
    let mut found = None;
    for object in drive.list(&replica_prefix()).unwrap() {
        let sealed = drive.get(&object.blob_id).unwrap();
        let Ok((ObjectKind::Manifest, plain)) = decrypt_object(key, &sealed) else {
            continue;
        };
        let manifest: crate::sync::types::Manifest = serde_json::from_slice(&plain).unwrap();
        found = Some(manifest.entries);
    }
    found.unwrap_or_default()
}

fn manifest_count(key: &CloudSyncKey, drive: &FakeDrive) -> usize {
    drive
        .list(&replica_prefix())
        .unwrap()
        .iter()
        .filter(|object| {
            decrypt_object(key, &drive.get(&object.blob_id).unwrap())
                .map(|(kind, _)| kind == ObjectKind::Manifest)
                .unwrap_or(false)
        })
        .count()
}

#[test]
fn resume_skips_re_put_of_done_content_hash() {
    let dir = tempdir().unwrap();
    let key = CloudSyncKey::create(dir.path()).unwrap();
    seed_two_files(dir.path());
    let journal = fs::read(dir.path().join("journals/claude.md")).unwrap();
    let hash = sha256_hex(&journal);
    let reused = BlobId::parse(&"ab".repeat(32)).unwrap();
    save_resume(
        dir.path(),
        &ResumeState {
            action: "up".into(),
            done: vec![ResumeDone {
                content_hash: hash.clone(),
                blob_id: reused.clone(),
                relative_path: "journals/claude.md".into(),
                size: journal.len() as u64,
                kind: ObjectKind::Blob,
            }],
        },
    )
    .unwrap();
    let mut drive = FakeDrive::new();
    upload_home(dir.path(), &key, &mut drive, &SyncConfig::v1_defaults(), 3).unwrap();
    let entries = manifest_entries(&key, &drive);
    let journal_entry = entries
        .iter()
        .find(|row| row.relative_path == "journals/claude.md")
        .unwrap();
    assert_eq!(journal_entry.blob_id, reused);
    assert_eq!(journal_entry.content_hash, hash);
}

#[test]
fn quota_max_objects_one_keeps_resume_and_skips_manifest() {
    let dir = tempdir().unwrap();
    let key = CloudSyncKey::create(dir.path()).unwrap();
    seed_two_files(dir.path());
    write_limits(
        dir.path(),
        &SyncLimits {
            max_objects: 1,
            max_bytes: DEFAULT_MAX_BYTES,
            max_concurrent: DEFAULT_MAX_CONCURRENT,
        },
    )
    .unwrap();
    let mut drive = FakeDrive::new();
    let result = upload_home(dir.path(), &key, &mut drive, &SyncConfig::v1_defaults(), 3).unwrap();
    assert!(result.warnings.iter().any(|row| row == QUOTA_WARNING));
    assert_eq!(manifest_count(&key, &drive), 0);
    assert!(!dir.path().join("sync/last-verified.json").exists());
    let resume = load_resume(dir.path()).unwrap().expect("resume kept");
    assert_eq!(resume.done.len(), 1);
}

#[test]
fn retry_without_resume_is_invalid() {
    let dir = tempdir().unwrap();
    CloudSyncKey::create(dir.path()).unwrap();
    let err = run_locked(dir.path(), 3, "retry").unwrap_err();
    assert!(matches!(err, crate::sync::SyncError::Invalid(message) if message.contains("resume")));
}

#[test]
fn limits_json_has_no_secrets() {
    let dir = tempdir().unwrap();
    let limits = ensure_limits(dir.path()).unwrap();
    let json = fs::read_to_string(dir.path().join("sync/limits.json")).unwrap();
    assert_eq!(limits.max_objects, DEFAULT_MAX_OBJECTS);
    assert_eq!(limits.max_bytes, DEFAULT_MAX_BYTES);
    assert_eq!(limits.max_concurrent, DEFAULT_MAX_CONCURRENT);
    assert!(!json.contains("token"));
    assert!(!json.contains("key"));
    assert!(!json.contains("ya29"));
    assert!(!json.contains("Bearer"));
    assert!(!json.contains("cloud-sync.key"));
    assert!(!contains(json.as_bytes(), b"cloud-sync.key"));
}
