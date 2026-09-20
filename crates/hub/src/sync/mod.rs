//! Cloud Drive synchronization domain (S1–S2 / #91/#92).
//!
//! Provider-neutral contracts, local `cloud-sync.key`, and CAS1 objects.
//! The Google Drive adapter is S3. This module never uploads plaintext or
//! credentials.

mod client;
mod crypto;
mod key;
mod layout;
mod pack;
pub mod policy;
mod types;

pub use client::{DriveClient, FakeDrive, RemoteObject, ReplicaAdvance, ReplicaPut};
pub use crypto::{decrypt_object, encrypt_object};
pub use key::CloudSyncKey;
pub use layout::{device_prefix, manifests_prefix, replica_prefix};
pub use pack::seal_and_put;
pub use types::{
    BlobId, Category, CategoryPolicy, DeviceId, DeviceIdentity, ETag, Manifest, ManifestEntry,
    ObjectKind, ProviderKind, RemotePrefix, Snapshot, SyncConfig, SyncResult, TrustedAccount,
};

/// Sync failures. Messages must never include tokens, keys, or ciphertext.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SyncError {
    #[error("precondition failed")]
    Precondition,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Invalid(String),
}
