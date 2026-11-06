//! S7 acceptance gaps (#133). Not a second copy of the S1–S5 store tests.
//!
//! Already on main, cited rather than repeated:
//! - no file, malformed load (the blank-startup guard: in-memory defaults,
//!   original bytes kept, `save` refuses to overwrite), leftover tmp,
//!   unknown schema left untouched, backup retention/restore, comments,
//!   string-distinct workspace overrides, unreadable load: `settings::tests`
//! - named profile selection without copying secret fields:
//!   `settings::tests::profiles`
//! - safe confirmation defaults (`confirm_new_enrollment`, `confirm_broadcast`):
//!   `orchestration_policy_defaults_are_safe`. Task delivery never spawning a
//!   harness is `inject_harness`, outside this store; not retested here.
//!
//! No pre-version dialect exists. `CURRENT_SETTINGS_SCHEMA` is 1. A missing
//! or unknown `schema_version` is invalid and is not rewritten.
//!
//! No separate settings diagnostics command exists. The redacted export seam
//! is `EffectiveSettings` / `ProfileSnapshot` (what IPC returns) plus
//! `HubStore::list_settings_audit_events`.
//!
//! No pure "one settings window" function lives in this crate. Open, focus,
//! and close stay an owner live check: `src/lib/settingsWindow.ts` (label
//! `settings`) and the main-window close path in `src-tauri/src/lib.rs`.
//!
//! Typed purge/reset confirmation is `tui::app::danger_ops::DangerModalState`,
//! which hub cannot call. Wrong and empty input leaving transcripts,
//! memories, and settings in place is
//! `crates/tui/tests/settings_s7_danger_cancel.rs`.

use super::super::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const PLANTED_SECRET: &str = "sk-live-PLANTED-9f3c2a";

fn fresh_home() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir(&home).unwrap();
    (dir, home)
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
fn euid_is_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                let rest = line.strip_prefix("Uid:")?;
                rest.split_whitespace().next().map(|uid| uid == "0")
            })
        })
        .unwrap_or(false)
}

#[cfg(unix)]
struct RestoreMode {
    path: PathBuf,
    mode: u32,
}

#[cfg(unix)]
impl Drop for RestoreMode {
    fn drop(&mut self) {
        let Ok(meta) = fs::metadata(&self.path) else {
            return;
        };
        let mut perms = meta.permissions();
        perms.set_mode(self.mode);
        let _ = fs::set_permissions(&self.path, perms);
    }
}

#[cfg(unix)]
fn deny_writes(path: &Path, mode: u32) -> RestoreMode {
    let original = fs::metadata(path).unwrap().permissions().mode();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms).unwrap();
    RestoreMode {
        path: path.to_path_buf(),
        mode: original,
    }
}

#[test]
fn symlink_distinct_workspace_paths_keep_separate_overrides() {
    let dir = tempdir().unwrap();
    let real = dir.path().join("real-repo");
    fs::create_dir(&real).unwrap();
    let link = dir.path().join("link-repo");
    #[cfg(unix)]
    {
        if let Err(err) = std::os::unix::fs::symlink(&real, &link) {
            eprintln!("skipping symlink-distinct override test: cannot create symlink ({err})");
            return;
        }
    }
    #[cfg(not(unix))]
    {
        eprintln!("skipping symlink-distinct override test: symlinks are not available on this OS");
        return;
    }

    let real_s = real.to_string_lossy().into_owned();
    let link_s = link.to_string_lossy().into_owned();
    assert_ne!(real_s, link_s);
    assert_eq!(
        real.canonicalize().unwrap(),
        link.canonicalize().unwrap(),
        "the fixture must be two path strings for one directory"
    );

    let (_tmp, home) = fresh_home();
    let mut store = SettingsStore::open(&home);
    store.set_workspace_backup_retention(&real_s, 4).unwrap();
    store.set_workspace_backup_retention(&link_s, 8).unwrap();
    store.save().unwrap();

    let via_real = store.effective(Some(&real_s));
    let via_link = store.effective(Some(&link_s));
    assert_eq!(via_real.backup_retention, 4);
    assert_eq!(via_real.backup_retention_status, FieldStatus::Override);
    assert_eq!(via_real.workspace.as_deref(), Some(real_s.as_str()));
    assert_eq!(via_link.backup_retention, 8);
    assert_eq!(via_link.backup_retention_status, FieldStatus::Override);
    assert_eq!(via_link.workspace.as_deref(), Some(link_s.as_str()));

    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(raw.contains(&real_s), "{raw}");
    assert!(raw.contains(&link_s), "{raw}");

    let reloaded = SettingsStore::open(&home);
    assert_eq!(reloaded.effective(Some(&real_s)).backup_retention, 4);
    assert_eq!(reloaded.effective(Some(&link_s)).backup_retention, 8);
}

#[test]
fn first_release_document_loads_and_keeps_comments_on_save() {
    let (_tmp, home) = fresh_home();
    let path = home.join("settings.toml");
    let original = "\
# first-release header
schema_version = 1

[storage]
# keep-retention-note
backup_retention = 4
";
    fs::write(&path, original).unwrap();

    let mut store = SettingsStore::open(&home);
    assert_eq!(store.load().status, LoadStatus::Loaded);
    assert_eq!(store.snapshot().schema_version, CURRENT_SETTINGS_SCHEMA);
    assert_eq!(store.snapshot().backup_retention, 4);
    assert!(store.snapshot().orchestration.confirm_new_enrollment);
    assert!(store.snapshot().orchestration.confirm_broadcast);
    assert_eq!(
        store.snapshot().embedding_provider,
        EmbeddingProvider::Local
    );

    store.save().unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("# first-release header"), "{raw}");
    assert!(raw.contains("# keep-retention-note"), "{raw}");
    assert!(raw.contains("backup_retention = 4"), "{raw}");
    assert_eq!(SettingsStore::open(&home).snapshot().backup_retention, 4);
}

#[test]
fn unversioned_document_is_not_migrated_or_overwritten() {
    let (_tmp, home) = fresh_home();
    let path = home.join("settings.toml");
    let original = "\
# pre-version, not a dialect we upgrade
[storage]
backup_retention = 4
";
    fs::write(&path, original).unwrap();

    let mut store = SettingsStore::open(&home);
    assert!(matches!(store.load().status, LoadStatus::Invalid { .. }));
    assert_eq!(store.snapshot().backup_retention, DEFAULT_BACKUP_RETENTION);
    assert!(store.save().is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn save_permission_failure_keeps_the_last_good_file() {
    if euid_is_root() {
        eprintln!(
            "skipping save permission-failure test: euid 0 ignores mode bits, so a denial cannot be forced"
        );
        return;
    }

    let (_tmp, home) = fresh_home();
    let mut store = SettingsStore::open(&home);
    store.set_backup_retention(7).unwrap();
    store.save().unwrap();
    let path = store.path().to_path_buf();
    let original = fs::read_to_string(&path).unwrap();
    assert!(original.contains("backup_retention = 7"), "{original}");

    store.set_backup_retention(9).unwrap();

    let _dir_guard = deny_writes(&home, 0o555);
    let dir_mode = fs::metadata(&home).unwrap().permissions().mode() & 0o222;
    if dir_mode != 0 {
        eprintln!(
            "skipping directory permission-failure case: mode bits did not stick on {}",
            home.display()
        );
    } else {
        let err = store.save().unwrap_err();
        assert!(
            err.to_string().contains("io:") || err.to_string().contains("Permission denied"),
            "{err}"
        );
    }
    drop(_dir_guard);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);

    let mut store = SettingsStore::open(&home);
    assert_eq!(store.snapshot().backup_retention, 7);
    store.set_backup_retention(9).unwrap();
    let _file_guard = deny_writes(&path, 0o000);
    let file_mode = fs::metadata(&path).unwrap().permissions().mode() & 0o222;
    if file_mode != 0 {
        eprintln!(
            "skipping file permission-failure case: mode bits did not stick on {}",
            path.display()
        );
    } else {
        assert!(store.save().is_err());
    }
    drop(_file_guard);
    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(after, original, "{after}");
    assert!(
        !after.contains("backup_retention = 9"),
        "a failed save must not replace the last good file with the new value or with defaults"
    );
    assert_eq!(SettingsStore::open(&home).load().status, LoadStatus::Loaded);
    assert_eq!(SettingsStore::open(&home).snapshot().backup_retention, 7);
}

#[test]
fn redacted_snapshot_omits_a_planted_keychain_secret() {
    let (_tmp, home) = fresh_home();
    let path = home.join("settings.toml");
    let planted = format!(
        "schema_version = 1\n[storage]\nbackup_retention = 3\n\n\
         [[profile]]\nname = \"work\"\nprovider = \"grok\"\n\
         secret_source = \"keychain\"\nsecret_ref = \"{PLANTED_SECRET}\"\n"
    );
    fs::write(&path, &planted).unwrap();

    let mut store = SettingsStore::open(&home);
    match &store.load().status {
        LoadStatus::Invalid { reason } => {
            assert!(
                !reason.contains(PLANTED_SECRET),
                "load diagnostics must not echo the planted secret: {reason}"
            );
        }
        other => panic!("expected invalid load, got {other:?}"),
    }
    let exported = serde_json::to_string(&store.effective(None)).unwrap();
    assert!(!exported.contains(PLANTED_SECRET), "{exported}");
    assert!(!serde_json::to_string(&store.list_profiles())
        .unwrap()
        .contains(PLANTED_SECRET));
    assert!(store.save().is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), planted);

    let rejected = store.upsert_profile(ProviderProfile {
        name: "work".into(),
        provider: "grok".into(),
        model: None,
        base_url: None,
        secret: SecretReference::Keychain {
            id: PLANTED_SECRET.into(),
        },
    });
    assert!(
        rejected.is_err(),
        "upsert must refuse to store the raw secret"
    );

    fs::remove_file(&path).unwrap();
    let mut store = SettingsStore::open(&home);
    const OPAQUE_ID: &str = "ca.profile.work";
    store
        .upsert_profile(ProviderProfile {
            name: "work".into(),
            provider: "grok".into(),
            model: None,
            base_url: None,
            secret: SecretReference::Keychain {
                id: OPAQUE_ID.into(),
            },
        })
        .unwrap();
    store
        .upsert_profile(ProviderProfile {
            name: "env".into(),
            provider: "grok".into(),
            model: None,
            base_url: None,
            secret: SecretReference::EnvVar {
                name: "XAI_API_KEY".into(),
            },
        })
        .unwrap();
    store
        .set_workspace_default_profile("/abs/repo", "grok", "work")
        .unwrap();
    store.save().unwrap();

    let effective = store.effective(Some("/abs/repo"));
    let exported = serde_json::to_string(&effective).unwrap();
    assert!(!exported.contains(OPAQUE_ID), "{exported}");
    assert!(!exported.contains(PLANTED_SECRET), "{exported}");
    assert!(exported.contains("Stored in System Keychain"), "{exported}");
    assert!(exported.contains("Env Var $XAI_API_KEY"), "{exported}");
    assert!(!serde_json::to_string(&store.list_profiles())
        .unwrap()
        .contains(OPAQUE_ID));

    let selected = store.effective_harness(Some("/abs/repo"), "grok").unwrap();
    assert_eq!(selected.default_profile.as_deref(), Some("work"));
    assert_eq!(selected.default_profile_status, FieldStatus::Override);
    assert!(!serde_json::to_string(&selected)
        .unwrap()
        .contains(OPAQUE_ID));

    let hub = crate::HubStore::open(&home).unwrap();
    hub.record_settings_audit_event("profile.work", "global", "upsert")
        .unwrap();
    let audit = format!("{:?}", hub.list_settings_audit_events().unwrap());
    assert!(!audit.contains(PLANTED_SECRET), "{audit}");
    assert!(!audit.contains(OPAQUE_ID), "{audit}");
}
