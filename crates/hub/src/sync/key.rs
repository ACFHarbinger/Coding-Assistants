//! Local-only `cloud-sync.key` (S2 / #92).
//!
//! 32 random bytes, never uploaded, copied between devices by the owner.
//! Distinct from journal Fernet keys and from the OS-user vault seed.

use super::SyncError;
use ring::rand::{SecureRandom, SystemRandom};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const KEY_LEN: usize = 32;
const FILE_NAME: &str = "cloud-sync.key";

/// Portable replica key. Redacted in `Debug`, zeroized on drop, not serialized.
pub struct CloudSyncKey(Zeroizing<[u8; KEY_LEN]>);

impl CloudSyncKey {
    pub fn key_path(home: &Path) -> PathBuf {
        home.join("keys").join(FILE_NAME)
    }

    /// Generate a new key. Fails if the file already exists (no silent rotate).
    pub fn create(home: &Path) -> Result<Self, SyncError> {
        let mut bytes = [0u8; KEY_LEN];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| SyncError::Invalid("failed to generate cloud-sync.key".into()))?;
        let key = Self(Zeroizing::new(bytes));
        write_key(home, key.expose(), true)?;
        Ok(key)
    }

    /// Install owner-copied key bytes (replace if present). Must be 32 bytes.
    pub fn import(home: &Path, bytes: &[u8]) -> Result<Self, SyncError> {
        let key = Self::from_bytes(bytes)?;
        write_key(home, key.expose(), false)?;
        Ok(key)
    }

    pub fn load(home: &Path) -> Result<Self, SyncError> {
        let path = Self::key_path(home);
        let data = fs::read(&path).map_err(|error| {
            SyncError::Invalid(format!("failed to read cloud-sync.key: {error}"))
        })?;
        Self::from_bytes(&data)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, SyncError> {
        let array: [u8; KEY_LEN] = bytes
            .try_into()
            .map_err(|_| SyncError::Invalid("cloud-sync.key must be 32 bytes".into()))?;
        Ok(Self(Zeroizing::new(array)))
    }

    pub fn expose(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

impl fmt::Debug for CloudSyncKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CloudSyncKey(***)")
    }
}

fn write_key(home: &Path, bytes: &[u8; KEY_LEN], create_new: bool) -> Result<(), SyncError> {
    let dir = home.join("keys");
    fs::create_dir_all(&dir)
        .map_err(|error| SyncError::Invalid(format!("failed to create keys/: {error}")))?;
    let path = CloudSyncKey::key_path(home);
    if create_new && path.exists() {
        return Err(SyncError::Invalid("cloud-sync.key already exists".into()));
    }
    let tmp = dir.join("cloud-sync.key.tmp");
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|error| {
                SyncError::Invalid(format!("failed to write cloud-sync.key: {error}"))
            })?;
        file.write_all(bytes).map_err(|error| {
            SyncError::Invalid(format!("failed to write cloud-sync.key: {error}"))
        })?;
        apply_file_mode_0600(&file).map_err(|error| {
            SyncError::Invalid(format!("failed to chmod cloud-sync.key: {error}"))
        })?;
        file.sync_all().map_err(|error| {
            SyncError::Invalid(format!("failed to sync cloud-sync.key: {error}"))
        })?;
    }
    fs::rename(&tmp, &path).map_err(|error| {
        SyncError::Invalid(format!("failed to install cloud-sync.key: {error}"))
    })?;
    let _ = set_path_mode_0600(&path);
    Ok(())
}

#[cfg(unix)]
fn apply_file_mode_0600(file: &fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn apply_file_mode_0600(_file: &fs::File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_path_mode_0600(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_path_mode_0600(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn debug_is_redacted() {
        let dir = tempdir().unwrap();
        let key = CloudSyncKey::create(dir.path()).unwrap();
        assert_eq!(format!("{key:?}"), "CloudSyncKey(***)");
        assert!(!format!("{key:#?}").contains(&format!("{:02x}", key.expose()[0])));
    }

    #[test]
    fn create_load_roundtrip_and_refuses_a_second_create() {
        let dir = tempdir().unwrap();
        let created = CloudSyncKey::create(dir.path()).unwrap();
        let loaded = CloudSyncKey::load(dir.path()).unwrap();
        assert_eq!(created.expose(), loaded.expose());
        assert!(CloudSyncKey::create(dir.path()).is_err());
    }

    #[test]
    fn import_rejects_wrong_length_and_replaces() {
        let dir = tempdir().unwrap();
        assert!(CloudSyncKey::import(dir.path(), &[1, 2, 3]).is_err());
        let first = [7u8; 32];
        CloudSyncKey::import(dir.path(), &first).unwrap();
        let second = [9u8; 32];
        CloudSyncKey::import(dir.path(), &second).unwrap();
        assert_eq!(CloudSyncKey::load(dir.path()).unwrap().expose(), &second);
    }

    #[cfg(unix)]
    #[test]
    fn key_file_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        CloudSyncKey::create(dir.path()).unwrap();
        let mode = fs::metadata(CloudSyncKey::key_path(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
