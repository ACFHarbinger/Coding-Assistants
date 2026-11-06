use super::*;
use crate::sync::key::CloudSyncKey;
use crate::sync::ops::{self, ResumeState};
use crate::sync::trust;
use crate::sync::types::{DeviceId, SyncResult};
use std::fs;
use tempfile::tempdir;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[test]
fn history_strips_leaky_warnings() {
    let dir = tempdir().unwrap();
    let result = SyncResult {
        uploaded: 1,
        warnings: vec![
            "live hub.db was not replaced".into(),
            "refresh token ya29.leak".into(),
            "cloud-sync.key mentioned".into(),
        ],
        ..SyncResult::default()
    };
    append_history(dir.path(), "up", &result, true).unwrap();
    let rows = list_history(dir.path()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].warnings, vec!["live hub.db was not replaced"]);
    let raw = fs::read_to_string(dir.path().join("sync/history.jsonl")).unwrap();
    assert!(!raw.contains("ya29"));
    assert!(!raw.contains("cloud-sync.key"));
    assert!(!raw.contains("token"));
}

#[test]
fn diagnostics_export_has_no_secrets_or_raw_key() {
    let dir = tempdir().unwrap();
    let key = CloudSyncKey::create(dir.path()).unwrap();
    let peer = DeviceId::generate();
    trust::register(dir.path(), peer).unwrap();
    ops::begin_resume(dir.path(), "up").unwrap();
    ops::save_resume(
        dir.path(),
        &ResumeState {
            action: "up".into(),
            done: Vec::new(),
        },
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("sync")).unwrap();
    fs::write(
        dir.path().join("sync/last-verified.json"),
        r#"{"schema_version":"3","base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
    )
    .unwrap();
    append_history(
        dir.path(),
        "up",
        &SyncResult {
            warnings: vec!["Bearer ya29.should-not-appear".into()],
            ..SyncResult::default()
        },
        true,
    )
    .unwrap();
    let diag = export_diagnostics(dir.path()).unwrap();
    assert_eq!(
        diag.last_verified_base.as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(diag.schema, 1);
    assert_eq!(diag.resume_action.as_deref(), Some("up"));
    let json = serde_json::to_string(&diag).unwrap();
    assert!(!json.contains("cloud-sync.key"));
    assert!(!json.contains("ya29"));
    assert!(!json.contains("Bearer"));
    assert!(!contains(json.as_bytes(), key.expose().as_slice()));
    assert!(diag.trust.iter().all(|row| row.folder.len() == 32));
}
