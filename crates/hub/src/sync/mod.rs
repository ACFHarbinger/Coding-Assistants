//! Cloud Drive synchronization domain (S1–S10 / #91–#100).
//!
//! Provider-neutral contracts, local `cloud-sync.key`, CAS1 objects, a
//! Google Drive `drive.appdata` adapter, optional Firebase Auth identity
//! plus private Storage, owner-started CLI/UI, a mutation-only Hub lock,
//! encrypted snapshot transfer, three-way reconcile with fork-aware
//! audit rebase, owner conflict review, confirm-only tombstones, device
//! trust, resumable owner retry, and redacted diagnostics. Never uploads
//! plaintext or credentials. Live `hub.db` is never replaced. Revoke
//! does not rotate the key. Reserved recovery envelope stays unused.

mod client;
mod crypto;
mod firebase;
mod firebase_auth;
mod fs_drive;
mod google;
mod google_auth;
mod google_http;
mod history;
mod journal;
mod key;
mod layout;
mod lock;
mod merge;
mod ops;
mod pack;
mod plan;
pub mod policy;
mod rebase;
mod review;
mod snapshot;
mod tombstone;
mod trust;
mod types;

pub use client::{DriveClient, FakeDrive, RemoteObject, ReplicaAdvance, ReplicaPut};
pub use crypto::{decrypt_object, encrypt_object};
pub use firebase::FirebaseStorage;
pub use firebase_auth::{
    firebase_account, parse_id_token, resolve_api_key, resolve_bucket,
    resolve_refresh_token as resolve_firebase_refresh_token,
    token_request_form as firebase_token_request_form, API_KEY as FIREBASE_API_KEY,
    REFRESH_TOKEN_KEY as FIREBASE_REFRESH_TOKEN_KEY, SECURE_TOKEN_URL,
    STORAGE_BUCKET_KEY as FIREBASE_STORAGE_BUCKET,
};
pub use fs_drive::FsDrive;
pub use google::GoogleDrive;
pub use google_auth::{
    parse_access_token, resolve_refresh_token, token_request_form, DRIVE_APPDATA_SCOPE,
    REFRESH_TOKEN_KEY, TOKEN_URL,
};
pub use google_http::{HttpResponse, Transport, UreqTransport};
pub use history::{
    append_history, export_diagnostics, list_history, Diagnostics, HistoryEntry, TrustExport,
    HISTORY_LIMIT,
};
pub use key::CloudSyncKey;
pub use layout::{device_prefix, manifests_prefix, replica_prefix};
pub use lock::{
    acquire_persisted, is_held, read as read_lock, release, LockFile, SyncLockGuard, LOCKED_MESSAGE,
};
pub use ops::{
    begin_resume, clear_resume, ensure_limits, load_resume, pending_retry, write_limits,
    ResumeDone, ResumeState, SyncLimits, DEFAULT_MAX_BYTES, DEFAULT_MAX_CONCURRENT,
    DEFAULT_MAX_OBJECTS, QUOTA_WARNING,
};
pub use pack::seal_and_put;
pub use plan::{build_plan, run_locked, start_persisted, SyncPlan, SyncSession};
pub use review::{apply_choice, list_conflicts, ConflictChoice, ConflictDecision, ConflictItem};
pub use tombstone::{
    confirm_delete, expired_cleanup_candidates, list_tombstones, purge_expired, CleanupCandidate,
    PurgeReport, Tombstone, DEFAULT_RETENTION_DAYS,
};
pub use trust::{
    ensure_local_device, list_trust, may_accept, register, retrust, revoke, TrustEntry, TrustList,
    TrustStatus,
};
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
