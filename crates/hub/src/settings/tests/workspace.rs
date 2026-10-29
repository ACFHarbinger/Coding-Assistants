use super::super::*;
use std::fs;
use tempfile::tempdir;

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
