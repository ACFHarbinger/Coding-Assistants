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

fn sample_profile(name: &str, provider: &str) -> ProviderProfile {
    ProviderProfile {
        name: name.into(),
        provider: provider.into(),
        model: Some("grok-4".into()),
        base_url: Some("https://api.x.ai".into()),
        secret: SecretReference::EnvVar {
            name: "XAI_API_KEY".into(),
        },
    }
}

#[test]
fn profiles_round_trip_without_persisting_secrets() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(sample_profile("work", "grok"))
        .unwrap();
    store
        .upsert_profile(ProviderProfile {
            name: "login".into(),
            provider: "claude".into(),
            model: None,
            base_url: None,
            secret: SecretReference::ProviderLogin,
        })
        .unwrap();
    store.save().unwrap();

    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(raw.contains("name = \"work\""));
    assert!(raw.contains("secret_source = \"env_var\""));
    assert!(raw.contains("secret_ref = \"XAI_API_KEY\""));
    assert!(!raw.to_ascii_lowercase().contains("sk-"));
    assert!(!raw.contains("Bearer"));

    let reloaded = SettingsStore::open(dir.path());
    let work = reloaded
        .list_profiles()
        .into_iter()
        .find(|p| p.name == "work")
        .unwrap();
    assert_eq!(work.secret_source, SecretSourceKind::EnvVar);
    assert_eq!(work.secret_badge, "Env Var $XAI_API_KEY");
    assert_eq!(
        reloaded.profile("login").unwrap().secret,
        SecretReference::ProviderLogin
    );
}

#[test]
fn profile_rename_updates_workspace_default_selection() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(sample_profile("work", "grok"))
        .unwrap();
    store
        .set_workspace_default_profile("/abs/repo", "grok", "work")
        .unwrap();
    store.rename_profile("work", "office").unwrap();
    store.save().unwrap();

    let selected = store
        .workspace_override("/abs/repo")
        .unwrap()
        .default_profiles
        .get("grok")
        .cloned();
    assert_eq!(selected.as_deref(), Some("office"));
    assert!(store.profile("work").is_none());
}

#[test]
fn profile_remove_clears_workspace_selection_and_keeps_no_secret_delete() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(ProviderProfile {
            name: "stored".into(),
            provider: "chat".into(),
            model: None,
            base_url: None,
            secret: SecretReference::Keychain {
                id: "ca.profile.stored".into(),
            },
        })
        .unwrap();
    store
        .set_workspace_default_profile("/abs/repo", "chat", "stored")
        .unwrap();
    let removed = store.remove_profile("stored").unwrap();
    assert_eq!(removed.secret.badge(), "Stored in System Keychain");
    assert!(store
        .workspace_override("/abs/repo")
        .map(|over| over.default_profiles.is_empty())
        .unwrap_or(true));
}

#[test]
fn workspace_cannot_copy_or_select_a_foreign_profile() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(sample_profile("work", "grok"))
        .unwrap();
    assert!(store
        .set_workspace_default_profile("/abs/repo", "claude", "work")
        .is_err());
    assert!(store
        .upsert_profile(ProviderProfile {
            name: "bad".into(),
            provider: "grok".into(),
            model: Some("sk-secret".into()),
            base_url: None,
            secret: SecretReference::ProviderLogin,
        })
        .is_err());
}

#[test]
fn harness_settings_validate_executable_and_absolute_workdir() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    assert!(store
        .set_harness_settings(HarnessSettings {
            harness: "grok".into(),
            executable: "grok --evil".into(),
            workdir: None,
            capture_polling: true,
            inject_permission: true,
        })
        .is_err());
    assert!(store
        .set_harness_settings(HarnessSettings {
            harness: "grok".into(),
            executable: "grok".into(),
            workdir: Some("relative".into()),
            capture_polling: true,
            inject_permission: false,
        })
        .is_err());
    store
        .set_harness_settings(HarnessSettings {
            harness: "grok".into(),
            executable: "/usr/bin/grok".into(),
            workdir: Some("/abs/ws".into()),
            capture_polling: false,
            inject_permission: false,
        })
        .unwrap();
    store.save().unwrap();

    let reloaded = SettingsStore::open(dir.path());
    let harness = reloaded.harness_settings("grok").unwrap();
    assert_eq!(harness.executable, "/usr/bin/grok");
    assert_eq!(harness.workdir.as_deref(), Some("/abs/ws"));
    assert!(!harness.capture_polling);
    assert!(!harness.inject_permission);
}

#[test]
fn workspace_default_profile_is_a_name_reference_not_a_copy() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(sample_profile("work", "grok"))
        .unwrap();
    store
        .set_workspace_default_profile("/abs/repo", "grok", "work")
        .unwrap();
    store.save().unwrap();

    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(
        raw.contains("work = \"work\"") || raw.contains("grok = \"work\""),
        "{raw}"
    );
    assert_eq!(raw.matches("https://api.x.ai").count(), 1);

    store
        .upsert_profile(ProviderProfile {
            name: "work".into(),
            provider: "grok".into(),
            model: Some("grok-4.1".into()),
            base_url: Some("https://api.x.ai".into()),
            secret: SecretReference::EnvVar {
                name: "XAI_API_KEY".into(),
            },
        })
        .unwrap();
    let effective = store.effective_harness(Some("/abs/repo"), "grok").unwrap();
    assert_eq!(effective.default_profile.as_deref(), Some("work"));
    assert_eq!(effective.default_profile_status, FieldStatus::Override);
    assert_eq!(
        effective.default_profile_badge.as_deref(),
        Some("Env Var $XAI_API_KEY")
    );
    assert_eq!(
        store.profile("work").unwrap().model.as_deref(),
        Some("grok-4.1")
    );
}
