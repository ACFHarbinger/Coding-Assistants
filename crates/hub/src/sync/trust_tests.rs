use super::*;
use crate::sync::client::FakeDrive;
use crate::sync::key::CloudSyncKey;
use crate::sync::snapshot::{download_home, upload_home};
use crate::sync::types::SyncConfig;
use std::fs;
use tempfile::tempdir;

#[test]
fn revoke_then_may_accept_is_false_unknown_is_false() {
    let dir = tempdir().unwrap();
    let unknown = DeviceId::generate();
    assert!(!may_accept(dir.path(), &unknown));
    let id = DeviceId::generate();
    register(dir.path(), id.clone()).unwrap();
    assert!(may_accept(dir.path(), &id));
    revoke(dir.path(), id.clone()).unwrap();
    assert!(!may_accept(dir.path(), &id));
    retrust(dir.path(), id.clone()).unwrap();
    assert!(may_accept(dir.path(), &id));
}

#[test]
fn revoke_leaves_cloud_sync_key_bytes_identical() {
    let dir = tempdir().unwrap();
    let key = CloudSyncKey::create(dir.path()).unwrap();
    let before = fs::read(CloudSyncKey::key_path(dir.path())).unwrap();
    assert_eq!(before.as_slice(), key.expose().as_slice());
    let id = DeviceId::generate();
    register(dir.path(), id.clone()).unwrap();
    revoke(dir.path(), id).unwrap();
    let after = fs::read(CloudSyncKey::key_path(dir.path())).unwrap();
    assert_eq!(before, after);
}

#[test]
fn trust_json_uses_folder_hashes_and_omits_secrets() {
    let dir = tempdir().unwrap();
    let id = DeviceId::parse("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee").unwrap();
    register(dir.path(), id.clone()).unwrap();
    let list = list_trust(dir.path()).unwrap();
    let json = serde_json::to_string(&list).unwrap();
    let folder = &list.devices[0].folder;
    assert_eq!(folder, &id.remote_folder_name());
    assert_eq!(folder.len(), 32);
    assert!(folder.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')));
    assert!(!json.contains("hostname"));
    assert!(!json.contains("localhost"));
    assert!(!json.contains("cloud-sync.key"));
    assert!(!json.contains("ya29"));
    assert!(!fs::read_to_string(dir.path().join("sync/trust.json"))
        .unwrap()
        .contains("cloud-sync.key"));
}

#[test]
fn untrusted_download_does_not_apply() {
    let src = tempdir().unwrap();
    let dest = tempdir().unwrap();
    let key = CloudSyncKey::create(src.path()).unwrap();
    CloudSyncKey::import(dest.path(), key.expose()).unwrap();
    fs::create_dir_all(src.path().join("journals")).unwrap();
    fs::write(src.path().join("journals/claude.md"), b"from-src").unwrap();
    fs::create_dir_all(dest.path().join("journals")).unwrap();
    fs::write(dest.path().join("journals/claude.md"), b"keep-dest").unwrap();
    let config = SyncConfig::v1_defaults();
    let mut drive = FakeDrive::new();
    upload_home(src.path(), &key, &mut drive, &config, 3).unwrap();
    let err = download_home(dest.path(), &key, &drive, &config).unwrap_err();
    assert!(matches!(err, SyncError::Invalid(message) if message.contains("not trusted")));
    assert_eq!(
        fs::read(dest.path().join("journals/claude.md")).unwrap(),
        b"keep-dest"
    );
}

#[test]
fn ensure_local_device_persists_and_auto_trusts_self() {
    let dir = tempdir().unwrap();
    let first = ensure_local_device(dir.path()).unwrap();
    let second = ensure_local_device(dir.path()).unwrap();
    assert_eq!(first.id, second.id);
    assert!(may_accept(dir.path(), &first.id));
    assert!(dir.path().join("sync/device.json").is_file());
}
