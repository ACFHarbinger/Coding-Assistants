//! File-backed `DriveClient` for local two-profile rehearsal (S5 / #95).
//!
//! Remote names are BlobIds only. Not a live Google account.

use super::client::{DriveClient, RemoteObject, ReplicaAdvance};
use super::types::{sha256_hex, BlobId, ETag, RemotePrefix};
use super::SyncError;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FsDrive {
    root: PathBuf,
}

impl FsDrive {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, SyncError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(io_err)?;
        Ok(Self { root })
    }

    /// Shared rehearsal root: `CA_SYNC_FAKE_ROOT` or `{home}/sync/remote`.
    pub fn open_local(home: &Path) -> Result<Self, SyncError> {
        let root = std::env::var_os("CA_SYNC_FAKE_ROOT")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("sync").join("remote"));
        Self::open(root)
    }

    fn dir_for(&self, prefix: &RemotePrefix) -> PathBuf {
        self.root.join(prefix.as_str().trim_end_matches('/'))
    }

    fn blob_path(&self, prefix: &RemotePrefix, id: &BlobId) -> PathBuf {
        self.dir_for(prefix).join(id.as_str())
    }

    fn find(&self, id: &BlobId) -> Result<(PathBuf, ETag), SyncError> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Err(SyncError::NotFound);
        };
        for entry in entries.flatten() {
            let path = entry.path().join(id.as_str());
            if path.is_file() {
                let etag = read_etag(&path)?;
                return Ok((path, etag));
            }
        }
        Err(SyncError::NotFound)
    }
}

impl DriveClient for FsDrive {
    fn list(&self, prefix: &RemotePrefix) -> Result<Vec<RemoteObject>, SyncError> {
        let dir = self.dir_for(prefix);
        let Ok(entries) = fs::read_dir(&dir) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.ends_with(".etag") {
                continue;
            }
            let Ok(blob_id) = BlobId::parse(name) else {
                continue;
            };
            let meta = fs::metadata(&path).map_err(io_err)?;
            out.push(RemoteObject {
                blob_id,
                etag: read_etag(&path)?,
                size: meta.len(),
            });
        }
        Ok(out)
    }

    fn get(&self, id: &BlobId) -> Result<Vec<u8>, SyncError> {
        let (path, _) = self.find(id)?;
        fs::read(path).map_err(io_err)
    }

    fn put_if_unmodified(
        &mut self,
        prefix: &RemotePrefix,
        id: &BlobId,
        bytes: &[u8],
        expected: Option<&ETag>,
    ) -> Result<ETag, SyncError> {
        let path = self.blob_path(prefix, id);
        match (path.is_file(), expected) {
            (true, Some(etag)) if read_etag(&path)? == *etag => {}
            (false, None) => {}
            _ => return Err(SyncError::Precondition),
        }
        fs::create_dir_all(self.dir_for(prefix)).map_err(io_err)?;
        fs::write(&path, bytes).map_err(io_err)?;
        let etag = ETag::new(sha256_hex(bytes));
        fs::write(etag_path(&path), etag.as_str().as_bytes()).map_err(io_err)?;
        Ok(etag)
    }

    fn delete_if_match(&mut self, id: &BlobId, etag: &ETag) -> Result<(), SyncError> {
        let (path, stored) = self.find(id)?;
        if stored != *etag {
            return Err(SyncError::Precondition);
        }
        let _ = fs::remove_file(etag_path(&path));
        fs::remove_file(path).map_err(io_err)
    }

    fn advance_replica(&mut self, plan: ReplicaAdvance) -> Result<(), SyncError> {
        for put in &plan.puts {
            self.put_if_unmodified(
                &put.prefix,
                &put.blob_id,
                &put.bytes,
                put.expected_etag.as_ref(),
            )?;
        }
        if let Some(prefix) = plan.prune_prefix {
            for object in self.list(&prefix)? {
                self.delete_if_match(&object.blob_id, &object.etag)?;
            }
        }
        Ok(())
    }
}

fn etag_path(blob: &Path) -> PathBuf {
    let mut path = blob.as_os_str().to_os_string();
    path.push(".etag");
    PathBuf::from(path)
}

fn read_etag(blob: &Path) -> Result<ETag, SyncError> {
    let raw = fs::read_to_string(etag_path(blob)).map_err(|_| SyncError::NotFound)?;
    Ok(ETag::new(raw.trim().to_string()))
}

fn io_err(error: std::io::Error) -> SyncError {
    SyncError::Invalid(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::layout::replica_prefix;
    use tempfile::tempdir;

    #[test]
    fn names_are_hashed_blob_ids() {
        let dir = tempdir().unwrap();
        let mut drive = FsDrive::open(dir.path()).unwrap();
        let prefix = replica_prefix();
        let id = BlobId::from_encrypted_bytes(b"enc");
        drive.put_if_unmodified(&prefix, &id, b"enc", None).unwrap();
        let listed = drive.list(&prefix).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].blob_id.as_str().len(), 64);
        assert!(!listed[0].blob_id.as_str().contains('/'));
        assert_eq!(drive.get(&id).unwrap(), b"enc");
    }
}
