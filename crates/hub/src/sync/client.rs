//! Provider-neutral storage contract and an in-memory FakeDrive (S1).

use super::types::{BlobId, ETag, RemotePrefix};
use super::SyncError;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteObject {
    pub blob_id: BlobId,
    pub etag: ETag,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct ReplicaPut {
    pub prefix: RemotePrefix,
    pub blob_id: BlobId,
    pub bytes: Vec<u8>,
    pub expected_etag: Option<ETag>,
}

#[derive(Clone, Debug, Default)]
pub struct ReplicaAdvance {
    pub puts: Vec<ReplicaPut>,
    pub prune_prefix: Option<RemotePrefix>,
}

/// Storage operations every provider must implement. No merge, no keys.
pub trait DriveClient {
    fn list(&self, prefix: &RemotePrefix) -> Result<Vec<RemoteObject>, SyncError>;
    fn get(&self, id: &BlobId) -> Result<Vec<u8>, SyncError>;
    fn put_if_unmodified(
        &mut self,
        prefix: &RemotePrefix,
        id: &BlobId,
        bytes: &[u8],
        expected: Option<&ETag>,
    ) -> Result<ETag, SyncError>;
    fn delete_if_match(&mut self, id: &BlobId, etag: &ETag) -> Result<(), SyncError>;
    fn advance_replica(&mut self, plan: ReplicaAdvance) -> Result<(), SyncError>;
}

#[derive(Clone)]
struct Stored {
    bytes: Vec<u8>,
    etag: ETag,
    prefix: RemotePrefix,
}

/// In-memory DriveClient for unit tests. Names are BlobIds only.
#[derive(Clone, Default)]
pub struct FakeDrive {
    objects: BTreeMap<BlobId, Stored>,
    by_prefix: BTreeMap<RemotePrefix, BTreeSet<BlobId>>,
}

impl FakeDrive {
    pub fn new() -> Self {
        Self::default()
    }

    fn apply_put(
        &mut self,
        prefix: &RemotePrefix,
        id: &BlobId,
        bytes: &[u8],
        expected: Option<&ETag>,
    ) -> Result<ETag, SyncError> {
        match (self.objects.get(id), expected) {
            (Some(stored), Some(etag)) if stored.etag == *etag => {}
            (None, None) => {}
            _ => return Err(SyncError::Precondition),
        }
        let etag = ETag::new(super::types::sha256_hex(bytes));
        if let Some(old) = self.objects.remove(id) {
            if let Some(set) = self.by_prefix.get_mut(&old.prefix) {
                set.remove(id);
            }
        }
        self.by_prefix
            .entry(prefix.clone())
            .or_default()
            .insert(id.clone());
        self.objects.insert(
            id.clone(),
            Stored {
                bytes: bytes.to_vec(),
                etag: etag.clone(),
                prefix: prefix.clone(),
            },
        );
        Ok(etag)
    }

    fn apply_delete(&mut self, id: &BlobId, etag: &ETag) -> Result<(), SyncError> {
        let Some(stored) = self.objects.get(id) else {
            return Err(SyncError::NotFound);
        };
        if stored.etag != *etag {
            return Err(SyncError::Precondition);
        }
        let stored = self.objects.remove(id).expect("checked");
        if let Some(set) = self.by_prefix.get_mut(&stored.prefix) {
            set.remove(id);
        }
        Ok(())
    }
}

impl DriveClient for FakeDrive {
    fn list(&self, prefix: &RemotePrefix) -> Result<Vec<RemoteObject>, SyncError> {
        let Some(ids) = self.by_prefix.get(prefix) else {
            return Ok(Vec::new());
        };
        Ok(ids
            .iter()
            .filter_map(|id| {
                let stored = self.objects.get(id)?;
                Some(RemoteObject {
                    blob_id: id.clone(),
                    etag: stored.etag.clone(),
                    size: stored.bytes.len() as u64,
                })
            })
            .collect())
    }

    fn get(&self, id: &BlobId) -> Result<Vec<u8>, SyncError> {
        self.objects
            .get(id)
            .map(|stored| stored.bytes.clone())
            .ok_or(SyncError::NotFound)
    }

    fn put_if_unmodified(
        &mut self,
        prefix: &RemotePrefix,
        id: &BlobId,
        bytes: &[u8],
        expected: Option<&ETag>,
    ) -> Result<ETag, SyncError> {
        self.apply_put(prefix, id, bytes, expected)
    }

    fn delete_if_match(&mut self, id: &BlobId, etag: &ETag) -> Result<(), SyncError> {
        self.apply_delete(id, etag)
    }

    fn advance_replica(&mut self, plan: ReplicaAdvance) -> Result<(), SyncError> {
        let mut next = self.clone();
        for put in &plan.puts {
            next.apply_put(
                &put.prefix,
                &put.blob_id,
                &put.bytes,
                put.expected_etag.as_ref(),
            )?;
        }
        if let Some(prefix) = &plan.prune_prefix {
            let ids: Vec<BlobId> = next
                .by_prefix
                .get(prefix)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default();
            for id in ids {
                let etag = next
                    .objects
                    .get(&id)
                    .ok_or(SyncError::NotFound)?
                    .etag
                    .clone();
                next.apply_delete(&id, &etag)?;
            }
        }
        *self = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::layout::{device_prefix, replica_prefix};
    use crate::sync::types::DeviceId;

    fn blob(bytes: &[u8]) -> BlobId {
        BlobId::from_encrypted_bytes(bytes)
    }

    #[test]
    fn list_names_are_hashed_blob_ids() {
        let mut drive = FakeDrive::new();
        let prefix = replica_prefix();
        let bytes = b"enc-bytes";
        let id = blob(bytes);
        drive.put_if_unmodified(&prefix, &id, bytes, None).unwrap();
        let listed = drive.list(&prefix).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].blob_id, id);
        assert_eq!(listed[0].blob_id.as_str().len(), 64);
        assert!(!listed[0].blob_id.as_str().contains("enc"));
        assert!(!listed[0].blob_id.as_str().contains('/'));
    }

    #[test]
    fn stale_etag_is_rejected() {
        let mut drive = FakeDrive::new();
        let prefix = replica_prefix();
        let id = blob(b"a");
        let etag = drive.put_if_unmodified(&prefix, &id, b"a", None).unwrap();
        let err = drive
            .put_if_unmodified(&prefix, &id, b"b", Some(&ETag::new("nope")))
            .unwrap_err();
        assert_eq!(err, SyncError::Precondition);
        drive
            .put_if_unmodified(&prefix, &id, b"b", Some(&etag))
            .unwrap();
        assert_eq!(drive.get(&id).unwrap(), b"b");
    }

    #[test]
    fn replica_advance_prunes_device_prefix_atomically() {
        let mut drive = FakeDrive::new();
        let device = DeviceId::generate();
        let device_prefix = device_prefix(&device).unwrap();
        let replica = replica_prefix();
        let device_blob = blob(b"device-upload");
        let replica_blob = blob(b"replica-object");
        drive
            .put_if_unmodified(&device_prefix, &device_blob, b"device-upload", None)
            .unwrap();

        drive
            .advance_replica(ReplicaAdvance {
                puts: vec![ReplicaPut {
                    prefix: replica.clone(),
                    blob_id: replica_blob.clone(),
                    bytes: b"replica-object".to_vec(),
                    expected_etag: None,
                }],
                prune_prefix: Some(device_prefix.clone()),
            })
            .unwrap();

        assert!(drive.list(&device_prefix).unwrap().is_empty());
        assert_eq!(drive.get(&replica_blob).unwrap(), b"replica-object");
        assert!(drive.get(&device_blob).is_err());
    }

    #[test]
    fn failed_replica_put_leaves_device_folder() {
        let mut drive = FakeDrive::new();
        let device = DeviceId::generate();
        let device_prefix = device_prefix(&device).unwrap();
        let replica = replica_prefix();
        let device_blob = blob(b"keep-me");
        drive
            .put_if_unmodified(&device_prefix, &device_blob, b"keep-me", None)
            .unwrap();
        let existing = blob(b"already");
        drive
            .put_if_unmodified(&replica, &existing, b"already", None)
            .unwrap();

        let err = drive
            .advance_replica(ReplicaAdvance {
                puts: vec![ReplicaPut {
                    prefix: replica,
                    blob_id: existing,
                    bytes: b"conflict".to_vec(),
                    expected_etag: None,
                }],
                prune_prefix: Some(device_prefix.clone()),
            })
            .unwrap_err();
        assert_eq!(err, SyncError::Precondition);
        assert_eq!(drive.get(&device_blob).unwrap(), b"keep-me");
        assert_eq!(drive.list(&device_prefix).unwrap().len(), 1);
    }
}
