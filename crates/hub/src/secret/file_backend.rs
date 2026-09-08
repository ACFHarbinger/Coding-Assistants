//! Encrypted-file [`SecretBackend`] implementation (#289).
//!
//! Stores credentials encrypted with ChaCha20-Poly1305 and PBKDF2 in
//! `<CA_HOME or ~/.coding-assistants>/secrets.vault`, mode `0600`.
//! Selected automatically when no OS secret service answers, or forced
//! with `CA_SECRET_BACKEND=file`.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use super::file_crypto::{decrypt_payload, encrypt_payload};
use super::{now_unix, validate_key, SecretBackend, SecretError, SecretSource, SecretString};

/// All FileBackend instances in this process address the same default vault.
/// A per-instance mutex cannot protect the read-modify-write cycle when the
/// resolver constructs a fresh backend for each call.
static VAULT_IO_LOCK: Mutex<()> = Mutex::new(());

/// The random, owner-only seed file supplies the user-specific entropy. Keep
/// the PBKDF2 context stable rather than deriving it from mutable `USER` /
/// `USERNAME` environment variables, which would otherwise make an existing
/// vault unreadable after a launcher or service changes those variables.
const FILE_VAULT_DOMAIN: &str = "coding-assistants:file-vault:v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FileVaultEntry {
    pub(crate) secret: String,
    #[serde(default)]
    pub(crate) updated_at: Option<i64>,
}

pub struct FileBackend {
    vault_path: PathBuf,
    seed_path: PathBuf,
    lock: Mutex<()>,
}

impl FileBackend {
    pub fn new() -> Self {
        let base = crate::paths::default_hub_home();
        Self::with_paths(base.join("secrets.vault"), base.join(".vault_seed"))
    }

    pub fn with_paths(vault_path: PathBuf, seed_path: PathBuf) -> Self {
        Self {
            vault_path,
            seed_path,
            lock: Mutex::new(()),
        }
    }

    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    fn os_user_identity() -> &'static str {
        FILE_VAULT_DOMAIN
    }

    fn load_or_create_seed(&self) -> Result<Vec<u8>, SecretError> {
        if self.seed_path.exists() {
            let bytes = std::fs::read(&self.seed_path)
                .map_err(|e| SecretError::Backend(format!("failed to read vault seed: {e}")))?;
            if bytes.len() >= 16 {
                return Ok(bytes);
            }
        }

        if let Some(parent) = self.seed_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SecretError::Backend(format!("failed to create vault dir: {e}")))?;
        }

        let rng = SystemRandom::new();
        let mut seed = [0u8; 32];
        rng.fill(&mut seed)
            .map_err(|_| SecretError::Backend("failed to generate random vault seed".into()))?;

        let tmp_path = self
            .seed_path
            .with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)
                .map_err(|e| {
                    SecretError::Backend(format!("failed to create seed temp file: {e}"))
                })?;
            apply_file_mode_0600(&file).map_err(|e| {
                SecretError::Backend(format!("failed to set seed permissions: {e}"))
            })?;
            file.write_all(&seed)
                .map_err(|e| SecretError::Backend(format!("failed to write seed: {e}")))?;
            file.sync_all()
                .map_err(|e| SecretError::Backend(format!("failed to sync seed: {e}")))?;
        }

        std::fs::rename(&tmp_path, &self.seed_path).map_err(|e| {
            SecretError::Backend(format!("failed to atomically save vault seed: {e}"))
        })?;
        let _ = set_path_mode_0600(&self.seed_path);

        Ok(seed.to_vec())
    }

    fn read_entries(&self) -> Result<HashMap<String, FileVaultEntry>, SecretError> {
        if !self.vault_path.exists() {
            return Ok(HashMap::new());
        }

        let bytes = match std::fs::read(&self.vault_path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(SecretError::Unavailable(e.to_string()));
            }
            Err(e) => return Err(SecretError::Backend(format!("failed to read vault: {e}"))),
        };

        let seed = self.load_or_create_seed()?;
        let user = Self::os_user_identity();
        let plaintext = decrypt_payload(&seed, user, &bytes)?;

        serde_json::from_slice(&plaintext)
            .map_err(|e| SecretError::Backend(format!("failed to deserialize vault entries: {e}")))
    }

    fn write_entries(&self, entries: &HashMap<String, FileVaultEntry>) -> Result<(), SecretError> {
        let seed = self.load_or_create_seed()?;
        let user = Self::os_user_identity();
        let plaintext = serde_json::to_vec(entries)
            .map_err(|e| SecretError::Backend(format!("failed to serialize vault entries: {e}")))?;

        let encrypted = encrypt_payload(&seed, user, &plaintext)?;

        if let Some(parent) = self.vault_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SecretError::Backend(format!("failed to create vault dir: {e}")))?;
        }

        let tmp_path = self
            .vault_path
            .with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)
                .map_err(|e| {
                    SecretError::Backend(format!("failed to create vault temp file: {e}"))
                })?;
            apply_file_mode_0600(&file).map_err(|e| {
                SecretError::Backend(format!("failed to set vault temp file permissions: {e}"))
            })?;
            file.write_all(&encrypted)
                .map_err(|e| SecretError::Backend(format!("failed to write vault temp: {e}")))?;
            file.sync_all()
                .map_err(|e| SecretError::Backend(format!("failed to sync vault temp: {e}")))?;
        }

        std::fs::rename(&tmp_path, &self.vault_path)
            .map_err(|e| SecretError::Backend(format!("failed to atomically save vault: {e}")))?;
        let _ = set_path_mode_0600(&self.vault_path);

        Ok(())
    }
}

impl Default for FileBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(unix)]
fn apply_file_mode_0600(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn apply_file_mode_0600(_file: &std::fs::File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_path_mode_0600(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_path_mode_0600(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

impl SecretBackend for FileBackend {
    fn kind(&self) -> SecretSource {
        SecretSource::File
    }

    fn available(&self) -> bool {
        true
    }

    fn set(&self, key: &str, secret: &str) -> Result<(), SecretError> {
        validate_key(key)?;
        let _process_guard = VAULT_IO_LOCK.lock().unwrap();
        let _guard = self.lock.lock().unwrap();
        let mut entries = self.read_entries()?;
        entries.insert(
            key.to_string(),
            FileVaultEntry {
                secret: secret.to_string(),
                updated_at: Some(now_unix()),
            },
        );
        self.write_entries(&entries)
    }

    fn get(&self, key: &str) -> Result<Option<SecretString>, SecretError> {
        validate_key(key)?;
        let _process_guard = VAULT_IO_LOCK.lock().unwrap();
        let _guard = self.lock.lock().unwrap();
        let entries = self.read_entries()?;
        Ok(entries
            .get(key)
            .map(|entry| SecretString::new(entry.secret.clone())))
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        validate_key(key)?;
        let _process_guard = VAULT_IO_LOCK.lock().unwrap();
        let _guard = self.lock.lock().unwrap();
        if !self.vault_path.exists() {
            return Ok(());
        }
        let mut entries = self.read_entries()?;
        if entries.remove(key).is_some() {
            self.write_entries(&entries)?;
        }
        Ok(())
    }

    fn status(&self, key: &str) -> Result<(bool, Option<i64>), SecretError> {
        validate_key(key)?;
        let _process_guard = VAULT_IO_LOCK.lock().unwrap();
        let _guard = self.lock.lock().unwrap();
        if !self.vault_path.exists() {
            return Ok((false, None));
        }
        match self.read_entries() {
            Ok(entries) => match entries.get(key) {
                Some(entry) => Ok((true, entry.updated_at)),
                None => Ok((false, None)),
            },
            Err(SecretError::Unavailable(_)) => Ok((false, None)),
            Err(e) => Err(e),
        }
    }
}
