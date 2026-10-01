//! Cloud-sync domain types (S1 / #91).
//!
//! Remote names are hashed only. Nothing here encrypts, talks to Drive, or
//! carries credentials.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// SHA-256 hex of encrypted object bytes. The only remote file name we use.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlobId(String);

impl BlobId {
    pub fn from_encrypted_bytes(bytes: &[u8]) -> Self {
        Self(sha256_hex(bytes))
    }

    pub fn parse(value: &str) -> Result<Self, super::SyncError> {
        if value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
            Ok(Self(value.to_ascii_lowercase()))
        } else {
            Err(super::SyncError::Invalid(
                "blob id must be 64 hex chars".into(),
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Local random device identity. Remote folder names hash this; never a hostname.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn parse(value: &str) -> Result<Self, super::SyncError> {
        let trimmed = value.trim();
        if Uuid::parse_str(trimmed).is_err() {
            return Err(super::SyncError::Invalid("device id must be a UUID".into()));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// 32-char hex prefix of SHA-256(uuid). Safe as a Drive folder name.
    pub fn remote_folder_name(&self) -> String {
        sha256_hex(self.0.as_bytes()).chars().take(32).collect()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Provider-opaque conditional-write token.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ETag(String);

impl ETag {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// App-data path prefix (`devices/<hash>/`, `replica/`, `manifests/`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RemotePrefix(String);

impl RemotePrefix {
    pub fn new(value: impl Into<String>) -> Result<Self, super::SyncError> {
        let value = value.into();
        if value.is_empty() || value.contains("..") || value.contains('\\') || !value.ends_with('/')
        {
            return Err(super::SyncError::Invalid(
                "remote prefix must be a slash-terminated path without ..".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    HubDatabase,
    SharedDurable,
    PrivateJournal,
    Audit,
    MarkdownExport,
    Cache,
    Wake,
    SecretConfig,
    CloudSyncKey,
    JournalKey,
    SyncLocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryPolicy {
    Include,
    DownloadOnly,
    Exclude,
    Snapshot,
    LocalOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Fake,
    GoogleDrive,
    Firebase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Blob,
    Manifest,
    Snapshot,
}

impl ObjectKind {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Blob => 0,
            Self::Manifest => 1,
            Self::Snapshot => 2,
        }
    }

    pub fn from_u8(value: u8) -> Result<Self, super::SyncError> {
        match value {
            0 => Ok(Self::Blob),
            1 => Ok(Self::Manifest),
            2 => Ok(Self::Snapshot),
            _ => Err(super::SyncError::Invalid("unknown object kind".into())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncConfig {
    pub provider: ProviderKind,
    pub account_ref: Option<String>,
    pub policies: HashMap<Category, CategoryPolicy>,
}

impl SyncConfig {
    pub fn v1_defaults() -> Self {
        let mut policies = HashMap::new();
        policies.insert(Category::HubDatabase, CategoryPolicy::Snapshot);
        policies.insert(Category::SharedDurable, CategoryPolicy::Snapshot);
        policies.insert(Category::PrivateJournal, CategoryPolicy::Include);
        policies.insert(Category::Audit, CategoryPolicy::Include);
        policies.insert(Category::MarkdownExport, CategoryPolicy::Include);
        policies.insert(Category::Cache, CategoryPolicy::Exclude);
        policies.insert(Category::Wake, CategoryPolicy::Exclude);
        policies.insert(Category::SecretConfig, CategoryPolicy::LocalOnly);
        policies.insert(Category::CloudSyncKey, CategoryPolicy::LocalOnly);
        policies.insert(Category::JournalKey, CategoryPolicy::LocalOnly);
        policies.insert(Category::SyncLocal, CategoryPolicy::LocalOnly);
        Self {
            provider: ProviderKind::Fake,
            account_ref: None,
            policies,
        }
    }

    pub fn policy_for(&self, category: Category) -> CategoryPolicy {
        self.policies
            .get(&category)
            .copied()
            .unwrap_or_else(|| super::policy::default_policy(category))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub id: DeviceId,
    pub created_at: String,
}

impl DeviceIdentity {
    pub fn generate() -> Self {
        Self {
            id: DeviceId::generate(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub blob_id: BlobId,
    pub relative_path: String,
    pub category: Category,
    pub content_hash: String,
    pub size: u64,
    pub kind: ObjectKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub device_id: DeviceId,
    pub schema_version: Option<String>,
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub manifest: Manifest,
    pub created_at: String,
}

/// Outcome of a run. Never holds tokens, keys, or plaintext paths of secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SyncResult {
    pub uploaded: usize,
    pub downloaded: usize,
    pub pruned: usize,
    pub conflicts: usize,
    pub warnings: Vec<String>,
}

/// Presence-only cloud identity. Token lives in the P12 vault, not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedAccount {
    pub provider: String,
    pub label: Option<String>,
    pub source: String,
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_id_is_64_lowercase_hex_of_ciphertext() {
        let id = BlobId::from_encrypted_bytes(b"ciphertext");
        assert_eq!(id.as_str().len(), 64);
        assert!(id.as_str().chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(id.as_str(), id.as_str().to_ascii_lowercase());
        assert_ne!(id, BlobId::from_encrypted_bytes(b"other"));
        assert!(BlobId::parse("not-hex").is_err());
    }

    #[test]
    fn device_remote_folder_is_hashed_not_the_uuid() {
        let id = DeviceId::parse("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee").unwrap();
        let folder = id.remote_folder_name();
        assert_eq!(folder.len(), 32);
        assert!(folder.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')));
        assert!(!folder.contains('-'));
        assert!(!folder.contains("aaaa"));
    }

    #[test]
    fn v1_defaults_never_upload_keys_or_staging() {
        let config = SyncConfig::v1_defaults();
        assert_eq!(
            config.policy_for(Category::CloudSyncKey),
            CategoryPolicy::LocalOnly
        );
        assert_eq!(
            config.policy_for(Category::JournalKey),
            CategoryPolicy::LocalOnly
        );
        assert_eq!(
            config.policy_for(Category::SyncLocal),
            CategoryPolicy::LocalOnly
        );
        assert_eq!(
            config.policy_for(Category::HubDatabase),
            CategoryPolicy::Snapshot
        );
    }

    #[test]
    fn sync_result_has_no_secret_fields() {
        let json = serde_json::to_string(&SyncResult::default()).unwrap();
        assert!(!json.contains("token"));
        assert!(!json.contains("key"));
        assert!(!json.contains("refresh"));
    }
}
