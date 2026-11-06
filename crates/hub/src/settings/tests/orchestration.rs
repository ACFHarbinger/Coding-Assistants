use super::super::*;
use std::fs;
use tempfile::tempdir;

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
    assert!(effective.memory_recall_enabled);
    assert_eq!(effective.memory_recall_limit, 5);
    // Metered usage probes are on by default — a bare install still checks
    // every provider's usage; a distributor turns this off before handing
    // the app to testers.
    assert!(effective.allow_metered_quota_probes);
    // The background usage refresh is OFF by default: no timer spends tokens
    // without the user opting in. The cadence still has a sane default.
    assert!(!effective.quota_auto_refresh_enabled);
    assert_eq!(effective.quota_auto_refresh_interval_secs, 300);
    assert_eq!(
        effective.confirm_new_enrollment_status,
        FieldStatus::Inherited
    );
}

#[test]
fn allow_metered_quota_probes_round_trips_off() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store.set_allow_metered_quota_probes(false).unwrap();
    store.save().unwrap();

    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(raw.contains("allow_metered_quota_probes = false"), "{raw}");

    let reloaded = SettingsStore::open(dir.path());
    assert!(
        !reloaded
            .effective(None)
            .orchestration
            .allow_metered_quota_probes
    );
}

#[test]
fn quota_auto_refresh_round_trips_and_range_checks_the_cadence() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store.set_quota_auto_refresh_enabled(true).unwrap();
    store.set_quota_auto_refresh_interval_secs(120).unwrap();
    store.save().unwrap();

    // Out of range on both ends is rejected, leaving the stored value intact.
    assert!(store.set_quota_auto_refresh_interval_secs(5).is_err());
    assert!(store.set_quota_auto_refresh_interval_secs(7200).is_err());

    let reloaded = SettingsStore::open(dir.path());
    let effective = reloaded.effective(None).orchestration;
    assert!(effective.quota_auto_refresh_enabled);
    assert_eq!(effective.quota_auto_refresh_interval_secs, 120);
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
    store.set_memory_recall_enabled(false).unwrap();
    store.set_memory_recall_limit(8).unwrap();
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
    assert!(!effective.memory_recall_enabled);
    assert_eq!(effective.memory_recall_limit, 8);
    assert!(store.set_memory_recall_limit(0).is_err());

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
    store
        .set_workspace_memory_recall_enabled("/home/user/project", false)
        .unwrap();
    store
        .set_workspace_memory_recall_limit("/home/user/project", 3)
        .unwrap();
    store.save().unwrap();

    let effective = store.effective(Some("/home/user/project")).orchestration;
    assert!(!effective.confirm_broadcast);
    assert_eq!(effective.confirm_broadcast_status, FieldStatus::Override);
    assert_eq!(effective.sandbox_strictness, SandboxStrictness::Permissive);
    assert_eq!(effective.retention_days, Some(14));
    assert!(!effective.memory_recall_enabled);
    assert_eq!(effective.memory_recall_limit, 3);
    // Untouched fields still inherit the global default.
    assert!(effective.confirm_new_enrollment);
    assert_eq!(
        effective.confirm_new_enrollment_status,
        FieldStatus::Inherited
    );

    store
        .reset_workspace_field("/home/user/project", SettingsField::ConfirmBroadcast)
        .unwrap();
    store
        .reset_workspace_field("/home/user/project", SettingsField::MemoryRecallEnabled)
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
