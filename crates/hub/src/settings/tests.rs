use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use tempfile::tempdir;

#[path = "tests/profiles.rs"]
mod profiles;

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

#[test]
fn effective_settings_are_inherited_without_a_workspace_override() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store.set_backup_retention(7).unwrap();
    store.save().unwrap();

    let effective = store.effective(Some("/home/user/project"));
    assert_eq!(effective.backup_retention, 7);
    assert_eq!(effective.backup_retention_status, FieldStatus::Inherited);
    assert_eq!(effective.workspace.as_deref(), Some("/home/user/project"));

    let global = store.effective(None);
    assert_eq!(global.backup_retention, 7);
    assert_eq!(global.workspace, None);
}

#[test]
fn workspace_override_wins_over_global_default_and_round_trips() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .set_workspace_backup_retention("/home/user/project", 9)
        .unwrap();
    store.save().unwrap();

    let effective = store.effective(Some("/home/user/project"));
    assert_eq!(effective.backup_retention, 9);
    assert_eq!(effective.backup_retention_status, FieldStatus::Override);
    // The global default is untouched by a workspace-only override.
    assert_eq!(store.snapshot().backup_retention, DEFAULT_BACKUP_RETENTION);

    let reloaded = SettingsStore::open(dir.path());
    let reloaded_effective = reloaded.effective(Some("/home/user/project"));
    assert_eq!(reloaded_effective.backup_retention, 9);
    assert_eq!(
        reloaded_effective.backup_retention_status,
        FieldStatus::Override
    );
}

#[test]
fn distinct_workspace_paths_keep_separate_overrides() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .set_workspace_backup_retention("/repo/checkout-a", 4)
        .unwrap();
    store
        .set_workspace_backup_retention("/repo/checkout-b", 8)
        .unwrap();
    store.save().unwrap();

    assert_eq!(
        store.effective(Some("/repo/checkout-a")).backup_retention,
        4
    );
    assert_eq!(
        store.effective(Some("/repo/checkout-b")).backup_retention,
        8
    );
    // A workspace path never seen keeps the global default.
    assert_eq!(
        store.effective(Some("/repo/checkout-c")).backup_retention,
        DEFAULT_BACKUP_RETENTION
    );
}

#[test]
fn reset_workspace_field_falls_back_to_inherited_and_removes_empty_entry() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .set_workspace_backup_retention("/home/user/project", 12)
        .unwrap();
    store.save().unwrap();
    assert!(store.workspace_override("/home/user/project").is_some());

    store
        .reset_workspace_field("/home/user/project", SettingsField::BackupRetention)
        .unwrap();
    store.save().unwrap();

    assert!(store.workspace_override("/home/user/project").is_none());
    let effective = store.effective(Some("/home/user/project"));
    assert_eq!(effective.backup_retention, DEFAULT_BACKUP_RETENTION);
    assert_eq!(effective.backup_retention_status, FieldStatus::Inherited);
}

#[test]
fn workspace_override_out_of_bounds_is_rejected() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    assert!(store
        .set_workspace_backup_retention("/home/user/project", 99)
        .is_err());
    assert!(store.workspace_override("/home/user/project").is_none());
}

#[test]
fn workspace_overrides_survive_hand_authored_comments() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("settings.toml"),
        "schema_version = 1\n\n[storage]\nbackup_retention = 3\n\n\
         [[workspace]]\n# per-checkout note\npath = \"/home/user/project\"\nbackup_retention = 5\n",
    )
    .unwrap();

    let mut store = SettingsStore::open(dir.path());
    assert_eq!(store.load().status, LoadStatus::Loaded);
    assert_eq!(
        store.effective(Some("/home/user/project")).backup_retention,
        5
    );

    // Saving rebuilds the workspace array; top-level comments outside it
    // still survive.
    store.set_backup_retention(4).unwrap();
    store.save().unwrap();
    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(raw.contains("path = \"/home/user/project\""));
    assert!(raw.contains("backup_retention = 5"), "{raw}");
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

#[test]
fn orchestration_policy_defaults_are_safe() {
    let dir = tempdir().unwrap();
    let store = SettingsStore::open(dir.path());
    let effective = store.effective(None).orchestration;
    assert!(effective.confirm_new_enrollment);
    assert!(effective.confirm_broadcast);
    assert!(effective.auto_enrollment_allowed);
    assert_eq!(effective.sandbox_strictness, SandboxStrictness::Standard);
    assert_eq!(effective.retention_days, None);
    assert!(effective.export_enabled);
    assert_eq!(
        effective.confirm_new_enrollment_status,
        FieldStatus::Inherited
    );
}

#[test]
fn global_orchestration_fields_round_trip_and_validate() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store.set_confirm_new_enrollment(false).unwrap();
    store.set_confirm_broadcast(false).unwrap();
    store.set_auto_enrollment_allowed(false).unwrap();
    store
        .set_sandbox_strictness(SandboxStrictness::Strict)
        .unwrap();
    store.set_retention_days(Some(30)).unwrap();
    store.set_export_enabled(false).unwrap();
    store.save().unwrap();

    assert!(store.set_retention_days(Some(0)).is_err());

    let reloaded = SettingsStore::open(dir.path());
    let effective = reloaded.effective(None).orchestration;
    assert!(!effective.confirm_new_enrollment);
    assert!(!effective.confirm_broadcast);
    assert!(!effective.auto_enrollment_allowed);
    assert_eq!(effective.sandbox_strictness, SandboxStrictness::Strict);
    assert_eq!(effective.retention_days, Some(30));
    assert!(!effective.export_enabled);

    let raw = fs::read_to_string(reloaded.path()).unwrap();
    assert!(raw.contains("sandbox_strictness = \"strict\""), "{raw}");
}

#[test]
fn workspace_orchestration_override_wins_and_resets_to_inherited() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .set_workspace_confirm_broadcast("/home/user/project", false)
        .unwrap();
    store
        .set_workspace_sandbox_strictness("/home/user/project", SandboxStrictness::Permissive)
        .unwrap();
    store
        .set_workspace_retention_days("/home/user/project", 14)
        .unwrap();
    store.save().unwrap();

    let effective = store.effective(Some("/home/user/project")).orchestration;
    assert!(!effective.confirm_broadcast);
    assert_eq!(effective.confirm_broadcast_status, FieldStatus::Override);
    assert_eq!(effective.sandbox_strictness, SandboxStrictness::Permissive);
    assert_eq!(effective.retention_days, Some(14));
    // Untouched fields still inherit the global default.
    assert!(effective.confirm_new_enrollment);
    assert_eq!(
        effective.confirm_new_enrollment_status,
        FieldStatus::Inherited
    );

    store
        .reset_workspace_field("/home/user/project", SettingsField::ConfirmBroadcast)
        .unwrap();
    store.save().unwrap();
    let after_reset = store.effective(Some("/home/user/project")).orchestration;
    assert!(after_reset.confirm_broadcast);
    assert_eq!(after_reset.confirm_broadcast_status, FieldStatus::Inherited);
    // Other overrides on the same workspace remain.
    assert_eq!(
        after_reset.sandbox_strictness,
        SandboxStrictness::Permissive
    );
}

#[test]
fn orchestration_workspace_override_survives_hand_authored_comments() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("settings.toml"),
        "schema_version = 1\n\n[storage]\nbackup_retention = 3\n\n\
         [orchestration]\n# standing policy\nconfirm_new_enrollment = true\n\
         confirm_broadcast = true\nauto_enrollment_allowed = true\n\
         sandbox_strictness = \"standard\"\nexport_enabled = true\n\n\
         [[workspace]]\npath = \"/home/user/project\"\n\
         orchestration = { confirm_broadcast = false, retention_days = 7 }\n",
    )
    .unwrap();

    let store = SettingsStore::open(dir.path());
    assert_eq!(store.load().status, LoadStatus::Loaded);
    let effective = store.effective(Some("/home/user/project")).orchestration;
    assert!(!effective.confirm_broadcast);
    assert_eq!(effective.retention_days, Some(7));
    assert!(effective.auto_enrollment_allowed);
}
