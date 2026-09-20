//! Cloud Drive synchronization domain (S1 / #91).
//!
//! Provider-neutral contracts and an in-memory FakeDrive. Encryption is S2;
//! the Google Drive adapter is S3. This module never uploads plaintext or
//! credentials.

mod client;
mod layout;
pub mod policy;
mod types;

pub use client::{DriveClient, FakeDrive, RemoteObject, ReplicaAdvance, ReplicaPut};
pub use layout::{device_prefix, manifests_prefix, replica_prefix};
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
