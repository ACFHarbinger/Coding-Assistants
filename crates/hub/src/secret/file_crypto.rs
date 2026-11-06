//! Cryptographic operations for the encrypted-file secret vault (#289).
//!
//! # Architecture
//!
//! * **Cipher**: ChaCha20-Poly1305 AEAD (`ring::aead::CHACHA20_POLY1305`).
//! * **Key Derivation**: PBKDF2-HMAC-SHA256 (`ring::pbkdf2::PBKDF2_HMAC_SHA256`)
//!   with 100,000 iterations and a 16-byte random salt per file rewrite.
//! * **Entropy / Nonce**: 96-bit (12-byte) cryptographically secure random nonce
//!   from `ring::rand::SystemRandom` generated on every write.
//! * **Authenticated Header (AAD)**: The entire 37-byte file header (magic,
//!   version, iterations, salt, nonce) is authenticated as AEAD Additional
//!   Data. Any tampering with salt, iterations, nonce, or version fails closed.
//!
//! # File Format (Version 1)
//!
//! ```text
//! +---------------+--------------+-------------------+-----------------+------------------+----------------------+---------------+
//! | Magic (4B)    | Version (1B) | Iterations (4B)   | Salt (16B)      | Nonce (12B)      | Ciphertext (var)     | Poly Tag (16B)|
//! | "CAVT"        | 0x01         | u32 Big-Endian    | random bytes    | random bytes     | JSON map             | AEAD MAC tag  |
//! +---------------+--------------+-------------------+-----------------+------------------+----------------------+---------------+
//! <--------------------------- AAD (37 Bytes) -------------------------->
//! ```

use std::num::NonZeroU32;

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, CHACHA20_POLY1305};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};

use super::SecretError;

pub const VAULT_MAGIC: &[u8; 4] = b"CAVT";
pub const VAULT_VERSION: u8 = 1;
pub const DEFAULT_PBKDF2_ITERATIONS: u32 = 100_000;
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;
pub const HEADER_LEN: usize = 4 + 1 + 4 + SALT_LEN + NONCE_LEN; // 37 bytes

/// Derive a 256-bit symmetric encryption key from OS-user-scoped seed material.
pub fn derive_key(
    seed: &[u8],
    user_info: &str,
    salt: &[u8],
    iterations: u32,
) -> Result<[u8; 32], SecretError> {
    let non_zero_iter = NonZeroU32::new(iterations)
        .ok_or_else(|| SecretError::Backend("KDF iterations cannot be zero".into()))?;

    // Combine seed with user context (e.g. username/UID) for domain separation
    let mut ikm = Vec::with_capacity(seed.len() + user_info.len() + 1);
    ikm.extend_from_slice(seed);
    ikm.push(b':');
    ikm.extend_from_slice(user_info.as_bytes());

    let mut key = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        non_zero_iter,
        salt,
        &ikm,
        &mut key,
    );
    Ok(key)
}

/// Encrypt `plaintext` using ChaCha20-Poly1305 with a fresh salt and nonce.
/// Returns the full file bytes (header + ciphertext + tag).
pub fn encrypt_payload(
    seed: &[u8],
    user_info: &str,
    plaintext: &[u8],
) -> Result<Vec<u8>, SecretError> {
    let rng = SystemRandom::new();

    let mut salt = [0u8; SALT_LEN];
    rng.fill(&mut salt)
        .map_err(|_| SecretError::Backend("failed to generate random salt".into()))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| SecretError::Backend("failed to generate random nonce".into()))?;

    let key_bytes = derive_key(seed, user_info, &salt, DEFAULT_PBKDF2_ITERATIONS)?;

    let unbound_key = UnboundKey::new(&CHACHA20_POLY1305, &key_bytes)
        .map_err(|_| SecretError::Backend("failed to initialize cipher key".into()))?;
    let key = LessSafeKey::new(unbound_key);

    let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
        .map_err(|_| SecretError::Backend("failed to construct AEAD nonce".into()))?;

    // Construct 37-byte header
    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(VAULT_MAGIC);
    header.push(VAULT_VERSION);
    header.extend_from_slice(&DEFAULT_PBKDF2_ITERATIONS.to_be_bytes());
    header.extend_from_slice(&salt);
    header.extend_from_slice(&nonce_bytes);

    // In-place encryption: payload buffer initially holds plaintext,
    // seal_in_place_append_tag encrypts it and appends 16-byte Poly1305 tag.
    let mut payload = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(&header), &mut payload)
        .map_err(|_| SecretError::Backend("failed to seal vault payload".into()))?;

    let mut file_bytes = Vec::with_capacity(HEADER_LEN + payload.len());
    file_bytes.extend_from_slice(&header);
    file_bytes.extend_from_slice(&payload);
    Ok(file_bytes)
}

/// Decrypt `data` and verify integrity using ChaCha20-Poly1305.
/// Fails closed on any truncation, tampering, or wrong key.
pub fn decrypt_payload(seed: &[u8], user_info: &str, data: &[u8]) -> Result<Vec<u8>, SecretError> {
    if data.len() < HEADER_LEN + TAG_LEN {
        return Err(SecretError::Backend(
            "vault file is truncated or too small".into(),
        ));
    }

    let header = &data[..HEADER_LEN];
    if &header[..4] != VAULT_MAGIC {
        return Err(SecretError::Backend(
            "invalid vault file magic bytes".into(),
        ));
    }

    let version = header[4];
    if version != VAULT_VERSION {
        return Err(SecretError::Backend(format!(
            "unsupported vault file version: {version}"
        )));
    }

    let iterations = u32::from_be_bytes(
        header[5..9]
            .try_into()
            .map_err(|_| SecretError::Backend("malformed iteration count in header".into()))?,
    );
    let salt = &header[9..9 + SALT_LEN];
    let nonce_bytes: [u8; NONCE_LEN] = header[9 + SALT_LEN..HEADER_LEN]
        .try_into()
        .map_err(|_| SecretError::Backend("malformed nonce in header".into()))?;

    let key_bytes = derive_key(seed, user_info, salt, iterations)?;

    let unbound_key = UnboundKey::new(&CHACHA20_POLY1305, &key_bytes)
        .map_err(|_| SecretError::Backend("failed to initialize cipher key".into()))?;
    let key = LessSafeKey::new(unbound_key);

    let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
        .map_err(|_| SecretError::Backend("failed to construct AEAD nonce".into()))?;

    let mut payload = data[HEADER_LEN..].to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::from(header), &mut payload)
        .map_err(|_| {
            SecretError::Backend(
                "vault decryption or integrity check failed (tampered or wrong key)".into(),
            )
        })?;

    Ok(plaintext.to_vec())
}
