//! Encrypt then store on a [`DriveClient`] (S2 / #92).
//!
//! Refuses local-only categories so keys and staging cannot be uploaded even
//! to FakeDrive. Leak tests scan stored ciphertexts for forbidden bytes.

use super::client::DriveClient;
use super::crypto::encrypt_object;
use super::key::CloudSyncKey;
use super::policy::{self, classify};
use super::types::{BlobId, ObjectKind, RemotePrefix, SyncConfig};
use super::SyncError;
use std::path::Path;

/// Seal plaintext and put it. Local-only categories are rejected before encrypt.
pub fn seal_and_put(
    drive: &mut impl DriveClient,
    prefix: &RemotePrefix,
    key: &CloudSyncKey,
    config: &SyncConfig,
    relative_path: &Path,
    kind: ObjectKind,
    plaintext: &[u8],
) -> Result<BlobId, SyncError> {
    let category = classify(relative_path);
    if !policy::may_upload(config.policy_for(category)) {
        return Err(SyncError::Invalid("category is not uploadable".into()));
    }
    let sealed = encrypt_object(key, kind, plaintext)?;
    let blob_id = BlobId::from_encrypted_bytes(&sealed);
    drive.put_if_unmodified(prefix, &blob_id, &sealed, None)?;
    Ok(blob_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::client::FakeDrive;
    use crate::sync::crypto::decrypt_object;
    use crate::sync::key::CloudSyncKey;
    use crate::sync::layout::replica_prefix;
    use crate::sync::types::{Category, DeviceId, Manifest, ManifestEntry};
    use tempfile::tempdir;

    const JOURNAL_BODY: &[u8] = b"secret-journal-body";
    const REFRESH: &[u8] = b"ya29.leak-me-refresh-token";
    const FERNET: &[u8] = b"<!--ENC-->gAAAAABjournalfernet";

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty()
            && haystack
                .windows(needle.len())
                .any(|window| window == needle)
    }

    #[test]
    fn local_only_paths_never_reach_the_fake_provider() {
        let dir = tempdir().unwrap();
        let key = CloudSyncKey::create(dir.path()).unwrap();
        let mut drive = FakeDrive::new();
        let config = SyncConfig::v1_defaults();
        let prefix = replica_prefix();
        for path in [
            "keys/cloud-sync.key",
            "keys/journals/claude.key",
            "sync/lock",
            "sync/staging/part",
        ] {
            let err = seal_and_put(
                &mut drive,
                &prefix,
                &key,
                &config,
                Path::new(path),
                ObjectKind::Blob,
                b"should-not-upload",
            )
            .unwrap_err();
            assert_eq!(err, SyncError::Invalid("category is not uploadable".into()));
        }
        assert!(drive.ciphertext_blobs().is_empty());
    }

    #[test]
    fn fake_provider_does_not_see_plaintext_keys_or_paths() {
        let dir = tempdir().unwrap();
        let key = CloudSyncKey::create(dir.path()).unwrap();
        let mut drive = FakeDrive::new();
        let config = SyncConfig::v1_defaults();
        let prefix = replica_prefix();

        let mut journal = b"# claude\n".to_vec();
        journal.extend_from_slice(FERNET);
        journal.extend_from_slice(b"\n");
        journal.extend_from_slice(JOURNAL_BODY);
        journal.extend_from_slice(b"\n");
        journal.extend_from_slice(REFRESH);

        let blob_id = seal_and_put(
            &mut drive,
            &prefix,
            &key,
            &config,
            Path::new("journals/claude.md"),
            ObjectKind::Blob,
            &journal,
        )
        .unwrap();

        let manifest = Manifest {
            format_version: 1,
            device_id: DeviceId::generate(),
            schema_version: Some("3".into()),
            entries: vec![ManifestEntry {
                blob_id: blob_id.clone(),
                relative_path: "journals/claude.md".into(),
                category: Category::PrivateJournal,
                content_hash: "abc".into(),
                size: journal.len() as u64,
                kind: ObjectKind::Blob,
            }],
        };
        let manifest_json = serde_json::to_vec(&manifest).unwrap();
        seal_and_put(
            &mut drive,
            &prefix,
            &key,
            &config,
            Path::new("markdown/export.md"),
            ObjectKind::Manifest,
            &manifest_json,
        )
        .unwrap();

        let stored = drive.ciphertext_blobs();
        assert_eq!(stored.len(), 2);
        let forbidden: &[&[u8]] = &[
            key.expose().as_slice(),
            JOURNAL_BODY,
            REFRESH,
            FERNET,
            b"cloud-sync.key",
            b"journals/claude.key",
            b"journals/claude.md",
            b"should-not-upload",
        ];
        for blob in &stored {
            for needle in forbidden {
                assert!(
                    !contains(blob, needle),
                    "ciphertext leaked {} bytes of forbidden material",
                    needle.len()
                );
            }
            assert!(blob.starts_with(super::super::crypto::MAGIC));
        }

        let opened = decrypt_object(&key, &drive.get(&blob_id).unwrap()).unwrap();
        assert_eq!(opened.0, ObjectKind::Blob);
        assert_eq!(opened.1, journal);
    }
}
