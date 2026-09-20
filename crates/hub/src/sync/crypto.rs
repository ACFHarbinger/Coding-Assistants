//! Versioned authenticated sync objects (S2 / #92).
//!
//! Portable ChaCha20-Poly1305 using `cloud-sync.key` directly. Not the vault
//! `CAVT` format (that PBKDF2 is OS-user-scoped and cannot move devices).

use super::key::CloudSyncKey;
use super::types::ObjectKind;
use super::SyncError;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, CHACHA20_POLY1305};
use ring::rand::{SecureRandom, SystemRandom};

pub const MAGIC: &[u8; 4] = b"CAS1";
pub const VERSION: u8 = 1;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const HEADER_LEN: usize = 4 + 1 + 1 + NONCE_LEN; // magic, version, kind, nonce
const AAD_LEN: usize = 6; // magic || version || kind

/// Seal `plaintext` as a CAS1 object. Fresh nonce every call.
pub fn encrypt_object(
    key: &CloudSyncKey,
    kind: ObjectKind,
    plaintext: &[u8],
) -> Result<Vec<u8>, SyncError> {
    let mut nonce_bytes = [0u8; NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce_bytes)
        .map_err(|_| SyncError::Invalid("failed to generate object nonce".into()))?;

    let aead = aead_key(key)?;
    let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
        .map_err(|_| SyncError::Invalid("failed to construct object nonce".into()))?;

    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.push(VERSION);
    header.push(kind.as_u8());
    header.extend_from_slice(&nonce_bytes);

    let mut payload = plaintext.to_vec();
    aead.seal_in_place_append_tag(nonce, Aad::from(&header[..AAD_LEN]), &mut payload)
        .map_err(|_| SyncError::Invalid("failed to seal sync object".into()))?;

    let mut out = header;
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Open a CAS1 object. Fails closed on truncation, wrong key, or tamper.
pub fn decrypt_object(key: &CloudSyncKey, data: &[u8]) -> Result<(ObjectKind, Vec<u8>), SyncError> {
    if data.len() < HEADER_LEN + TAG_LEN {
        return Err(SyncError::Invalid("sync object is truncated".into()));
    }
    if &data[..4] != MAGIC {
        return Err(SyncError::Invalid("sync object magic mismatch".into()));
    }
    if data[4] != VERSION {
        return Err(SyncError::Invalid("unsupported sync object version".into()));
    }
    let kind = ObjectKind::from_u8(data[5])?;
    let nonce_bytes: [u8; NONCE_LEN] = data[6..HEADER_LEN]
        .try_into()
        .map_err(|_| SyncError::Invalid("malformed object nonce".into()))?;

    let aead = aead_key(key)?;
    let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
        .map_err(|_| SyncError::Invalid("failed to construct object nonce".into()))?;

    let mut payload = data[HEADER_LEN..].to_vec();
    let plaintext = aead
        .open_in_place(nonce, Aad::from(&data[..AAD_LEN]), &mut payload)
        .map_err(|_| SyncError::Invalid("sync object decryption failed".into()))?;
    Ok((kind, plaintext.to_vec()))
}

fn aead_key(key: &CloudSyncKey) -> Result<LessSafeKey, SyncError> {
    let unbound = UnboundKey::new(&CHACHA20_POLY1305, key.expose())
        .map_err(|_| SyncError::Invalid("failed to initialize sync cipher".into()))?;
    Ok(LessSafeKey::new(unbound))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::key::CloudSyncKey;
    use tempfile::tempdir;

    fn test_key() -> CloudSyncKey {
        CloudSyncKey::import(tempdir().unwrap().path(), &[0x11; 32]).unwrap()
    }

    #[test]
    fn roundtrip_and_fresh_nonce() {
        let key = test_key();
        let a = encrypt_object(&key, ObjectKind::Blob, b"hello").unwrap();
        let b = encrypt_object(&key, ObjectKind::Blob, b"hello").unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with(MAGIC));
        let (kind, plain) = decrypt_object(&key, &a).unwrap();
        assert_eq!(kind, ObjectKind::Blob);
        assert_eq!(plain, b"hello");
    }

    #[test]
    fn journal_fernet_blocks_are_opaque_bytes() {
        let key = test_key();
        let journal = b"# claude\n<!--ENC-->gAAAAABjournalfernet\nmore\n";
        let sealed = encrypt_object(&key, ObjectKind::Blob, journal).unwrap();
        let (kind, plain) = decrypt_object(&key, &sealed).unwrap();
        assert_eq!(kind, ObjectKind::Blob);
        assert_eq!(plain, journal);
        assert!(!sealed
            .windows(b"<!--ENC-->".len())
            .any(|w| w == b"<!--ENC-->"));
    }

    #[test]
    fn wrong_key_or_tamper_fails_closed() {
        let dir = tempdir().unwrap();
        let key = CloudSyncKey::import(dir.path(), &[0x11; 32]).unwrap();
        let other = CloudSyncKey::import(tempdir().unwrap().path(), &[0x22; 32]).unwrap();
        let mut sealed = encrypt_object(&key, ObjectKind::Manifest, b"{}").unwrap();
        assert!(decrypt_object(&other, &sealed).is_err());
        let last = sealed.len() - 1;
        sealed[last] ^= 0xff;
        assert!(decrypt_object(&key, &sealed).is_err());
    }
}
