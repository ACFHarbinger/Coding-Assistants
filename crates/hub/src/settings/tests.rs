use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn missing_file_loads_defaults_and_does_not_create() {
    let dir = tempdir().unwrap();
    let store = SettingsStore::open(dir.path());
    assert_eq!(store.load().status, LoadStatus::Missing);
    assert_eq!(store.snapshot().backup_retention, DEFAULT_BACKUP_RETENTION);
    assert_eq!(store.snapshot().schema_version, CURRENT_SETTINGS_SCHEMA);
    assert!(!store.path().exists());
}

#[test]
fn save_creates_file_and_round_trips() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store.set_backup_retention(5).unwrap();
    store.save().unwrap();

    let reloaded = SettingsStore::open(dir.path());
    assert_eq!(reloaded.load().status, LoadStatus::Loaded);
    assert_eq!(reloaded.snapshot().backup_retention, 5);
}

#[test]
fn save_preserves_hand_authored_comments_and_unknown_tables() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("settings.toml"),
        r#"# keep-this-header
schema_version = 1

[storage]
# keep-retention-note
backup_retention = 3

[owner]
# keep-owner
name = "harbinger"
"#,
    )
    .unwrap();

    let mut store = SettingsStore::open(dir.path());
    assert_eq!(store.load().status, LoadStatus::Loaded);
    store.set_backup_retention(4).unwrap();
    store.save().unwrap();

    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(raw.contains("# keep-this-header"), "{raw}");
    assert!(raw.contains("# keep-retention-note"), "{raw}");
    assert!(raw.contains("# keep-owner"), "{raw}");
    assert!(raw.contains("name = \"harbinger\""), "{raw}");
    assert!(raw.contains("backup_retention = 4"), "{raw}");
}

#[test]
fn malformed_file_loads_defaults_and_leaves_original() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    fs::write(&path, "this is not toml [[[").unwrap();

    let mut store = SettingsStore::open(dir.path());
    assert!(matches!(store.load().status, LoadStatus::Invalid { .. }));
    assert_eq!(store.snapshot().backup_retention, DEFAULT_BACKUP_RETENTION);
    assert_eq!(fs::read_to_string(&path).unwrap(), "this is not toml [[[");
    assert!(store.save().is_err());
}

#[test]
fn leftover_tmp_is_ignored_on_load() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("settings.toml.tmp"), "garbage").unwrap();
    let store = SettingsStore::open(dir.path());
    assert_eq!(store.load().status, LoadStatus::Missing);
}

#[test]
fn unknown_schema_is_invalid_and_not_overwritten() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    fs::write(
        &path,
        "schema_version = 99\n[storage]\nbackup_retention = 3\n",
    )
    .unwrap();
    let store = SettingsStore::open(dir.path());
    assert!(matches!(store.load().status, LoadStatus::Invalid { .. }));
    assert!(path.exists());
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("schema_version = 99"));
}

#[test]
fn retention_out_of_bounds_is_invalid() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("settings.toml"),
        "schema_version = 1\n[storage]\nbackup_retention = 99\n",
    )
    .unwrap();
    let store = SettingsStore::open(dir.path());
    assert!(matches!(store.load().status, LoadStatus::Invalid { .. }));
}

#[test]
fn save_writes_timestamped_backups_and_prunes() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store.set_backup_retention(2).unwrap();
    store.save().unwrap();
    for retention in [2_u32, 2, 2, 2] {
        std::thread::sleep(Duration::from_millis(5));
        store.set_backup_retention(retention).unwrap();
        store.save().unwrap();
    }
    let backups = store.list_backups().unwrap();
    assert_eq!(backups.len(), 2, "{backups:?}");
}

#[test]
fn restore_backup_replaces_current_file() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store.save().unwrap();
    store.set_backup_retention(6).unwrap();
    store.save().unwrap();
    let backups = store.list_backups().unwrap();
    assert!(!backups.is_empty());
    store.restore_backup(&backups[0]).unwrap();
    let reloaded = SettingsStore::open(dir.path());
    assert_eq!(reloaded.load().status, LoadStatus::Loaded);
    assert_eq!(
        reloaded.snapshot().backup_retention,
        DEFAULT_BACKUP_RETENTION
    );
}

#[test]
fn restore_rejects_path_outside_backup_dir() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store.save().unwrap();
    let outside = dir.path().join("evil.toml");
    fs::write(
        &outside,
        "schema_version = 1\n[storage]\nbackup_retention = 3\n",
    )
    .unwrap();
    assert!(store.restore_backup(&outside).is_err());
}

#[test]
fn quarantine_moves_malformed_file_then_writes_defaults() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    fs::write(&path, "nope").unwrap();
    let mut store = SettingsStore::open(dir.path());
    let quarantined = store.quarantine_invalid_and_save().unwrap();
    assert!(quarantined
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("invalid"));
    assert_eq!(fs::read_to_string(&quarantined).unwrap(), "nope");
    let reloaded = SettingsStore::open(dir.path());
    assert_eq!(reloaded.load().status, LoadStatus::Loaded);
    assert_eq!(
        reloaded.snapshot().backup_retention,
        DEFAULT_BACKUP_RETENTION
    );
}

#[cfg(unix)]
#[test]
fn unreadable_file_does_not_crash() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    fs::write(
        &path,
        "schema_version = 1\n[storage]\nbackup_retention = 3\n",
    )
    .unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&path, perms).unwrap();

    let store = SettingsStore::open(dir.path());
    assert!(matches!(
        store.load().status,
        LoadStatus::Unreadable { .. } | LoadStatus::Invalid { .. }
    ));

    let mut restore = fs::metadata(&path).unwrap().permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&path, restore).unwrap();
}
