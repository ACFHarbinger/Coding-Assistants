//! Cloud Drive synchronization domain (S1–S4 / #91–#94).
//!
//! Provider-neutral contracts, local `cloud-sync.key`, CAS1 objects, a
//! Google Drive `drive.appdata` adapter, owner-started CLI/UI, and a
//! mutation-only Hub lock. This module never uploads plaintext or
//! credentials. Live Drive acceptance is S5.

mod client;
mod crypto;
mod google;
mod google_auth;
mod google_http;
mod key;
mod layout;
mod lock;
mod pack;
mod plan;
pub mod policy;
mod types;

pub use client::{DriveClient, FakeDrive, RemoteObject, ReplicaAdvance, ReplicaPut};
pub use crypto::{decrypt_object, encrypt_object};
pub use google::GoogleDrive;
pub use google_auth::{
    parse_access_token, resolve_refresh_token, token_request_form, DRIVE_APPDATA_SCOPE,
    REFRESH_TOKEN_KEY, TOKEN_URL,
};
pub use google_http::{HttpResponse, Transport, UreqTransport};
pub use key::CloudSyncKey;
pub use layout::{device_prefix, manifests_prefix, replica_prefix};
pub use lock::{
    acquire_persisted, is_held, read as read_lock, release, LockFile, SyncLockGuard, LOCKED_MESSAGE,
};
pub use pack::seal_and_put;
pub use plan::{build_plan, run_locked, start_persisted, SyncPlan, SyncSession};
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
