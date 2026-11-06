use super::file_backend::FileBackend;
use super::file_crypto::{HEADER_LEN, TAG_LEN};
use super::{SecretBackend, SecretError, SecretSource};
use tempfile::tempdir;

#[test]
fn file_backend_round_trip_set_get_delete() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("secrets.vault");
    let seed_path = dir.path().join(".vault_seed");

    let backend = FileBackend::with_paths(vault_path.clone(), seed_path);
    assert_eq!(backend.kind(), SecretSource::File);
    assert!(backend.available());

    // Initially empty
    assert!(backend.get("TEST_KEY").unwrap().is_none());
    assert_eq!(backend.status("TEST_KEY").unwrap(), (false, None));

    // Set
    backend.set("TEST_KEY", "super-secret-123").unwrap();

    // Verify status and get
    let (is_set, updated_at) = backend.status("TEST_KEY").unwrap();
    assert!(is_set);
    assert!(updated_at.is_some());

    let secret = backend.get("TEST_KEY").unwrap().unwrap();
    assert_eq!(secret.expose(), "super-secret-123");

    // Second key
    backend.set("ANOTHER_KEY", "another-secret").unwrap();
    assert_eq!(
        backend.get("ANOTHER_KEY").unwrap().unwrap().expose(),
        "another-secret"
    );
    assert_eq!(
        backend.get("TEST_KEY").unwrap().unwrap().expose(),
        "super-secret-123"
    );

    // Delete first key
    backend.delete("TEST_KEY").unwrap();
    assert!(backend.get("TEST_KEY").unwrap().is_none());
    assert_eq!(backend.status("TEST_KEY").unwrap(), (false, None));

    // Second key still intact
    assert_eq!(
        backend.get("ANOTHER_KEY").unwrap().unwrap().expose(),
        "another-secret"
    );

    // Delete is idempotent
    backend.delete("TEST_KEY").unwrap();
    backend.delete("NON_EXISTENT").unwrap();
}

#[test]
fn file_backend_fails_closed_on_truncated_file() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("secrets.vault");
    let seed_path = dir.path().join(".vault_seed");

    let backend = FileBackend::with_paths(vault_path.clone(), seed_path);
    backend.set("K", "secret-data").unwrap();

    let bytes = std::fs::read(&vault_path).unwrap();
    assert!(bytes.len() > HEADER_LEN + TAG_LEN);

    // Truncate to less than header
    std::fs::write(&vault_path, &bytes[..10]).unwrap();
    let err = backend.get("K").unwrap_err();
    assert!(matches!(err, SecretError::Backend(_)));

    // Truncate inside ciphertext
    std::fs::write(&vault_path, &bytes[..HEADER_LEN + 2]).unwrap();
    let err = backend.get("K").unwrap_err();
    assert!(matches!(err, SecretError::Backend(_)));
}

#[test]
fn file_backend_fails_closed_on_tampered_ciphertext() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("secrets.vault");
    let seed_path = dir.path().join(".vault_seed");

    let backend = FileBackend::with_paths(vault_path.clone(), seed_path);
    backend.set("KEY1", "important-secret").unwrap();

    let mut bytes = std::fs::read(&vault_path).unwrap();
    // Tamper with one byte in the ciphertext/tag payload
    let last = bytes.len() - 1;
    bytes[last] ^= 0x55;
    std::fs::write(&vault_path, &bytes).unwrap();

    let err = backend.get("KEY1").unwrap_err();
    assert!(matches!(err, SecretError::Backend(_)));
    assert_eq!(
        backend.status("KEY1").unwrap_err().to_string(),
        err.to_string()
    );
}

#[test]
fn file_backend_fails_closed_on_tampered_header() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("secrets.vault");
    let seed_path = dir.path().join(".vault_seed");

    let backend = FileBackend::with_paths(vault_path.clone(), seed_path);
    backend.set("KEY1", "important-secret").unwrap();

    let mut bytes = std::fs::read(&vault_path).unwrap();
    // Tamper with salt byte (inside header/AAD)
    bytes[10] ^= 0xff;
    std::fs::write(&vault_path, &bytes).unwrap();

    let err = backend.get("KEY1").unwrap_err();
    assert!(matches!(err, SecretError::Backend(_)));
}

#[test]
fn file_backend_fails_closed_on_wrong_seed() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("secrets.vault");
    let seed_path1 = dir.path().join(".vault_seed1");
    let seed_path2 = dir.path().join(".vault_seed2");

    let backend1 = FileBackend::with_paths(vault_path.clone(), seed_path1);
    backend1.set("SEC", "correct-value").unwrap();

    // Create a second backend pointing to the same vault file but a different seed
    let backend2 = FileBackend::with_paths(vault_path, seed_path2);
    let err = backend2.get("SEC").unwrap_err();
    assert!(matches!(err, SecretError::Backend(_)));
}

#[test]
fn file_backend_atomic_write_preserves_old_file_on_failure() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("secrets.vault");
    let seed_path = dir.path().join(".vault_seed");

    let backend = FileBackend::with_paths(vault_path.clone(), seed_path);
    backend.set("EXISTING", "precious-secret").unwrap();

    let original_bytes = std::fs::read(&vault_path).unwrap();

    // Verify key validation fails early before any write or temp file
    let err = backend.set("invalid key with spaces", "new-secret");
    assert!(matches!(err, Err(SecretError::InvalidKey(_))));

    // Original file is untouched
    let after_bytes = std::fs::read(&vault_path).unwrap();
    assert_eq!(original_bytes, after_bytes);
    assert_eq!(
        backend.get("EXISTING").unwrap().unwrap().expose(),
        "precious-secret"
    );
}

#[cfg(unix)]
#[test]
fn file_backend_sets_0600_permissions_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("secrets.vault");
    let seed_path = dir.path().join(".vault_seed");

    let backend = FileBackend::with_paths(vault_path.clone(), seed_path.clone());
    backend.set("TEST", "val").unwrap();

    let vault_mode = std::fs::metadata(&vault_path).unwrap().permissions().mode() & 0o777;
    let seed_mode = std::fs::metadata(&seed_path).unwrap().permissions().mode() & 0o777;

    assert_eq!(vault_mode, 0o600, "vault file should have 0600 mode");
    assert_eq!(seed_mode, 0o600, "seed file should have 0600 mode");
}
