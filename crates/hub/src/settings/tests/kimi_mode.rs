use super::super::*;
use tempfile::tempdir;

#[test]
fn kimi_mode_is_off_by_default_for_every_harness() {
    let dir = tempdir().unwrap();
    let store = SettingsStore::open(dir.path());
    let kimi = store.effective_harness(Some("/repo/ws"), "kimi").unwrap();
    assert_eq!(kimi.selected_kimi_mode, None);
    assert_eq!(kimi.selected_kimi_mode_status, FieldStatus::Inherited);
    let grok = store.effective_harness(Some("/repo/ws"), "grok").unwrap();
    assert_eq!(grok.selected_kimi_mode, None);
}

#[test]
fn workspace_kimi_mode_round_trips_and_applies_only_to_kimi() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .set_workspace_kimi_mode("/repo/ws", Some(crate::KimiTaskMode::Swarm))
        .unwrap();
    store.save().unwrap();

    let reloaded = SettingsStore::open(dir.path());
    let kimi = reloaded
        .effective_harness(Some("/repo/ws"), "kimi")
        .unwrap();
    assert_eq!(kimi.selected_kimi_mode, Some(crate::KimiTaskMode::Swarm));
    assert_eq!(kimi.selected_kimi_mode_status, FieldStatus::Override);
    // The override is Kimi-only: other harnesses in the same workspace
    // never see it.
    let claude = reloaded
        .effective_harness(Some("/repo/ws"), "claude")
        .unwrap();
    assert_eq!(claude.selected_kimi_mode, None);
    // A workspace without the override stays off.
    let other = reloaded
        .effective_harness(Some("/repo/other"), "kimi")
        .unwrap();
    assert_eq!(other.selected_kimi_mode, None);

    // The on-disk form is a plain string beside the other workspace fields.
    let raw = std::fs::read_to_string(reloaded.path()).unwrap();
    assert!(raw.contains("kimi_mode = \"swarm\""), "{raw}");
}

#[test]
fn clearing_workspace_kimi_mode_removes_the_empty_override_entry() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .set_workspace_kimi_mode("/repo/ws", Some(crate::KimiTaskMode::Goal))
        .unwrap();
    store.save().unwrap();
    store.set_workspace_kimi_mode("/repo/ws", None).unwrap();
    store.save().unwrap();

    let reloaded = SettingsStore::open(dir.path());
    let kimi = reloaded
        .effective_harness(Some("/repo/ws"), "kimi")
        .unwrap();
    assert_eq!(kimi.selected_kimi_mode, None);
    let raw = std::fs::read_to_string(reloaded.path()).unwrap();
    assert!(!raw.contains("kimi_mode"), "{raw}");
    assert!(!raw.contains("[[workspace]]"), "{raw}");
}

#[test]
fn invalid_kimi_mode_string_is_rejected_on_load() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.toml"),
        "schema_version = 1\n\n[[workspace]]\npath = \"/repo/ws\"\nkimi_mode = \"flock\"\n",
    )
    .unwrap();
    let store = SettingsStore::open(dir.path());
    assert!(matches!(store.load().status, LoadStatus::Invalid { .. }));
}
